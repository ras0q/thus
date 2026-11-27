use std::panic::{AssertUnwindSafe, catch_unwind};
use thus::verify;

fn panic_message(f: impl FnOnce()) -> String {
    let payload = catch_unwind(AssertUnwindSafe(f)).expect_err("expected a panic");
    payload
        .downcast_ref::<String>()
        .cloned()
        .expect("panic payload should be a String")
}

fn apply_discount(price: i32, discount: i32) -> i32 {
    price - price * discount / 100
}

#[test]
fn passes_with_binding_observation_and_predicates() {
    let find_user = |_id: u32| Some(("alice", 7));
    let find_profile = |id: u32| -> Result<bool, ()> { Ok(id == 7) };
    verify! {
        let Some(user) = find_user(1) => {
            let Ok(active) = find_profile(user.1);

            user.0 == "alice";
            active;
            user.1 > 0 && user.1 <= 100;
            matches!(user.0, "alice");
        }
    }
}

#[test]
fn passes_without_binding_and_with_empty_block() {
    let mut items = vec![];
    verify! {
        items.push(1) => {
            items.len() == 1;
        }
    }
    verify! {
        let Ok(_) = Ok::<_, ()>(()) => {}
    }
}

#[test]
fn action_is_evaluated_once() {
    let mut calls = 0;
    verify! {
        let n = { calls += 1; calls } => {
            n == 1;
        }
    }
    assert_eq!(calls, 1);
}

#[test]
fn table_driven_with_context() {
    for (price, discount, expected) in [(1000, 0, 1000), (1000, 10, 900), (1000, 100, 0)] {
        verify! {
            let actual = apply_discount(price, discount) => {
                actual == expected;
                actual >= 0 && actual <= price;
            },
            "price={price}, discount={discount}"
        }
    }
}

#[test]
fn comparison_failure_reports_context_action_predicate_and_values() {
    let (price, discount, expected) = (1000, 10, 800);
    let message = panic_message(|| {
        verify! {
            let actual = apply_discount(price, discount) => {
                actual == expected;
            },
            "price={price}, discount={discount}"
        }
    });
    assert!(
        message.starts_with("Verification failed at tests/verify.rs:"),
        "{message}"
    );
    assert!(
        message.contains("Context: price=1000, discount=10\n"),
        "{message}"
    );
    assert!(
        message.contains("Action: apply_discount(price, discount)\n"),
        "{message}"
    );
    assert!(
        message.contains("Predicate: actual == expected\n"),
        "{message}"
    );
    assert!(message.contains("  actual:   900\n"), "{message}");
    assert!(message.contains("  expected: 800\n"), "{message}");
}

#[test]
fn comparison_does_not_require_debug() {
    struct Opaque(u32);
    impl PartialEq for Opaque {
        fn eq(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }
    let message = panic_message(|| {
        verify! {
            let a = Opaque(1) => {
                a == Opaque(2);
            }
        }
    });
    assert!(message.contains("<not Debug>"), "{message}");
}

#[test]
fn boolean_predicate_failure_has_no_values() {
    let message = panic_message(|| {
        verify! {
            let n = 150 => {
                n > 0 && n <= 100;
            }
        }
    });
    assert!(
        message.contains("Predicate: n > 0 && n <= 100\n"),
        "{message}"
    );
    assert!(!message.contains("<not Debug>"), "{message}");
}

#[test]
fn action_pattern_mismatch_fails() {
    let none: Option<u32> = None;
    let message = panic_message(|| {
        verify! {
            let Some(_) = none => {}
        }
    });
    assert!(message.contains("Action: none\n"), "{message}");
    assert!(message.contains("Pattern: Some(_)\n"), "{message}");
}

#[test]
fn observation_pattern_mismatch_fails() {
    let message = panic_message(|| {
        verify! {
            let n = 1 => {
                let Err(e) = Ok::<u32, u32>(n);
                e == 0;
            }
        }
    });
    assert!(message.contains("Pattern: Err(e)\n"), "{message}");
}

#[test]
fn stops_at_first_failure() {
    let mut reached = false;
    let _ = panic_message(|| {
        verify! {
            let n = 1 => {
                n == 2;
                { reached = true; reached };
            }
        }
    });
    assert!(!reached);
}

#[test]
fn context_is_lazy() {
    let mut evaluated = false;
    let mut ctx = || {
        evaluated = true;
        "ctx"
    };
    verify! {
        let n = 1 => {
            n == 1;
        },
        "{}", ctx()
    }
    assert!(!evaluated);
}

#[test]
fn control_flow_follows_the_caller() -> Result<(), String> {
    verify! {
        let n = Ok::<u32, String>(3)? => {
            n == 3;
        }
    }
    for i in 0..3 {
        verify! {
            let n = i => {
                if n == 1 { continue; } else { true };
                n != 1;
            }
        }
    }
    Ok(())
}
