#![forbid(unsafe_code)]

pub mod app_lifecycle;

pub use app_lifecycle::{
    AppCapabilities, AppEffect, AppEffectKind, AppEvent, AppLifecycleMachine, AppOperation,
    AppPhase, AppSnapshot, AppTransition, PendingAction, TransitionDisposition,
};
