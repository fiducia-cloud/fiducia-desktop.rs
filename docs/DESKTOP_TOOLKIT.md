# Desktop toolkit decision

The native desktop client uses GPUI `0.2.2`, pinned exactly in `Cargo.toml`.
This follows the Fiducia desktop policy: Rust-native rendering, shared Rust
domain logic, and no webview or web-first shell.

The `desktop-ui` Cargo feature owns the GPUI dependency and binary. The default
feature set contains only the headless lifecycle library so state-machine,
formal-refinement, and server-side CI checks do not need a display server or
graphics SDK. Release builds enable `desktop-ui` on a native host.

GPUI's macOS build compiles Metal shaders. Install the Xcode Metal toolchain if
`cargo check --features desktop-ui --bin fiducia-desktop` reports that the
`metal` tool is unavailable. This is a host prerequisite, not a reason to
replace the native toolkit or weaken lifecycle tests.
