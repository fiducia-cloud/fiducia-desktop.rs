# Formal application lifecycle

The application lifecycle is a security boundary, not a collection of UI
booleans. `AppLifecycleMachine` is the sole owner of session authority, tenant
authority, connectivity mode, protected-action intent, and recovery intent.
Widgets render `AppSnapshot` and use its derived capabilities. Authentication,
storage, network, and API adapters execute typed effects and return typed events.

```text
cold -> bootstrapping -> signed out -> authenticating -> tenant selection
                                |                           |
                                +---------------------------+
                                                            v
ready offline <-> synchronizing -> ready online -> confirmation -> execution
      ^                                                |            |
      |                                                +-- cancel --+
      |                                                             |
      +------ reconciliation required <- unknown result ------------+
                         |
                         v
                    reconciling -> ready online

any authorized phase -> signing out -> signed out
invalid state -> failed -> recovering -> signed out / synchronization
```

Each started operation increments a generation and emits an effect carrying
that exact token. A completion with another token is stale and cannot change
state. Each authority grant or revocation increments an authority epoch. A
pending protected action is bound to that epoch; it cannot survive account or
tenant changes. Sign-out clears session, tenant, and pending-action authority
before the cleanup effect runs.

The transition function is total and deterministic:

- supported events produce one controlled successor;
- unsupported requests reject without changing state;
- stale completions stutter without changing state;
- invalid input or output snapshots revoke authority and enter `failed`;
- uncertain protected writes enter reconciliation and block further writes.

The Quint model is the normative abstraction. Deterministic traces cover the
critical security workflows, randomized simulation searches 10,000 traces,
Apalache checks the invariant exhaustively to the configured bound, and Dart
and Rust tests independently explore their finite transition graphs. This is a
bounded safety claim, not a claim that operating systems, frameworks, services,
or adapters can never fail.
