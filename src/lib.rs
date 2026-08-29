#![forbid(unsafe_code)]

pub mod app_lifecycle;
pub mod deep_link_admission;

pub use app_lifecycle::{
    AppCapabilities, AppEffect, AppEffectKind, AppEvent, AppLifecycleMachine, AppOperation,
    AppPhase, AppSnapshot, AppTransition, PendingAction, TransitionDisposition,
};
pub use deep_link_admission::{
    ALLOWED_DEEP_LINK_ACTIONS, DEEP_LINK_CONTRACT_VERSION, DeepLinkAdmissionMachine,
    DeepLinkDisposition, DeepLinkEffect, DeepLinkHandoff, DeepLinkIntent, DeepLinkPhase,
    DeepLinkReason, DeepLinkSnapshot, DeepLinkTransition, MAXIMUM_DEEP_LINK_BYTES,
};
