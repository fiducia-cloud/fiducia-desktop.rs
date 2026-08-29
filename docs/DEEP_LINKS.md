# Formal HTTPS deep-link admission

Fiducia accepts only these canonical HTTPS links:

- `https://fiducia.cloud/open?action=rotate-api-key`
- `https://fiducia.cloud/open?action=review-reconciliation`

A link is an untrusted request, never an authorization grant. Admission parses
the raw platform string with a deliberately small grammar, and a separate
generation-fenced state machine records the latest accepted intent. Consuming
that intent submits `actionRequested` to the application lifecycle machine.
The lifecycle remains the sole authority: it accepts the request only from
`readyOnline` and then requires explicit user confirmation before any protected
effect can run.

Admission fails closed for non-HTTPS schemes, another host or port, another
path, fragments, credentials, encoded or `+`-spelled query data, duplicate or
additional query parameters, unknown actions, malformed input, non-canonical
spelling, or input longer than 2048 UTF-8 bytes. Rejection does not erase the
last accepted intent. A late resolution tagged with an older generation is a
stutter and cannot replace a newer capture. Portable generation-counter
exhaustion revokes admitted intent and fails closed instead of reusing a token.

```text
idle -- capture --> resolving -- accept --> pending -- lifecycle accepts --> idle
                       |    ^                  |
                       |    +-- stale --------+
                       +-- reject ----------> idle
```

The cross-runtime contract is checked through:

- `contracts/deep_link.schema.json` for the wire contract;
- `contracts/deep_link_vectors.json` for shared hostile and valid inputs;
- `formal/deep_link_admission.qnt` for the abstract admission machine;
- `formal/deep_link_admission_test.qnt` for deterministic formal traces;
- `contracts/parity.sha256` for byte-for-byte Flutter/Rust parity.

The Flutter companion creates `AppLinks` before rendering and admits values
from its raw string stream. Android declares a verified HTTPS intent filter;
the iOS target and signed macOS release target declare the associated-domain
entitlement there. This Rust shell currently admits an operating-system-
provided URL argument through the same contract.

The proof does not establish control of `fiducia.cloud`, correctness of OS URL
delivery, signing identity, or native association. Release still requires the
site to serve correctly signed Android Digital Asset Links and Apple App Site
Association files, plus signed physical/native platform tests. Those are
external deployment gates rather than assumptions hidden inside the model.
