//! `verify!`: a test DSL that separates the behavior under test from setup and
//! observation.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, quote};
use syn::{
    BinOp, Block, Expr, ExprMacro, Ident, LitStr, Pat, Stmt, Token, braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

/// Evaluates an action once, then checks the observations and predicates in
/// order. Panics with a diagnostic on the first failure.
///
/// ```ignore
/// verify! {
///     let Some(user) = find_user(id) => {
///         let Ok(profile) = find_profile(user.id);
///
///         user.name == "alice";
///         profile.active;
///     },
///     "id={id}"
/// }
/// ```
#[proc_macro]
pub fn verify(input: TokenStream) -> TokenStream {
    let verify = parse_macro_input!(input as Verify);
    verify.expand().into()
}

struct Verify {
    binding: Option<Pat>,
    action: Expr,
    stmts: Vec<Stmt>,
    context: Option<TokenStream2>,
}

impl Parse for Verify {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let binding = if input.peek(Token![let]) {
            input.parse::<Token![let]>()?;
            let pat = Pat::parse_multi_with_leading_vert(input)?;
            input.parse::<Token![=]>()?;
            Some(pat)
        } else {
            None
        };
        let action = input.parse()?;
        input.parse::<Token![=>]>()?;
        let content;
        braced!(content in input);
        let stmts = content.call(Block::parse_within)?;
        let context = if input.parse::<Option<Token![,]>>()?.is_some() && !input.is_empty() {
            Some(input.parse::<TokenStream2>()?)
        } else {
            None
        };
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after verification block"));
        }
        Ok(Self {
            binding,
            action,
            stmts,
            context,
        })
    }
}

/// What is shown in a failure report, besides the location.
struct Report<'a> {
    context: &'a Option<TokenStream2>,
    action: String,
}

impl Verify {
    fn expand(&self) -> TokenStream2 {
        let report = Report {
            context: &self.context,
            action: source(&self.action),
        };
        let action = &self.action;

        let head = match &self.binding {
            Some(pat) => let_else(&[], pat, action, &report, "Pattern"),
            None => quote!(#action;),
        };

        let mut uses_show = false;
        let body: Vec<TokenStream2> = self
            .stmts
            .iter()
            .map(|stmt| match expand_stmt(stmt, &report) {
                Ok((tokens, shows)) => {
                    uses_show |= shows;
                    tokens
                }
                Err(e) => e.to_compile_error(),
            })
            .collect();

        let show_helper = uses_show.then(show_helper);
        quote! {{
            #show_helper
            #head
            #(#body)*
        }}
    }
}

fn expand_stmt(stmt: &Stmt, report: &Report) -> syn::Result<(TokenStream2, bool)> {
    match stmt {
        Stmt::Local(local) => {
            let init = local.init.as_ref().ok_or_else(|| {
                syn::Error::new_spanned(local, "observation requires an initializer")
            })?;
            if let Some((else_token, _)) = &init.diverge {
                return Err(syn::Error::new_spanned(
                    else_token,
                    "a pattern mismatch already fails; remove `else`",
                ));
            }
            Ok((
                let_else(&local.attrs, &local.pat, &init.expr, report, "Pattern"),
                false,
            ))
        }
        Stmt::Expr(expr, _) => Ok(predicate(expr, report)),
        Stmt::Macro(m) => {
            let expr = Expr::Macro(ExprMacro {
                attrs: m.attrs.clone(),
                mac: m.mac.clone(),
            });
            Ok(predicate(&expr, report))
        }
        Stmt::Item(item) => Err(syn::Error::new_spanned(
            item,
            "items are not allowed in a verification block",
        )),
    }
}

fn let_else(
    attrs: &[syn::Attribute],
    pat: &Pat,
    init: &Expr,
    report: &Report,
    label: &str,
) -> TokenStream2 {
    let fail = fail(report, label, &source(pat), None);
    // Parentheses allow an init expression ending in `}`; the pattern may be irrefutable.
    quote! {
        #(#attrs)*
        #[allow(irrefutable_let_patterns, unused_parens)]
        let #pat = (#init) else { #fail };
    }
}

/// Returns the tokens and whether they rely on `show_helper`.
fn predicate(expr: &Expr, report: &Report) -> (TokenStream2, bool) {
    if let Expr::Binary(bin) = expr
        && matches!(
            bin.op,
            BinOp::Eq(_) | BinOp::Ne(_) | BinOp::Gt(_) | BinOp::Ge(_) | BinOp::Lt(_) | BinOp::Le(_)
        )
    {
        let (left, op, right) = (&bin.left, &bin.op, &bin.right);
        let (l, r) = (hygienic("thus_left"), hygienic("thus_right"));
        let (left_src, right_src) = (source(left), source(right));
        let width = left_src.chars().count().max(right_src.chars().count()) + 1;
        let label = |src: &str| format!("  {:<width$} ", format!("{src}:"));
        let (left_label, right_label) = (label(&left_src), label(&right_src));
        let values = quote! {
            __thus_report.push('\n');
            __thus_report.push_str(#left_label);
            __thus_report.push_str(&(&__ThusShow(#l)).__thus_show());
            __thus_report.push('\n');
            __thus_report.push_str(#right_label);
            __thus_report.push_str(&(&__ThusShow(#r)).__thus_show());
            __thus_report.push('\n');
        };
        let fail = fail(report, "Predicate", &source(expr), Some(values));
        let tokens = quote! {
            match (&(#left), &(#right)) {
                (#l, #r) => {
                    if !(*#l #op *#r) { #fail }
                }
            }
        };
        (tokens, true)
    } else {
        let fail = fail(report, "Predicate", &source(expr), None);
        (quote! { if !(#expr) { #fail } }, false)
    }
}

/// Panics with the report. The context is evaluated only here.
fn fail(report: &Report, label: &str, source: &str, values: Option<TokenStream2>) -> TokenStream2 {
    let context = report.context.as_ref().map(|args| {
        quote! {
            __thus_report.push_str("Context: ");
            __thus_report.push_str(&::std::format!(#args));
            __thus_report.push('\n');
        }
    });
    let action = LitStr::new(&format!("Action: {}\n", report.action), Span::call_site());
    let predicate = LitStr::new(&format!("{label}: {source}\n"), Span::call_site());
    quote! {
        let mut __thus_report = ::std::format!(
            "Verification failed at {}:{}\n",
            ::core::file!(),
            ::core::line!(),
        );
        #context
        __thus_report.push_str(#action);
        __thus_report.push_str(#predicate);
        #values
        ::core::panic!("{}", __thus_report)
    }
}

/// Formats values with `Debug` when available, without requiring it
/// (autoref specialization: the `Debug` impl is found one autoref earlier).
fn show_helper() -> TokenStream2 {
    quote! {
        struct __ThusShow<'a, T: ?Sized>(&'a T);
        trait __ThusViaDebug {
            fn __thus_show(&self) -> ::std::string::String;
        }
        impl<T: ::core::fmt::Debug + ?Sized> __ThusViaDebug for __ThusShow<'_, T> {
            fn __thus_show(&self) -> ::std::string::String {
                ::std::format!("{:?}", self.0)
            }
        }
        trait __ThusViaFallback {
            fn __thus_show(&self) -> ::std::string::String;
        }
        impl<T: ?Sized> __ThusViaFallback for &__ThusShow<'_, T> {
            fn __thus_show(&self) -> ::std::string::String {
                ::std::string::String::from("<not Debug>")
            }
        }
    }
}

fn hygienic(name: &str) -> Ident {
    Ident::new(name, Span::mixed_site())
}

fn source(node: &impl ToTokens) -> String {
    node.to_token_stream().to_string()
}
