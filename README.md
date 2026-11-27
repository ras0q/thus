# thus

`verify!` marks the behavior under test and states what must hold afterwards.
It works with plain Rust expressions and any test runner.

## Usage

```rust
use thus::verify;

#[test]
fn discount() {
    let (price, discount, expected) = (1000, 10, 900);

    verify! {
        let actual = apply_discount(price, discount) => {
            actual == expected;
            actual >= 0 && actual <= price;
        },
        "price={price}, discount={discount}"
    }
}
```

Shape: `[let pattern =] action => { observations and predicates } [, context]`.

- **Action**: evaluated once. `.await`, `?`, `return`, and `continue` follow the caller.
- **Binding**: `let` pattern on the action's result. Refutable patterns are allowed; a mismatch fails.
- **Observation**: `let pattern = expr;` in the block. Later lines can use it. A mismatch fails.
- **Predicate**: `expr;` of type `bool` in the block. `false` fails.
- **Context**: optional `format!` arguments, evaluated only on failure.

Statements run in order. The first failure panics and skips the rest.
`verify!` evaluates to `()`.

## Failure output

```text
Verification failed at tests/discount.rs:24
Context: price=1000, discount=10
Action: apply_discount(price, discount)
Predicate: actual == expected

  actual:   900
  expected: 800
```

Comparison operands are shown when they can be formatted.

## Limits

- Context sees names from outside `verify!`, not the binding or observations.
- Bindings can't have type annotations. Observations can.