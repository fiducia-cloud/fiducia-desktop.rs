# Fiducia Desktop

Native Fiducia operator desktop application implemented in Rust and GPUI, with
no embedded webview. Its application lifecycle is total, deterministic,
runtime-validated, and backed by the same Quint model as the Flutter mobile and
desktop companion in `../fiducia-flutter`.

The production crate separates the headless lifecycle core from the optional
native UI feature. This keeps formal/refinement tests fast and portable while
the shipped desktop binary remains a native GPUI application.

## Safety contract

- Offline mode is read-only.
- Protected actions require online readiness and explicit confirmation.
- Async completions carry operation generations; stale callbacks cannot change
  state.
- Pending actions are bound to the authority epoch that created them.
- Ambiguous write outcomes block new writes until reconciliation finishes.
- Sign-out revokes local authority before cleanup begins.
- Invalid snapshots fail closed with no session, tenant, or pending action.

See `docs/FORMAL_METHODS.md`, `docs/DESKTOP_TOOLKIT.md`, and `formal/README.md`.
Delivery is tracked by DEN-3971.

## Develop

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo check --features desktop-ui --bin fiducia-desktop
```

The last command requires Xcode's Metal toolchain on macOS because GPUI
compiles native shaders. The crate is private and is not published to crates.io.
