//! Formal deep-link admission refinement.
//!
//! The repository ORES policy requires explicit return sites at public and
//! state-transition boundaries. Keep that readability rule without weakening
//! the remainder of the crate's deny-by-default Clippy configuration.
#![allow(clippy::needless_return)]

use url::Url;

use crate::app_lifecycle::{
    AppEvent, AppLifecycleMachine, AppTransition, MAX_PORTABLE_COUNTER, TransitionDisposition,
};

pub const DEEP_LINK_CONTRACT_VERSION: u8 = 1;
pub const MAXIMUM_DEEP_LINK_BYTES: usize = 2048;
pub const ALLOWED_DEEP_LINK_ACTIONS: [&str; 2] = ["rotate-api-key", "review-reconciliation"];

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeepLinkPhase {
    Idle,
    Resolving,
    Pending,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeepLinkDisposition {
    Began,
    Accepted,
    Rejected,
    Stale,
    Consumed,
    FailedClosed,
}

impl DeepLinkDisposition {
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        return match self {
            Self::Began => "began",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Stale => "stale",
            Self::Consumed => "consumed",
            Self::FailedClosed => "failedClosed",
        };
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeepLinkReason {
    Resolving,
    Accepted,
    Consumed,
    Empty,
    TooLong,
    Malformed,
    InsecureScheme,
    UnexpectedOrigin,
    CredentialsForbidden,
    UnexpectedPort,
    UnexpectedPath,
    FragmentForbidden,
    UnexpectedParameter,
    DuplicateParameter,
    MissingAction,
    UnknownAction,
    NonCanonical,
    NotPending,
    StaleGeneration,
    InvalidSnapshot,
    LifecycleRejected,
}

impl DeepLinkReason {
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Resolving => "resolving",
            Self::Accepted => "accepted",
            Self::Consumed => "consumed",
            Self::Empty => "empty",
            Self::TooLong => "too_long",
            Self::Malformed => "malformed",
            Self::InsecureScheme => "insecure_scheme",
            Self::UnexpectedOrigin => "unexpected_origin",
            Self::CredentialsForbidden => "credentials_forbidden",
            Self::UnexpectedPort => "unexpected_port",
            Self::UnexpectedPath => "unexpected_path",
            Self::FragmentForbidden => "fragment_forbidden",
            Self::UnexpectedParameter => "unexpected_parameter",
            Self::DuplicateParameter => "duplicate_parameter",
            Self::MissingAction => "missing_action",
            Self::UnknownAction => "unknown_action",
            Self::NonCanonical => "non_canonical",
            Self::NotPending => "not_pending",
            Self::StaleGeneration => "stale_generation",
            Self::InvalidSnapshot => "invalid_snapshot",
            Self::LifecycleRejected => "lifecycle_rejected",
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DeepLinkIntent {
    action: String,
}

impl DeepLinkIntent {
    #[must_use]
    pub fn new(action: impl Into<String>) -> Self {
        return Self {
            action: action.into(),
        };
    }

    #[must_use]
    pub fn action(&self) -> &str {
        return &self.action;
    }

    #[must_use]
    pub const fn kind(&self) -> &'static str {
        return "request_action";
    }

    #[must_use]
    pub fn canonical_url(&self) -> String {
        return format!("https://fiducia.cloud/open?action={}", self.action);
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DeepLinkSnapshot {
    phase: DeepLinkPhase,
    generation: u64,
    candidate: Option<String>,
    last_accepted: Option<DeepLinkIntent>,
}

impl Default for DeepLinkSnapshot {
    fn default() -> Self {
        return Self {
            phase: DeepLinkPhase::Idle,
            generation: 0,
            candidate: None,
            last_accepted: None,
        };
    }
}

impl DeepLinkSnapshot {
    #[must_use]
    pub fn phase(&self) -> DeepLinkPhase {
        return self.phase;
    }

    #[must_use]
    pub fn generation(&self) -> u64 {
        return self.generation;
    }

    #[must_use]
    pub fn candidate(&self) -> Option<&str> {
        return self.candidate.as_deref();
    }

    #[must_use]
    pub fn last_accepted(&self) -> Option<&DeepLinkIntent> {
        return self.last_accepted.as_ref();
    }

    #[must_use]
    pub fn can_handoff(&self) -> bool {
        return self.phase == DeepLinkPhase::Pending && self.last_accepted.is_some();
    }

    fn validate(&self) -> bool {
        if self.generation > MAX_PORTABLE_COUNTER
            || self
                .last_accepted
                .as_ref()
                .is_some_and(|intent| !ALLOWED_DEEP_LINK_ACTIONS.contains(&intent.action.as_str()))
        {
            return false;
        }
        return match self.phase {
            DeepLinkPhase::Idle => self.candidate.is_none(),
            DeepLinkPhase::Resolving => self
                .candidate
                .as_deref()
                .is_some_and(|candidate| preflight(candidate).is_none()),
            DeepLinkPhase::Pending => self.candidate.is_none() && self.last_accepted.is_some(),
        };
    }

    #[cfg(test)]
    fn corrupt_for_test() -> Self {
        return Self {
            phase: DeepLinkPhase::Pending,
            generation: 9,
            candidate: Some("not allowed".to_owned()),
            last_accepted: None,
        };
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeepLinkEffect {
    pub generation: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeepLinkTransition {
    pub before: DeepLinkSnapshot,
    pub after: DeepLinkSnapshot,
    pub disposition: DeepLinkDisposition,
    pub reason: DeepLinkReason,
    pub effect: Option<DeepLinkEffect>,
    pub intent: Option<DeepLinkIntent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeepLinkHandoff {
    pub admission: DeepLinkTransition,
    pub lifecycle: Option<AppTransition>,
}

impl DeepLinkHandoff {
    #[must_use]
    pub fn delivered(&self) -> bool {
        return self.admission.disposition == DeepLinkDisposition::Consumed
            && self.lifecycle.as_ref().is_some_and(|transition| {
                transition.disposition == TransitionDisposition::Applied
            });
    }
}

#[derive(Clone, Debug, Default)]
pub struct DeepLinkAdmissionMachine {
    snapshot: DeepLinkSnapshot,
}

impl DeepLinkAdmissionMachine {
    #[must_use]
    pub fn from_snapshot(snapshot: DeepLinkSnapshot) -> Self {
        if snapshot.validate() {
            return Self { snapshot };
        }
        return Self {
            snapshot: DeepLinkSnapshot {
                generation: closed_generation(snapshot.generation),
                ..DeepLinkSnapshot::default()
            },
        };
    }

    #[must_use]
    pub fn snapshot(&self) -> &DeepLinkSnapshot {
        return &self.snapshot;
    }

    pub fn begin(&mut self, raw: &str) -> DeepLinkTransition {
        let before = self.snapshot.clone();
        if !before.validate() {
            return self.fail_closed(before);
        }
        if before.generation >= MAX_PORTABLE_COUNTER {
            return self.fail_closed(before);
        }
        if let Some(reason) = preflight(raw) {
            return unchanged(before, DeepLinkDisposition::Rejected, reason, None);
        }

        let after = DeepLinkSnapshot {
            phase: DeepLinkPhase::Resolving,
            generation: before.generation.saturating_add(1),
            candidate: Some(raw.to_owned()),
            last_accepted: before.last_accepted.clone(),
        };
        self.snapshot = after.clone();
        return DeepLinkTransition {
            before,
            after: after.clone(),
            disposition: DeepLinkDisposition::Began,
            reason: DeepLinkReason::Resolving,
            effect: Some(DeepLinkEffect {
                generation: after.generation,
            }),
            intent: None,
        };
    }

    pub fn complete(&mut self, generation: u64) -> DeepLinkTransition {
        let before = self.snapshot.clone();
        if !before.validate() {
            return self.fail_closed(before);
        }
        if before.phase != DeepLinkPhase::Resolving || generation != before.generation {
            return unchanged(
                before,
                DeepLinkDisposition::Stale,
                DeepLinkReason::StaleGeneration,
                None,
            );
        }

        let candidate = before.candidate.as_deref().unwrap_or_default();
        match parse(candidate) {
            Ok(intent) => {
                let after = DeepLinkSnapshot {
                    phase: DeepLinkPhase::Pending,
                    generation: before.generation,
                    candidate: None,
                    last_accepted: Some(intent.clone()),
                };
                self.snapshot = after.clone();
                return DeepLinkTransition {
                    before,
                    after,
                    disposition: DeepLinkDisposition::Accepted,
                    reason: DeepLinkReason::Accepted,
                    effect: None,
                    intent: Some(intent),
                };
            }
            Err(reason) => {
                let after = DeepLinkSnapshot {
                    phase: DeepLinkPhase::Idle,
                    generation: before.generation,
                    candidate: None,
                    last_accepted: before.last_accepted.clone(),
                };
                self.snapshot = after.clone();
                return DeepLinkTransition {
                    before,
                    after,
                    disposition: DeepLinkDisposition::Rejected,
                    reason,
                    effect: None,
                    intent: None,
                };
            }
        }
    }

    pub fn consume(&mut self, generation: u64) -> DeepLinkTransition {
        let before = self.snapshot.clone();
        if !before.validate() {
            return self.fail_closed(before);
        }
        if generation != before.generation {
            return unchanged(
                before,
                DeepLinkDisposition::Stale,
                DeepLinkReason::StaleGeneration,
                None,
            );
        }
        if !before.can_handoff() {
            return unchanged(
                before,
                DeepLinkDisposition::Rejected,
                DeepLinkReason::NotPending,
                None,
            );
        }

        let intent = before.last_accepted.clone();
        let after = DeepLinkSnapshot {
            phase: DeepLinkPhase::Idle,
            generation: before.generation,
            candidate: None,
            last_accepted: before.last_accepted.clone(),
        };
        self.snapshot = after.clone();
        return DeepLinkTransition {
            before,
            after,
            disposition: DeepLinkDisposition::Consumed,
            reason: DeepLinkReason::Consumed,
            effect: None,
            intent,
        };
    }

    pub fn handoff_to(&mut self, lifecycle: &mut AppLifecycleMachine) -> DeepLinkHandoff {
        let before = self.snapshot.clone();
        let Some(intent) = before
            .last_accepted
            .clone()
            .filter(|_| before.can_handoff())
        else {
            return DeepLinkHandoff {
                admission: self.consume(before.generation),
                lifecycle: None,
            };
        };

        let lifecycle_transition = lifecycle.dispatch(&AppEvent::ActionRequested {
            action_id: intent.action.clone(),
        });
        if lifecycle_transition.disposition != TransitionDisposition::Applied {
            return DeepLinkHandoff {
                admission: unchanged(
                    before,
                    DeepLinkDisposition::Rejected,
                    DeepLinkReason::LifecycleRejected,
                    Some(intent),
                ),
                lifecycle: Some(lifecycle_transition),
            };
        }

        return DeepLinkHandoff {
            admission: self.consume(before.generation),
            lifecycle: Some(lifecycle_transition),
        };
    }

    fn fail_closed(&mut self, before: DeepLinkSnapshot) -> DeepLinkTransition {
        let after = DeepLinkSnapshot {
            generation: closed_generation(before.generation),
            ..DeepLinkSnapshot::default()
        };
        self.snapshot = after.clone();
        return DeepLinkTransition {
            before,
            after,
            disposition: DeepLinkDisposition::FailedClosed,
            reason: DeepLinkReason::InvalidSnapshot,
            effect: None,
            intent: None,
        };
    }
}

fn closed_generation(generation: u64) -> u64 {
    if generation > MAX_PORTABLE_COUNTER {
        return 0;
    }
    if generation == MAX_PORTABLE_COUNTER {
        return generation;
    }
    return generation + 1;
}

fn unchanged(
    snapshot: DeepLinkSnapshot,
    disposition: DeepLinkDisposition,
    reason: DeepLinkReason,
    intent: Option<DeepLinkIntent>,
) -> DeepLinkTransition {
    return DeepLinkTransition {
        before: snapshot.clone(),
        after: snapshot,
        disposition,
        reason,
        effect: None,
        intent,
    };
}

fn preflight(raw: &str) -> Option<DeepLinkReason> {
    if raw.is_empty() {
        return Some(DeepLinkReason::Empty);
    }
    if raw.len() > MAXIMUM_DEEP_LINK_BYTES {
        return Some(DeepLinkReason::TooLong);
    }
    if raw.trim() != raw || raw.chars().any(|character| character.is_control()) {
        return Some(DeepLinkReason::Malformed);
    }
    return None;
}

fn parse(raw: &str) -> Result<DeepLinkIntent, DeepLinkReason> {
    let parsed = Url::parse(raw).map_err(|_| DeepLinkReason::Malformed)?;
    if parsed.scheme() != "https" {
        return Err(DeepLinkReason::InsecureScheme);
    }
    if !raw.starts_with("https://") {
        return Err(DeepLinkReason::NonCanonical);
    }

    let authority = raw_authority(raw).ok_or(DeepLinkReason::Malformed)?;
    if authority.contains('@') || !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(DeepLinkReason::CredentialsForbidden);
    }
    if authority.starts_with("fiducia.cloud:") {
        return Err(DeepLinkReason::UnexpectedPort);
    }
    if authority.eq_ignore_ascii_case("fiducia.cloud") && authority != "fiducia.cloud" {
        return Err(DeepLinkReason::NonCanonical);
    }
    if authority != "fiducia.cloud" || parsed.host_str() != Some("fiducia.cloud") {
        return Err(DeepLinkReason::UnexpectedOrigin);
    }
    if parsed.port().is_some() {
        return Err(DeepLinkReason::UnexpectedPort);
    }
    if raw.contains('#') || parsed.fragment().is_some() {
        return Err(DeepLinkReason::FragmentForbidden);
    }
    if parsed.path() != "/open" {
        return Err(DeepLinkReason::UnexpectedPath);
    }
    if raw.contains('%') || raw.contains('+') {
        return Err(DeepLinkReason::NonCanonical);
    }

    let query_start = raw.find('?').ok_or(DeepLinkReason::MissingAction)?;
    let query = raw
        .get(query_start.saturating_add(1)..)
        .ok_or(DeepLinkReason::MissingAction)?;
    if query.is_empty() {
        return Err(DeepLinkReason::MissingAction);
    }

    let segments: Vec<&str> = query.split('&').collect();
    if segments.iter().any(|segment| segment.is_empty()) {
        return Err(DeepLinkReason::NonCanonical);
    }

    let mut actions = Vec::new();
    for segment in segments {
        let mut parts = segment.split('=');
        let key = parts.next().unwrap_or_default();
        let value = parts.next().ok_or(DeepLinkReason::NonCanonical)?;
        if parts.next().is_some() || key.is_empty() {
            return Err(DeepLinkReason::NonCanonical);
        }
        if key != "action" {
            return Err(DeepLinkReason::UnexpectedParameter);
        }
        actions.push(value);
    }

    if actions.len() > 1 {
        return Err(DeepLinkReason::DuplicateParameter);
    }
    let action = actions.first().copied().unwrap_or_default();
    if action.is_empty() {
        return Err(DeepLinkReason::MissingAction);
    }
    if !ALLOWED_DEEP_LINK_ACTIONS.contains(&action) {
        return Err(DeepLinkReason::UnknownAction);
    }

    let intent = DeepLinkIntent::new(action);
    if raw != intent.canonical_url() || parsed.as_str() != raw {
        return Err(DeepLinkReason::NonCanonical);
    }
    return Ok(intent);
}

fn raw_authority(raw: &str) -> Option<&str> {
    let rest = raw.strip_prefix("https://")?;
    let end = rest
        .char_indices()
        .find_map(|(index, character)| matches!(character, '/' | '?' | '#').then_some(index))
        .unwrap_or(rest.len());
    return rest.get(..end);
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_lifecycle::{AppEffect, AppPhase};
    use serde_json::{Value, json};
    use std::collections::HashSet;

    #[test]
    fn shared_json_contract_and_every_conformance_vector_are_valid() {
        let schema: Value =
            serde_json::from_str(include_str!("../contracts/deep_link.schema.json"))
                .expect("schema must be valid JSON");
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(schema["additionalProperties"], false);

        let corpus: Value =
            serde_json::from_str(include_str!("../contracts/deep_link_vectors.json"))
                .expect("vectors must be valid JSON");
        assert_eq!(corpus["contract_version"], DEEP_LINK_CONTRACT_VERSION);
        assert_eq!(corpus["maximum_input_bytes"], MAXIMUM_DEEP_LINK_BYTES);

        for vector in corpus["vectors"]
            .as_array()
            .expect("vectors must be an array")
        {
            let id = vector["id"].as_str().expect("vector id");
            let input = expanded_input(vector);
            let expected = &vector["expected"];
            let mut machine = DeepLinkAdmissionMachine::default();
            let begin = machine.begin(&input);
            let result = match begin.effect {
                Some(effect) => machine.complete(effect.generation),
                None => begin,
            };

            assert_eq!(
                result.disposition.wire_name(),
                expected["disposition"],
                "{id}"
            );
            assert_eq!(result.reason.wire_name(), expected["reason"], "{id}");
            if result.disposition == DeepLinkDisposition::Accepted {
                let intent = result.intent.expect("accepted intent");
                assert_eq!(
                    json!({
                        "version": DEEP_LINK_CONTRACT_VERSION,
                        "kind": intent.kind(),
                        "action": intent.action(),
                        "canonical_url": intent.canonical_url(),
                    }),
                    expected["intent"],
                    "{id}"
                );
            } else {
                assert!(result.intent.is_none(), "{id}");
            }
            assert!(machine.snapshot().validate(), "{id}");
        }
    }

    #[test]
    fn newer_capture_fences_stale_completion() {
        let mut machine = DeepLinkAdmissionMachine::default();
        let first = machine.begin("https://fiducia.cloud/open?action=rotate-api-key");
        let second = machine.begin("https://fiducia.cloud/open?action=review-reconciliation");
        let before_stale = machine.snapshot().clone();

        let stale = machine.complete(first.effect.expect("first resolve effect").generation);
        assert_eq!(stale.disposition, DeepLinkDisposition::Stale);
        assert_eq!(stale.after, before_stale);

        let accepted = machine.complete(second.effect.expect("second resolve effect").generation);
        assert_eq!(
            accepted.intent.expect("accepted intent").action(),
            "review-reconciliation"
        );
    }

    #[test]
    fn rejected_input_preserves_last_accepted_intent() {
        let mut machine = DeepLinkAdmissionMachine::default();
        let accepted = machine.begin("https://fiducia.cloud/open?action=rotate-api-key");
        machine.complete(accepted.effect.expect("resolve effect").generation);
        machine.consume(machine.snapshot().generation());

        let invalid = machine.begin("https://fiducia.cloud/open?action=delete-company");
        let rejected = machine.complete(invalid.effect.expect("resolve effect").generation);
        assert_eq!(rejected.reason, DeepLinkReason::UnknownAction);
        assert_eq!(machine.snapshot().phase(), DeepLinkPhase::Idle);
        assert_eq!(
            machine
                .snapshot()
                .last_accepted()
                .map(DeepLinkIntent::action),
            Some("rotate-api-key")
        );
    }

    #[test]
    fn lifecycle_authority_retains_link_until_action_is_legal() {
        let mut links = DeepLinkAdmissionMachine::default();
        let begin = links.begin("https://fiducia.cloud/open?action=rotate-api-key");
        links.complete(begin.effect.expect("resolve effect").generation);

        let mut lifecycle = AppLifecycleMachine::default();
        let rejected = links.handoff_to(&mut lifecycle);
        assert!(!rejected.delivered());
        assert_eq!(links.snapshot().phase(), DeepLinkPhase::Pending);
        assert_eq!(lifecycle.snapshot().phase(), AppPhase::Cold);

        let mut ready = boot_ready();
        let delivered = links.handoff_to(&mut ready);
        assert!(delivered.delivered());
        assert_eq!(links.snapshot().phase(), DeepLinkPhase::Idle);
        assert_eq!(ready.snapshot().phase(), AppPhase::ConfirmingAction);
        assert_eq!(
            ready.snapshot().pending_action().map(|action| action.id()),
            Some("rotate-api-key")
        );
    }

    #[test]
    fn invalid_snapshot_fails_closed() {
        let machine = DeepLinkAdmissionMachine::from_snapshot(DeepLinkSnapshot::corrupt_for_test());
        assert!(machine.snapshot().validate());
        assert_eq!(machine.snapshot().phase(), DeepLinkPhase::Idle);
        assert!(machine.snapshot().last_accepted().is_none());
        assert_eq!(machine.snapshot().generation(), 10);

        let out_of_domain = DeepLinkSnapshot {
            phase: DeepLinkPhase::Idle,
            generation: MAX_PORTABLE_COUNTER + 1,
            candidate: None,
            last_accepted: None,
        };
        let normalized = DeepLinkAdmissionMachine::from_snapshot(out_of_domain);
        assert_eq!(normalized.snapshot(), &DeepLinkSnapshot::default());
    }

    #[test]
    fn generation_exhaustion_fails_closed_without_token_reuse() {
        let mut machine = DeepLinkAdmissionMachine::from_snapshot(DeepLinkSnapshot {
            phase: DeepLinkPhase::Idle,
            generation: MAX_PORTABLE_COUNTER,
            candidate: None,
            last_accepted: Some(DeepLinkIntent::new("rotate-api-key")),
        });

        let exhausted = machine.begin("https://fiducia.cloud/open?action=review-reconciliation");
        assert_eq!(exhausted.disposition, DeepLinkDisposition::FailedClosed);
        assert_eq!(exhausted.reason, DeepLinkReason::InvalidSnapshot);
        assert_eq!(exhausted.effect, None);
        assert_eq!(exhausted.after.generation(), MAX_PORTABLE_COUNTER);
        assert_eq!(exhausted.after.last_accepted(), None);
    }

    #[test]
    fn bounded_graph_is_total_deterministic_and_invariant_preserving() {
        let initial = DeepLinkAdmissionMachine::default().snapshot().clone();
        let mut visited = HashSet::from([initial.clone()]);
        let mut frontier = HashSet::from([initial]);
        let mut phases = HashSet::new();
        let mut dispositions = HashSet::new();

        for _depth in 0..7 {
            let mut next = HashSet::new();
            for snapshot in &frontier {
                phases.insert(snapshot.phase());
                for event in bounded_events(snapshot) {
                    let first = apply(snapshot, &event);
                    let second = apply(snapshot, &event);
                    assert_eq!(first.after, second.after);
                    assert_eq!(first.disposition, second.disposition);
                    assert_eq!(first.reason, second.reason);
                    assert!(first.after.validate());
                    assert!(first.after.generation() >= snapshot.generation());
                    if first.disposition == DeepLinkDisposition::Stale {
                        assert_eq!(&first.after, snapshot);
                    }
                    phases.insert(first.after.phase());
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

        assert_eq!(
            phases,
            HashSet::from([
                DeepLinkPhase::Idle,
                DeepLinkPhase::Resolving,
                DeepLinkPhase::Pending,
            ])
        );
        for expected in [
            DeepLinkDisposition::Began,
            DeepLinkDisposition::Accepted,
            DeepLinkDisposition::Rejected,
            DeepLinkDisposition::Stale,
            DeepLinkDisposition::Consumed,
        ] {
            assert!(dispositions.contains(&expected), "{expected:?}");
        }
        assert!(visited.len() < 2_000);
    }

    fn expanded_input(vector: &Value) -> String {
        let mut input = vector["input"].as_str().expect("vector input").to_owned();
        let append = vector["append_ascii"].as_str().expect("append ascii");
        let count = vector["append_count"].as_u64().expect("append count");
        for _ in 0..count {
            input.push_str(append);
        }
        return input;
    }

    fn boot_ready() -> AppLifecycleMachine {
        let mut machine = AppLifecycleMachine::default();
        let launch = machine.dispatch(&AppEvent::LaunchRequested);
        let launch_effect: AppEffect = launch.effect.expect("bootstrap effect");
        let bootstrap = machine.dispatch(&AppEvent::BootstrapSucceeded {
            operation_id: launch_effect.operation_id,
            authenticated: true,
            has_tenant: true,
            online: true,
        });
        let sync_effect = bootstrap.effect.expect("sync effect");
        machine.dispatch(&AppEvent::OperationSucceeded {
            operation_id: sync_effect.operation_id,
        });
        return machine;
    }

    #[derive(Clone, Debug)]
    enum BoundedEvent {
        Begin(&'static str),
        Complete(u64),
        Consume(u64),
    }

    fn bounded_events(snapshot: &DeepLinkSnapshot) -> Vec<BoundedEvent> {
        return vec![
            BoundedEvent::Begin("https://fiducia.cloud/open?action=rotate-api-key"),
            BoundedEvent::Begin("https://fiducia.cloud/open?action=review-reconciliation"),
            BoundedEvent::Begin("https://fiducia.cloud/open?action=delete-company"),
            BoundedEvent::Begin(""),
            BoundedEvent::Complete(snapshot.generation()),
            BoundedEvent::Complete(snapshot.generation().saturating_add(1)),
            BoundedEvent::Consume(snapshot.generation()),
            BoundedEvent::Consume(snapshot.generation().saturating_add(1)),
        ];
    }

    fn apply(snapshot: &DeepLinkSnapshot, event: &BoundedEvent) -> DeepLinkTransition {
        let mut machine = DeepLinkAdmissionMachine::from_snapshot(snapshot.clone());
        return match event {
            BoundedEvent::Begin(raw) => machine.begin(raw),
            BoundedEvent::Complete(generation) => machine.complete(*generation),
            BoundedEvent::Consume(generation) => machine.consume(*generation),
        };
    }
}
