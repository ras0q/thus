# Contributing

## Setup

Use stable Rust.

```sh
cargo test
cargo clippy --all-targets
cargo fmt
```

## Layout

- `src/lib.rs`: the `verify!` proc macro (parse, then generate).
- `tests/verify.rs`: behavior tests, including failure messages.

## Principles

- Public API is `verify!` only.
- Keep Rust's evaluation semantics: no implicit clone, no re-evaluation for diagnostics.
- Keep dependencies minimal: `syn` (trimmed features), `quote`, `proc-macro2`.
  Discuss before adding one.
- Diagnostics must not require extra trait bounds such as `Debug`.

## Changes

- Test behavior through `verify!`. For failures, assert on the panic message.
- Put a reason in comments only when it is not obvious (why not, not how).
- Commit title says what changed; the body says why.
