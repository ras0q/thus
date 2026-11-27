# thus

`verify!` marks the behavior under test and states what must hold afterwards.
It works with plain Rust expressions and any test runner.

See [Discussion #1](https://github.com/ras0q/thus/discussions/1) for the initial design.

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

## Failure output

```text
Verification failed at tests/discount.rs:24
Context: price=1000, discount=10
Action: apply_discount(price, discount)
Predicate: actual == expected

  actual:   900
  expected: 800
```
