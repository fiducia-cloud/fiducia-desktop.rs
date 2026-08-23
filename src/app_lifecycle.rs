//! Total, deterministic lifecycle shared by the native Fiducia desktop shell.
//!
//! The transition relation is pure. Authentication, networking, storage, and
//! privileged API calls are explicit effects whose completions carry the exact
//! operation generation that created them. Unsupported requests reject;
//! stale completions stutter; corrupt snapshots revoke local authority and
//! enter a controlled failed state.

pub const MAX_PORTABLE_COUNTER: u64 = 9_007_199_254_740_991;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppPhase {
    Cold,
    Bootstrapping,
    SignedOut,
    Authenticating,
    SelectingTenant,
    Synchronizing,
    ReadyOnline,
    ReadyOffline,
    ConfirmingAction,
    ExecutingAction,
    ReconciliationRequired,
    ReconcilingAction,
    Recovering,
    SigningOut,
    Failed,
}

impl AppPhase {
    pub fn is_busy(self) -> bool {
        matches!(
            self,
            Self::Bootstrapping
                | Self::Authenticating
                | Self::Synchronizing
                | Self::ExecutingAction
                | Self::ReconcilingAction
                | Self::Recovering
                | Self::SigningOut
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppOperation {
    Bootstrap,
    Authenticate,
    Synchronize,
    ExecuteAction,
    ReconcileAction,
    Recover,
    SignOut,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppEffectKind {
    LoadSession,
    Authenticate,
    Synchronize,
    ExecuteAction,
    ReconcileAction,
    RecoverSession,
    ClearSession,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PendingAction {
    id: String,
    authority_epoch: u64,
}

impl PendingAction {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn authority_epoch(&self) -> u64 {
        self.authority_epoch
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppCapabilities {
    pub can_launch: bool,
    pub can_sign_in: bool,
    pub can_select_tenant: bool,
    pub can_read_cached_data: bool,
    pub can_request_privileged_action: bool,
    pub can_confirm_action: bool,
    pub can_cancel_action: bool,
    pub can_recover: bool,
    pub can_sign_out: bool,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AppSnapshot {
    phase: AppPhase,
    generation: u64,
    authority_epoch: u64,
    has_session: bool,
    has_tenant: bool,
    online: bool,
    active_operation: Option<AppOperation>,
    active_operation_id: Option<u64>,
    pending_action: Option<PendingAction>,
    failure: Option<String>,
}

impl Default for AppSnapshot {
    fn default() -> Self {
        Self {
            phase: AppPhase::Cold,
            generation: 0,
            authority_epoch: 0,
            has_session: false,
            has_tenant: false,
            online: false,
            active_operation: None,
            active_operation_id: None,
            pending_action: None,
            failure: None,
        }
    }
}

impl AppSnapshot {
    pub fn phase(&self) -> AppPhase {
        self.phase
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn authority_epoch(&self) -> u64 {
        self.authority_epoch
    }

    pub fn has_session(&self) -> bool {
        self.has_session
    }

    pub fn has_tenant(&self) -> bool {
        self.has_tenant
    }

    pub fn online(&self) -> bool {
        self.online
    }

    pub fn active_operation(&self) -> Option<AppOperation> {
        self.active_operation
    }

    pub fn active_operation_id(&self) -> Option<u64> {
        self.active_operation_id
    }

    pub fn pending_action(&self) -> Option<&PendingAction> {
        self.pending_action.as_ref()
    }

    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    pub fn capabilities(&self) -> AppCapabilities {
        AppCapabilities {
            can_launch: self.phase == AppPhase::Cold,
            can_sign_in: self.phase == AppPhase::SignedOut,
            can_select_tenant: self.phase == AppPhase::SelectingTenant,
            can_read_cached_data: matches!(
                self.phase,
                AppPhase::ReadyOnline
                    | AppPhase::ReadyOffline
                    | AppPhase::ConfirmingAction
                    | AppPhase::ExecutingAction
                    | AppPhase::ReconciliationRequired
                    | AppPhase::ReconcilingAction
            ),
            can_request_privileged_action: self.phase == AppPhase::ReadyOnline,
            can_confirm_action: self.phase == AppPhase::ConfirmingAction,
            can_cancel_action: self.phase == AppPhase::ConfirmingAction,
            can_recover: self.phase == AppPhase::Failed,
            can_sign_out: matches!(
                self.phase,
                AppPhase::SelectingTenant
                    | AppPhase::Synchronizing
                    | AppPhase::ReadyOnline
                    | AppPhase::ReadyOffline
                    | AppPhase::ConfirmingAction
                    | AppPhase::ExecutingAction
                    | AppPhase::ReconciliationRequired
                    | AppPhase::ReconcilingAction
                    | AppPhase::Failed
            ),
        }
    }

    /// Runtime invariant checked before and after every transition.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.generation > MAX_PORTABLE_COUNTER {
            return Err("generation is outside the portable domain");
        }
        if self.authority_epoch > MAX_PORTABLE_COUNTER {
            return Err("authority epoch is outside the portable domain");
        }
        let has_complete_operation =
            self.active_operation.is_some() && self.active_operation_id.is_some();
        if self.phase.is_busy() != has_complete_operation {
            return Err("transitional phases require exactly one active operation");
        }
        if self.active_operation.is_some() != self.active_operation_id.is_some() {
            return Err("active operation and id must appear together");
        }
        if self
            .active_operation_id
            .is_some_and(|id| id == 0 || id != self.generation)
        {
            return Err("active operation id must equal the current generation");
        }
        let operation_matches_phase = match self.phase {
            AppPhase::Bootstrapping => self.active_operation == Some(AppOperation::Bootstrap),
            AppPhase::Authenticating => self.active_operation == Some(AppOperation::Authenticate),
            AppPhase::Synchronizing => self.active_operation == Some(AppOperation::Synchronize),
            AppPhase::ExecutingAction => self.active_operation == Some(AppOperation::ExecuteAction),
            AppPhase::ReconcilingAction => {
                self.active_operation == Some(AppOperation::ReconcileAction)
            }
            AppPhase::Recovering => self.active_operation == Some(AppOperation::Recover),
            AppPhase::SigningOut => self.active_operation == Some(AppOperation::SignOut),
            _ => self.active_operation.is_none(),
        };
        if !operation_matches_phase {
            return Err("active operation is incompatible with the current phase");
        }
        if self.has_tenant && !self.has_session {
            return Err("tenant authority requires an authenticated session");
        }
        let requires_session = matches!(
            self.phase,
            AppPhase::SelectingTenant
                | AppPhase::Synchronizing
                | AppPhase::ReadyOnline
                | AppPhase::ReadyOffline
                | AppPhase::ConfirmingAction
                | AppPhase::ExecutingAction
                | AppPhase::ReconciliationRequired
                | AppPhase::ReconcilingAction
        );
        if requires_session != self.has_session {
            return Err("session presence is incompatible with the current phase");
        }
        let requires_tenant = matches!(
            self.phase,
            AppPhase::Synchronizing
                | AppPhase::ReadyOnline
                | AppPhase::ReadyOffline
                | AppPhase::ConfirmingAction
                | AppPhase::ExecutingAction
                | AppPhase::ReconciliationRequired
                | AppPhase::ReconcilingAction
        );
        if requires_tenant != self.has_tenant {
            return Err("tenant selection is incompatible with the current phase");
        }
        let requires_online = matches!(
            self.phase,
            AppPhase::Synchronizing
                | AppPhase::ReadyOnline
                | AppPhase::ConfirmingAction
                | AppPhase::ExecutingAction
                | AppPhase::ReconcilingAction
        );
        if requires_online && !self.online {
            return Err("the current phase requires verified online connectivity");
        }
        if self.phase == AppPhase::ReadyOffline && self.online {
            return Err("offline readiness cannot claim online connectivity");
        }
        let requires_pending_action = matches!(
            self.phase,
            AppPhase::ConfirmingAction
                | AppPhase::ExecutingAction
                | AppPhase::ReconciliationRequired
                | AppPhase::ReconcilingAction
        );
        if requires_pending_action != self.pending_action.is_some() {
            return Err("pending action presence is incompatible with the current phase");
        }
        if self.pending_action.as_ref().is_some_and(|action| {
            action.id.is_empty()
                || action.id.len() > 128
                || action.authority_epoch != self.authority_epoch
        }) {
            return Err("pending action is not bound to current authority");
        }
        if (self.phase == AppPhase::Failed) != self.failure.is_some() {
            return Err("failure details must exist exactly in the failed phase");
        }
        Ok(())
    }

    fn stable(
        &self,
        phase: AppPhase,
        has_session: bool,
        has_tenant: bool,
        online: bool,
        authority_epoch: u64,
        pending_action: Option<PendingAction>,
    ) -> Self {
        Self {
            phase,
            generation: self.generation,
            authority_epoch,
            has_session,
            has_tenant,
            online,
            active_operation: None,
            active_operation_id: None,
            pending_action,
            failure: None,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum AppEvent {
    LaunchRequested,
    BootstrapSucceeded {
        operation_id: u64,
        authenticated: bool,
        has_tenant: bool,
        online: bool,
    },
    SignInRequested,
    AuthenticationSucceeded {
        operation_id: u64,
        has_tenant: bool,
        online: bool,
    },
    TenantSelected,
    ConnectivityChanged {
        online: bool,
    },
    SyncRequested,
    ActionRequested {
        action_id: String,
    },
    ActionConfirmed,
    ActionCancelled,
    OperationSucceeded {
        operation_id: u64,
    },
    OperationFailed {
        operation_id: u64,
        reason: String,
        retryable: bool,
        ambiguous: bool,
    },
    SignOutRequested,
    SessionRevoked,
    RecoveryRequested,
    RecoverySucceeded {
        operation_id: u64,
        authenticated: bool,
        has_tenant: bool,
        online: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppEffect {
    pub kind: AppEffectKind,
    pub operation: AppOperation,
    pub operation_id: u64,
    pub action_id: Option<String>,
    pub authority_epoch: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TransitionDisposition {
    Applied,
    Rejected,
    Stale,
    FailedClosed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppTransition {
    pub before: AppSnapshot,
    pub after: AppSnapshot,
    pub disposition: TransitionDisposition,
    pub reason: String,
    pub effect: Option<AppEffect>,
}

#[derive(Clone, Debug, Default)]
pub struct AppLifecycleMachine {
    snapshot: AppSnapshot,
}

impl AppLifecycleMachine {
    pub fn snapshot(&self) -> &AppSnapshot {
        &self.snapshot
    }

    pub fn dispatch(&mut self, event: &AppEvent) -> AppTransition {
        let transition = Self::transition(&self.snapshot, event);
        self.snapshot = transition.after.clone();
        transition
    }

    /// Pure, total transition relation for every snapshot and event value.
    pub fn transition(current: &AppSnapshot, event: &AppEvent) -> AppTransition {
        if let Err(violation) = current.validate() {
            return Self::failed_closed(
                current,
                &format!("invalid lifecycle snapshot: {violation}"),
            );
        }
        let result = match event {
            AppEvent::LaunchRequested => Self::launch(current),
            AppEvent::BootstrapSucceeded {
                operation_id,
                authenticated,
                has_tenant,
                online,
            } => {
                if !Self::matches(current, AppOperation::Bootstrap, *operation_id) {
                    Self::stale(current, "stale bootstrap completion ignored")
                } else {
                    Self::establish_authority(
                        current,
                        *authenticated,
                        *has_tenant,
                        *online,
                        "session bootstrap completed",
                    )
                }
            }
            AppEvent::SignInRequested => Self::sign_in(current),
            AppEvent::AuthenticationSucceeded {
                operation_id,
                has_tenant,
                online,
            } => {
                if !Self::matches(current, AppOperation::Authenticate, *operation_id) {
                    Self::stale(current, "stale authentication completion ignored")
                } else {
                    Self::establish_authority(
                        current,
                        true,
                        *has_tenant,
                        *online,
                        "authentication completed",
                    )
                }
            }
            AppEvent::TenantSelected => Self::tenant_selected(current),
            AppEvent::ConnectivityChanged { online } => {
                Self::connectivity_changed(current, *online)
            }
            AppEvent::SyncRequested => Self::sync_requested(current),
            AppEvent::ActionRequested { action_id } => Self::action_requested(current, action_id),
            AppEvent::ActionConfirmed => Self::action_confirmed(current),
            AppEvent::ActionCancelled => Self::action_cancelled(current),
            AppEvent::OperationSucceeded { operation_id } => {
                Self::operation_succeeded(current, *operation_id)
            }
            AppEvent::OperationFailed {
                operation_id,
                reason,
                retryable,
                ambiguous,
            } => Self::operation_failed(current, *operation_id, reason, *retryable, *ambiguous),
            AppEvent::SignOutRequested => Self::sign_out(current, "sign-out requested"),
            AppEvent::SessionRevoked => Self::sign_out(current, "session revoked"),
            AppEvent::RecoveryRequested => Self::recovery_requested(current),
            AppEvent::RecoverySucceeded {
                operation_id,
                authenticated,
                has_tenant,
                online,
            } => {
                if !Self::matches(current, AppOperation::Recover, *operation_id) {
                    Self::stale(current, "stale recovery completion ignored")
                } else {
                    Self::establish_authority(
                        current,
                        *authenticated,
                        *has_tenant,
                        *online,
                        "explicit recovery completed",
                    )
                }
            }
        };
        if let Err(violation) = result.after.validate() {
            return Self::failed_closed(
                current,
                &format!("transition produced invalid lifecycle snapshot: {violation}"),
            );
        }
        result
    }

    fn launch(current: &AppSnapshot) -> AppTransition {
        if current.phase != AppPhase::Cold {
            return Self::rejected(current, "launch is available only from cold state");
        }
        Self::begin(
            current,
            Begin {
                phase: AppPhase::Bootstrapping,
                operation: AppOperation::Bootstrap,
                effect: AppEffectKind::LoadSession,
                has_session: false,
                has_tenant: false,
                online: false,
                pending_action: None,
                authority_epoch: current.authority_epoch,
                reason: "session bootstrap started".to_owned(),
            },
        )
    }

    fn sign_in(current: &AppSnapshot) -> AppTransition {
        if current.phase != AppPhase::SignedOut {
            return Self::rejected(current, "sign-in requires signed-out state");
        }
        Self::begin(
            current,
            Begin {
                phase: AppPhase::Authenticating,
                operation: AppOperation::Authenticate,
                effect: AppEffectKind::Authenticate,
                has_session: false,
                has_tenant: false,
                online: current.online,
                pending_action: None,
                authority_epoch: current.authority_epoch,
                reason: "authentication started".to_owned(),
            },
        )
    }

    fn tenant_selected(current: &AppSnapshot) -> AppTransition {
        if current.phase != AppPhase::SelectingTenant {
            return Self::rejected(current, "tenant selection is not currently available");
        }
        let Some(authority_epoch) = Self::next_authority(current) else {
            return Self::failed_closed(current, "authority epoch exhausted");
        };
        let selected = current.stable(
            if current.online {
                AppPhase::ReadyOnline
            } else {
                AppPhase::ReadyOffline
            },
            true,
            true,
            current.online,
            authority_epoch,
            None,
        );
        if !current.online {
            return Self::applied(current, selected, "tenant selected for offline mode", None);
        }
        Self::begin(
            &selected,
            Begin {
                phase: AppPhase::Synchronizing,
                operation: AppOperation::Synchronize,
                effect: AppEffectKind::Synchronize,
                has_session: true,
                has_tenant: true,
                online: true,
                pending_action: None,
                authority_epoch,
                reason: "tenant selected; synchronization started".to_owned(),
            },
        )
        .with_before(current)
    }

    fn connectivity_changed(current: &AppSnapshot, online: bool) -> AppTransition {
        if current.online == online {
            return Self::rejected(current, "connectivity state is unchanged");
        }
        if online {
            return match current.phase {
                AppPhase::SignedOut | AppPhase::SelectingTenant => Self::applied(
                    current,
                    current.stable(
                        current.phase,
                        current.has_session,
                        current.has_tenant,
                        true,
                        current.authority_epoch,
                        None,
                    ),
                    "connectivity restored",
                    None,
                ),
                AppPhase::ReadyOffline => {
                    Self::begin_sync(current, "connectivity restored; synchronization started")
                }
                AppPhase::ReconciliationRequired => Self::begin_reconciliation(
                    current,
                    "connectivity restored; action reconciliation started",
                ),
                _ => Self::rejected(
                    current,
                    "connectivity restoration is not applicable in this phase",
                ),
            };
        }
        match current.phase {
            AppPhase::SignedOut | AppPhase::SelectingTenant => Self::applied(
                current,
                current.stable(
                    current.phase,
                    current.has_session,
                    current.has_tenant,
                    false,
                    current.authority_epoch,
                    None,
                ),
                "connectivity lost",
                None,
            ),
            AppPhase::ReadyOnline => Self::applied(
                current,
                current.stable(
                    AppPhase::ReadyOffline,
                    true,
                    true,
                    false,
                    current.authority_epoch,
                    None,
                ),
                "entered controlled read-only offline mode",
                None,
            ),
            AppPhase::Synchronizing => Self::fence_to_stable(
                current,
                AppPhase::ReadyOffline,
                true,
                true,
                false,
                None,
                "sync fenced after connectivity loss",
            ),
            AppPhase::ConfirmingAction => Self::applied(
                current,
                current.stable(
                    AppPhase::ReadyOffline,
                    true,
                    true,
                    false,
                    current.authority_epoch,
                    None,
                ),
                "unexecuted action cancelled before entering offline mode",
                None,
            ),
            AppPhase::ExecutingAction | AppPhase::ReconcilingAction => Self::fence_to_stable(
                current,
                AppPhase::ReconciliationRequired,
                true,
                true,
                false,
                current.pending_action.clone(),
                "ambiguous action fenced pending online reconciliation",
            ),
            AppPhase::Authenticating => Self::fence_to_stable(
                current,
                AppPhase::SignedOut,
                false,
                false,
                false,
                None,
                "authentication fenced after connectivity loss",
            ),
            _ => Self::rejected(current, "connectivity loss is not applicable in this phase"),
        }
    }

    fn sync_requested(current: &AppSnapshot) -> AppTransition {
        if current.phase != AppPhase::ReadyOnline {
            return Self::rejected(current, "manual sync requires online readiness");
        }
        Self::begin_sync(current, "manual synchronization started")
    }

    fn action_requested(current: &AppSnapshot, action_id: &str) -> AppTransition {
        let action_id = action_id.trim();
        if current.phase != AppPhase::ReadyOnline {
            return Self::rejected(current, "privileged actions require online readiness");
        }
        if action_id.is_empty() || action_id.len() > 128 {
            return Self::rejected(current, "action id must contain 1 to 128 bytes");
        }
        Self::applied(
            current,
            current.stable(
                AppPhase::ConfirmingAction,
                true,
                true,
                true,
                current.authority_epoch,
                Some(PendingAction {
                    id: action_id.to_owned(),
                    authority_epoch: current.authority_epoch,
                }),
            ),
            "privileged action awaits explicit confirmation",
            None,
        )
    }

    fn action_confirmed(current: &AppSnapshot) -> AppTransition {
        if current.phase != AppPhase::ConfirmingAction
            || current.pending_action.is_none()
            || !current.online
        {
            return Self::rejected(
                current,
                "confirmation requires a bound online pending action",
            );
        }
        Self::begin(
            current,
            Begin {
                phase: AppPhase::ExecutingAction,
                operation: AppOperation::ExecuteAction,
                effect: AppEffectKind::ExecuteAction,
                has_session: true,
                has_tenant: true,
                online: true,
                pending_action: current.pending_action.clone(),
                authority_epoch: current.authority_epoch,
                reason: "confirmed privileged action started".to_owned(),
            },
        )
    }

    fn action_cancelled(current: &AppSnapshot) -> AppTransition {
        if current.phase != AppPhase::ConfirmingAction {
            return Self::rejected(current, "no action is awaiting confirmation");
        }
        Self::applied(
            current,
            current.stable(
                AppPhase::ReadyOnline,
                true,
                true,
                true,
                current.authority_epoch,
                None,
            ),
            "pending action cancelled without execution",
            None,
        )
    }

    fn operation_succeeded(current: &AppSnapshot, operation_id: u64) -> AppTransition {
        if current.active_operation.is_none() || current.active_operation_id != Some(operation_id) {
            return Self::stale(current, "stale operation success ignored");
        }
        match current
            .active_operation
            .expect("validated active operation")
        {
            AppOperation::Synchronize => Self::applied(
                current,
                current.stable(
                    AppPhase::ReadyOnline,
                    true,
                    true,
                    true,
                    current.authority_epoch,
                    None,
                ),
                "synchronization completed",
                None,
            ),
            AppOperation::ExecuteAction => Self::applied(
                current,
                current.stable(
                    AppPhase::ReadyOnline,
                    true,
                    true,
                    true,
                    current.authority_epoch,
                    None,
                ),
                "privileged action completed",
                None,
            ),
            AppOperation::ReconcileAction => Self::applied(
                current,
                current.stable(
                    AppPhase::ReadyOnline,
                    true,
                    true,
                    true,
                    current.authority_epoch,
                    None,
                ),
                "ambiguous action reconciled",
                None,
            ),
            AppOperation::SignOut => Self::applied(
                current,
                current.stable(
                    AppPhase::SignedOut,
                    false,
                    false,
                    current.online,
                    current.authority_epoch,
                    None,
                ),
                "local and remote sign-out completed",
                None,
            ),
            AppOperation::Bootstrap | AppOperation::Authenticate | AppOperation::Recover => {
                Self::rejected(current, "typed completion is required for this operation")
            }
        }
    }

    fn operation_failed(
        current: &AppSnapshot,
        operation_id: u64,
        reason: &str,
        retryable: bool,
        ambiguous: bool,
    ) -> AppTransition {
        if current.active_operation.is_none() || current.active_operation_id != Some(operation_id) {
            return Self::stale(current, "stale operation failure ignored");
        }
        let controlled = Self::controlled_reason(reason);
        match current
            .active_operation
            .expect("validated active operation")
        {
            AppOperation::Authenticate => Self::applied(
                current,
                current.stable(
                    AppPhase::SignedOut,
                    false,
                    false,
                    current.online,
                    current.authority_epoch,
                    None,
                ),
                "authentication failed without granting authority",
                None,
            ),
            AppOperation::Synchronize if retryable => Self::applied(
                current,
                current.stable(
                    AppPhase::ReadyOffline,
                    true,
                    true,
                    false,
                    current.authority_epoch,
                    None,
                ),
                &format!("sync failure entered read-only offline mode: {controlled}"),
                None,
            ),
            AppOperation::ExecuteAction if ambiguous => {
                if current.online {
                    Self::begin_reconciliation(
                        current,
                        "ambiguous action result requires reconciliation",
                    )
                } else {
                    Self::fence_to_stable(
                        current,
                        AppPhase::ReconciliationRequired,
                        true,
                        true,
                        false,
                        current.pending_action.clone(),
                        "ambiguous action awaits online reconciliation",
                    )
                }
            }
            AppOperation::ExecuteAction => Self::applied(
                current,
                current.stable(
                    if current.online {
                        AppPhase::ReadyOnline
                    } else {
                        AppPhase::ReadyOffline
                    },
                    true,
                    true,
                    current.online,
                    current.authority_epoch,
                    None,
                ),
                &format!("action failed definitively without committing: {controlled}"),
                None,
            ),
            AppOperation::SignOut => Self::applied(
                current,
                current.stable(
                    AppPhase::SignedOut,
                    false,
                    false,
                    current.online,
                    current.authority_epoch,
                    None,
                ),
                "local authority remains revoked despite cleanup failure",
                None,
            ),
            AppOperation::Bootstrap
            | AppOperation::Synchronize
            | AppOperation::ReconcileAction
            | AppOperation::Recover => Self::failed_closed(current, &controlled),
        }
    }

    fn sign_out(current: &AppSnapshot, reason: &str) -> AppTransition {
        if matches!(
            current.phase,
            AppPhase::Cold | AppPhase::SignedOut | AppPhase::SigningOut
        ) {
            return Self::rejected(current, "sign-out is not applicable in this phase");
        }
        let Some(authority_epoch) = Self::next_authority(current) else {
            return Self::failed_closed(current, "authority epoch exhausted");
        };
        Self::begin(
            current,
            Begin {
                phase: AppPhase::SigningOut,
                operation: AppOperation::SignOut,
                effect: AppEffectKind::ClearSession,
                has_session: false,
                has_tenant: false,
                online: current.online,
                pending_action: None,
                authority_epoch,
                reason: format!("{reason}; local authority revoked before cleanup"),
            },
        )
    }

    fn recovery_requested(current: &AppSnapshot) -> AppTransition {
        if current.phase != AppPhase::Failed {
            return Self::rejected(current, "recovery requires controlled failed state");
        }
        Self::begin(
            current,
            Begin {
                phase: AppPhase::Recovering,
                operation: AppOperation::Recover,
                effect: AppEffectKind::RecoverSession,
                has_session: false,
                has_tenant: false,
                online: false,
                pending_action: None,
                authority_epoch: current.authority_epoch,
                reason: "explicit recovery started".to_owned(),
            },
        )
    }

    fn establish_authority(
        current: &AppSnapshot,
        authenticated: bool,
        has_tenant: bool,
        online: bool,
        reason: &str,
    ) -> AppTransition {
        if !authenticated {
            return Self::applied(
                current,
                current.stable(
                    AppPhase::SignedOut,
                    false,
                    false,
                    online,
                    current.authority_epoch,
                    None,
                ),
                &format!("{reason} without an authenticated session"),
                None,
            );
        }
        let Some(authority_epoch) = Self::next_authority(current) else {
            return Self::failed_closed(current, "authority epoch exhausted");
        };
        if !has_tenant {
            return Self::applied(
                current,
                current.stable(
                    AppPhase::SelectingTenant,
                    true,
                    false,
                    online,
                    authority_epoch,
                    None,
                ),
                &format!("{reason}; tenant selection required"),
                None,
            );
        }
        let authorized = current.stable(
            if online {
                AppPhase::ReadyOnline
            } else {
                AppPhase::ReadyOffline
            },
            true,
            true,
            online,
            authority_epoch,
            None,
        );
        if !online {
            return Self::applied(
                current,
                authorized,
                &format!("{reason} into controlled read-only offline mode"),
                None,
            );
        }
        Self::begin(
            &authorized,
            Begin {
                phase: AppPhase::Synchronizing,
                operation: AppOperation::Synchronize,
                effect: AppEffectKind::Synchronize,
                has_session: true,
                has_tenant: true,
                online: true,
                pending_action: None,
                authority_epoch,
                reason: format!("{reason}; synchronization started"),
            },
        )
        .with_before(current)
    }

    fn begin_sync(current: &AppSnapshot, reason: &str) -> AppTransition {
        Self::begin(
            current,
            Begin {
                phase: AppPhase::Synchronizing,
                operation: AppOperation::Synchronize,
                effect: AppEffectKind::Synchronize,
                has_session: true,
                has_tenant: true,
                online: true,
                pending_action: None,
                authority_epoch: current.authority_epoch,
                reason: reason.to_owned(),
            },
        )
    }

    fn begin_reconciliation(current: &AppSnapshot, reason: &str) -> AppTransition {
        Self::begin(
            current,
            Begin {
                phase: AppPhase::ReconcilingAction,
                operation: AppOperation::ReconcileAction,
                effect: AppEffectKind::ReconcileAction,
                has_session: true,
                has_tenant: true,
                online: true,
                pending_action: current.pending_action.clone(),
                authority_epoch: current.authority_epoch,
                reason: reason.to_owned(),
            },
        )
    }

    fn matches(current: &AppSnapshot, operation: AppOperation, operation_id: u64) -> bool {
        current.active_operation == Some(operation)
            && current.active_operation_id == Some(operation_id)
    }

    fn next_authority(current: &AppSnapshot) -> Option<u64> {
        (current.authority_epoch < MAX_PORTABLE_COUNTER).then_some(current.authority_epoch + 1)
    }

    fn begin(current: &AppSnapshot, begin: Begin) -> AppTransition {
        if current.generation >= MAX_PORTABLE_COUNTER {
            return Self::failed_closed(current, "operation generation exhausted");
        }
        let operation_id = current.generation + 1;
        let action_id = begin
            .pending_action
            .as_ref()
            .map(|action| action.id.clone());
        let action_authority = begin
            .pending_action
            .as_ref()
            .map(|action| action.authority_epoch);
        let after = AppSnapshot {
            phase: begin.phase,
            generation: operation_id,
            authority_epoch: begin.authority_epoch,
            has_session: begin.has_session,
            has_tenant: begin.has_tenant,
            online: begin.online,
            active_operation: Some(begin.operation),
            active_operation_id: Some(operation_id),
            pending_action: begin.pending_action,
            failure: None,
        };
        Self::applied(
            current,
            after,
            &begin.reason,
            Some(AppEffect {
                kind: begin.effect,
                operation: begin.operation,
                operation_id,
                action_id,
                authority_epoch: action_authority,
            }),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn fence_to_stable(
        current: &AppSnapshot,
        phase: AppPhase,
        has_session: bool,
        has_tenant: bool,
        online: bool,
        pending_action: Option<PendingAction>,
        reason: &str,
    ) -> AppTransition {
        if current.generation >= MAX_PORTABLE_COUNTER {
            return Self::failed_closed(current, "operation generation exhausted");
        }
        Self::applied(
            current,
            AppSnapshot {
                phase,
                generation: current.generation + 1,
                authority_epoch: current.authority_epoch,
                has_session,
                has_tenant,
                online,
                active_operation: None,
                active_operation_id: None,
                pending_action,
                failure: None,
            },
            reason,
            None,
        )
    }

    fn applied(
        before: &AppSnapshot,
        after: AppSnapshot,
        reason: &str,
        effect: Option<AppEffect>,
    ) -> AppTransition {
        AppTransition {
            before: before.clone(),
            after,
            disposition: TransitionDisposition::Applied,
            reason: reason.to_owned(),
            effect,
        }
    }

    fn rejected(current: &AppSnapshot, reason: &str) -> AppTransition {
        AppTransition {
            before: current.clone(),
            after: current.clone(),
            disposition: TransitionDisposition::Rejected,
            reason: reason.to_owned(),
            effect: None,
        }
    }

    fn stale(current: &AppSnapshot, reason: &str) -> AppTransition {
        AppTransition {
            before: current.clone(),
            after: current.clone(),
            disposition: TransitionDisposition::Stale,
            reason: reason.to_owned(),
            effect: None,
        }
    }

    fn failed_closed(current: &AppSnapshot, reason: &str) -> AppTransition {
        AppTransition {
            before: current.clone(),
            after: AppSnapshot {
                phase: AppPhase::Failed,
                generation: current
                    .generation
                    .checked_add(1)
                    .filter(|value| *value <= MAX_PORTABLE_COUNTER)
                    .unwrap_or(current.generation.min(MAX_PORTABLE_COUNTER)),
                authority_epoch: current
                    .authority_epoch
                    .checked_add(1)
                    .filter(|value| *value <= MAX_PORTABLE_COUNTER)
                    .unwrap_or(current.authority_epoch.min(MAX_PORTABLE_COUNTER)),
                has_session: false,
                has_tenant: false,
                online: false,
                active_operation: None,
                active_operation_id: None,
                pending_action: None,
                failure: Some(Self::controlled_reason(reason)),
            },
            disposition: TransitionDisposition::FailedClosed,
            reason: "lifecycle failed closed".to_owned(),
            effect: None,
        }
    }

    fn controlled_reason(reason: &str) -> String {
        let reason = reason.trim();
        if reason.is_empty() {
            return "unspecified controlled failure".to_owned();
        }
        reason.chars().take(256).collect()
    }
}

struct Begin {
    phase: AppPhase,
    operation: AppOperation,
    effect: AppEffectKind,
    has_session: bool,
    has_tenant: bool,
    online: bool,
    pending_action: Option<PendingAction>,
    authority_epoch: u64,
    reason: String,
}

impl AppTransition {
    fn with_before(mut self, before: &AppSnapshot) -> Self {
        self.before = before.clone();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn happy_path_requires_auth_tenant_sync_and_confirmation() {
        let mut machine = AppLifecycleMachine::default();
        let launch = machine.dispatch(&AppEvent::LaunchRequested);
        let launch_token = launch.effect.expect("bootstrap effect").operation_id;
        machine.dispatch(&AppEvent::BootstrapSucceeded {
            operation_id: launch_token,
            authenticated: false,
            has_tenant: false,
            online: true,
        });
        let auth = machine.dispatch(&AppEvent::SignInRequested);
        machine.dispatch(&AppEvent::AuthenticationSucceeded {
            operation_id: auth.effect.expect("auth effect").operation_id,
            has_tenant: false,
            online: true,
        });
        assert_eq!(machine.snapshot.phase, AppPhase::SelectingTenant);
        let selected = machine.dispatch(&AppEvent::TenantSelected);
        machine.dispatch(&AppEvent::OperationSucceeded {
            operation_id: selected.effect.expect("sync effect").operation_id,
        });
        assert_eq!(machine.snapshot.phase, AppPhase::ReadyOnline);
        machine.dispatch(&AppEvent::ActionRequested {
            action_id: "rotate-api-key".to_owned(),
        });
        let execute = machine.dispatch(&AppEvent::ActionConfirmed);
        let effect = execute.effect.expect("execute effect");
        assert_eq!(effect.kind, AppEffectKind::ExecuteAction);
        assert_eq!(
            effect.authority_epoch,
            Some(machine.snapshot.authority_epoch)
        );
        machine.dispatch(&AppEvent::OperationSucceeded {
            operation_id: effect.operation_id,
        });
        assert_eq!(machine.snapshot.phase, AppPhase::ReadyOnline);
        assert!(machine.snapshot.pending_action.is_none());
    }

    #[test]
    fn offline_mode_is_read_only() {
        let mut machine = boot_ready();
        machine.dispatch(&AppEvent::ConnectivityChanged { online: false });
        assert_eq!(machine.snapshot.phase, AppPhase::ReadyOffline);
        assert!(machine.snapshot.capabilities().can_read_cached_data);
        assert!(
            !machine
                .snapshot
                .capabilities()
                .can_request_privileged_action
        );
        let rejected = machine.dispatch(&AppEvent::ActionRequested {
            action_id: "rotate-api-key".to_owned(),
        });
        assert_eq!(rejected.disposition, TransitionDisposition::Rejected);
        assert_eq!(rejected.after, rejected.before);
    }

    #[test]
    fn sign_out_fences_stale_completions_and_revokes_authority_first() {
        let mut machine = AppLifecycleMachine::default();
        let launch = machine.dispatch(&AppEvent::LaunchRequested);
        machine.dispatch(&AppEvent::BootstrapSucceeded {
            operation_id: launch.effect.expect("bootstrap effect").operation_id,
            authenticated: false,
            has_tenant: false,
            online: true,
        });
        let auth = machine.dispatch(&AppEvent::SignInRequested);
        let auth_token = auth.effect.expect("auth effect").operation_id;
        let sign_out = machine.dispatch(&AppEvent::SessionRevoked);
        assert_eq!(machine.snapshot.phase, AppPhase::SigningOut);
        assert!(!machine.snapshot.has_session);
        assert!(!machine.snapshot.has_tenant);
        let stale = machine.dispatch(&AppEvent::AuthenticationSucceeded {
            operation_id: auth_token,
            has_tenant: true,
            online: true,
        });
        assert_eq!(stale.disposition, TransitionDisposition::Stale);
        machine.dispatch(&AppEvent::OperationSucceeded {
            operation_id: sign_out.effect.expect("clear effect").operation_id,
        });
        assert_eq!(machine.snapshot.phase, AppPhase::SignedOut);
    }

    #[test]
    fn ambiguous_action_requires_reconciliation() {
        let mut machine = boot_ready();
        machine.dispatch(&AppEvent::ActionRequested {
            action_id: "move-leader".to_owned(),
        });
        let execute = machine.dispatch(&AppEvent::ActionConfirmed);
        let execute_token = execute.effect.expect("execute effect").operation_id;
        machine.dispatch(&AppEvent::ConnectivityChanged { online: false });
        assert_eq!(machine.snapshot.phase, AppPhase::ReconciliationRequired);
        let stale = machine.dispatch(&AppEvent::OperationSucceeded {
            operation_id: execute_token,
        });
        assert_eq!(stale.disposition, TransitionDisposition::Stale);
        let reconcile = machine.dispatch(&AppEvent::ConnectivityChanged { online: true });
        assert_eq!(
            reconcile.effect.as_ref().map(|effect| effect.kind),
            Some(AppEffectKind::ReconcileAction)
        );
        machine.dispatch(&AppEvent::OperationSucceeded {
            operation_id: reconcile.effect.expect("reconcile effect").operation_id,
        });
        assert_eq!(machine.snapshot.phase, AppPhase::ReadyOnline);
        assert!(machine.snapshot.pending_action.is_none());
    }

    #[test]
    fn corrupt_snapshot_fails_closed() {
        let corrupt = AppSnapshot {
            phase: AppPhase::ReadyOnline,
            generation: 4,
            authority_epoch: 2,
            has_session: false,
            has_tenant: true,
            online: true,
            active_operation: None,
            active_operation_id: None,
            pending_action: None,
            failure: None,
        };
        assert!(corrupt.validate().is_err());
        let transition = AppLifecycleMachine::transition(
            &corrupt,
            &AppEvent::ActionRequested {
                action_id: "unsafe".to_owned(),
            },
        );
        assert_eq!(transition.disposition, TransitionDisposition::FailedClosed);
        assert_eq!(transition.after.phase, AppPhase::Failed);
        assert!(!transition.after.has_session);
        assert!(!transition.after.has_tenant);
        assert!(transition.after.pending_action.is_none());
        assert!(transition.after.validate().is_ok());
    }

    #[test]
    fn bounded_graph_is_total_deterministic_and_invariant_preserving() {
        let initial = AppSnapshot::default();
        let mut visited = HashSet::from([initial.clone()]);
        let mut frontier = HashSet::from([initial]);
        let mut phases = HashSet::from([AppPhase::Cold]);
        let mut dispositions = HashSet::new();

        for _depth in 0..9 {
            let mut next = HashSet::new();
            for snapshot in frontier {
                for event in bounded_events(&snapshot) {
                    let first = AppLifecycleMachine::transition(&snapshot, &event);
                    let second = AppLifecycleMachine::transition(&snapshot, &event);
                    assert_eq!(first, second, "transition must be deterministic");
                    assert!(
                        first.after.validate().is_ok(),
                        "{:?} + {:?} produced {:?}",
                        snapshot.phase,
                        event,
                        first.after.validate()
                    );
                    assert!(first.after.generation >= snapshot.generation);
                    if matches!(
                        first.disposition,
                        TransitionDisposition::Rejected | TransitionDisposition::Stale
                    ) {
                        assert_eq!(first.after, snapshot);
                        assert!(first.effect.is_none());
                    }
                    if first
                        .effect
                        .as_ref()
                        .is_some_and(|effect| effect.kind == AppEffectKind::ExecuteAction)
                    {
                        assert_eq!(first.after.phase, AppPhase::ExecutingAction);
                        assert!(first.after.has_session);
                        assert!(first.after.has_tenant);
                        assert!(first.after.online);
                        assert_eq!(
                            first
                                .effect
                                .as_ref()
                                .and_then(|effect| effect.authority_epoch),
                            Some(first.after.authority_epoch)
                        );
                    }
                    phases.insert(first.after.phase);
                    dispositions.insert(first.disposition);
                    if visited.insert(first.after.clone()) {
                        next.insert(first.after);
                    }
                }
            }
            frontier = next;
            if frontier.is_empty() {
                break;
            }
        }

        assert_eq!(phases.len(), 15, "every declared phase must be reachable");
        assert_eq!(
            dispositions,
            HashSet::from([
                TransitionDisposition::Applied,
                TransitionDisposition::Rejected,
                TransitionDisposition::Stale,
                TransitionDisposition::FailedClosed,
            ])
        );
        assert!(visited.len() < 5_000);
    }

    fn boot_ready() -> AppLifecycleMachine {
        let mut machine = AppLifecycleMachine::default();
        let launch = machine.dispatch(&AppEvent::LaunchRequested);
        let bootstrap = machine.dispatch(&AppEvent::BootstrapSucceeded {
            operation_id: launch.effect.expect("bootstrap effect").operation_id,
            authenticated: true,
            has_tenant: true,
            online: true,
        });
        machine.dispatch(&AppEvent::OperationSucceeded {
            operation_id: bootstrap.effect.expect("sync effect").operation_id,
        });
        assert_eq!(machine.snapshot.phase, AppPhase::ReadyOnline);
        machine
    }

    fn bounded_events(snapshot: &AppSnapshot) -> Vec<AppEvent> {
        let active = snapshot.active_operation_id.unwrap_or(snapshot.generation);
        vec![
            AppEvent::LaunchRequested,
            AppEvent::BootstrapSucceeded {
                operation_id: active,
                authenticated: false,
                has_tenant: false,
                online: true,
            },
            AppEvent::BootstrapSucceeded {
                operation_id: active,
                authenticated: true,
                has_tenant: false,
                online: true,
            },
            AppEvent::BootstrapSucceeded {
                operation_id: active,
                authenticated: true,
                has_tenant: true,
                online: true,
            },
            AppEvent::BootstrapSucceeded {
                operation_id: active,
                authenticated: true,
                has_tenant: true,
                online: false,
            },
            AppEvent::SignInRequested,
            AppEvent::AuthenticationSucceeded {
                operation_id: active,
                has_tenant: false,
                online: true,
            },
            AppEvent::AuthenticationSucceeded {
                operation_id: active,
                has_tenant: true,
                online: true,
            },
            AppEvent::TenantSelected,
            AppEvent::ConnectivityChanged { online: false },
            AppEvent::ConnectivityChanged { online: true },
            AppEvent::SyncRequested,
            AppEvent::ActionRequested {
                action_id: "bounded-action".to_owned(),
            },
            AppEvent::ActionRequested {
                action_id: String::new(),
            },
            AppEvent::ActionConfirmed,
            AppEvent::ActionCancelled,
            AppEvent::OperationSucceeded {
                operation_id: active,
            },
            AppEvent::OperationSucceeded {
                operation_id: snapshot.generation.saturating_add(7),
            },
            AppEvent::OperationFailed {
                operation_id: active,
                reason: "bounded failure".to_owned(),
                retryable: false,
                ambiguous: false,
            },
            AppEvent::OperationFailed {
                operation_id: active,
                reason: "ambiguous bounded failure".to_owned(),
                retryable: true,
                ambiguous: true,
            },
            AppEvent::SignOutRequested,
            AppEvent::SessionRevoked,
            AppEvent::RecoveryRequested,
            AppEvent::RecoverySucceeded {
                operation_id: active,
                authenticated: false,
                has_tenant: false,
                online: true,
            },
            AppEvent::RecoverySucceeded {
                operation_id: active,
                authenticated: true,
                has_tenant: true,
                online: true,
            },
        ]
    }
}
