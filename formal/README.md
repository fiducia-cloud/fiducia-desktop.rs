# Fiducia application lifecycle model

`app_lifecycle.qnt` is the normative, platform-independent safety model for
Fiducia mobile and desktop clients. The Dart and Rust implementations must
refine the same phases, operation generations, authority epochs, effects, and
fail-closed rules. The model and deterministic trace file are intentionally
byte-identical in `fiducia-flutter` and `fiducia-desktop.rs`.

The model owns these safety decisions:

- session and tenant authority are present only in compatible phases;
- offline mode is read-only;
- a privileged action requires online readiness and explicit confirmation;
- every async completion is fenced by the generation that started it;
- an ambiguous privileged-action result blocks new actions until reconciliation;
- sign-out revokes local authority before cleanup starts;
- invalid runtime snapshots enter a controlled failed state with no authority;
- UI capabilities are derived from state, never maintained independently.

## Local verification

Use the pinned Quint release:

```sh
QUINT_PACKAGE='@informalsystems/quint@0.32.0'
npx --yes --package="$QUINT_PACKAGE" quint typecheck formal/app_lifecycle.qnt
npx --yes --package="$QUINT_PACKAGE" quint typecheck formal/app_lifecycle_test.qnt
npx --yes --package="$QUINT_PACKAGE" quint test formal/app_lifecycle_test.qnt \
  --main=app_lifecycle_test --match='.*Test$'
npx --yes --package="$QUINT_PACKAGE" quint run formal/app_lifecycle.qnt \
  --main=app_lifecycle --max-samples=10000 --max-steps=30 \
  --invariant=app_lifecycle_safety
npx --yes --package="$QUINT_PACKAGE" quint verify formal/app_lifecycle.qnt \
  --main=app_lifecycle --max-steps=8 --invariant=app_lifecycle_safety
```

Run `flutter test` in this repository and `cargo test --all-targets` in the
Rust companion. Those independent finite-state explorations check totality,
determinism, invariant preservation, stale-token behavior, and phase coverage
against each production implementation.

## Claim boundary

Passing these checks proves the stated invariant only for the abstract model
and the explored bounds. It does not prove platform frameworks, identity
providers, storage, networks, or service implementations correct. The runtime
machines therefore validate every pre-state and post-state and fail closed on
corruption. Expanding the product lifecycle requires updating the shared model,
both implementations, deterministic traces, and bounded implementation tests
in the same change.
