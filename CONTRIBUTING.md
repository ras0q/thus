# Contributing

```sh
cargo test
cargo clippy --all-targets
cargo fmt
```

- Public API is `verify!` only.
- Keep Rust's evaluation semantics: no implicit clone, no re-evaluation for diagnostics.
- Keep dependencies minimal. Discuss before adding one.
- Test behavior through `verify!`. For failures, assert on the panic message.
