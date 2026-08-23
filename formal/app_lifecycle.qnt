// Bounded, platform-independent lifecycle for Fiducia mobile and desktop apps.
// Phase, operation, and effect constants intentionally mirror the Dart and
// Rust implementations. Async completions are accepted only for the current
// generation; stale completions stutter and authority is revoked before
// sign-out cleanup begins.
module app_lifecycle {
  type State = {
    phase: int,
    generation: int,
    authority_epoch: int,
    has_session: bool,
    has_tenant: bool,
    online: bool,
    operation: int,
    active_token: int,
    pending_action: bool,
    pending_authority: int,
    failure: bool,
    effect: int,
    effect_token: int,
    effect_authority: int,
  }

  pure val COLD = 0
  pure val BOOTSTRAPPING = 1
  pure val SIGNED_OUT = 2
  pure val AUTHENTICATING = 3
  pure val SELECTING_TENANT = 4
  pure val SYNCHRONIZING = 5
  pure val READY_ONLINE = 6
  pure val READY_OFFLINE = 7
  pure val CONFIRMING_ACTION = 8
  pure val EXECUTING_ACTION = 9
  pure val RECONCILIATION_REQUIRED = 10
  pure val RECONCILING_ACTION = 11
  pure val RECOVERING = 12
  pure val SIGNING_OUT = 13
  pure val FAILED = 14

  pure val OP_NONE = 0
  pure val OP_BOOTSTRAP = 1
  pure val OP_AUTHENTICATE = 2
  pure val OP_SYNCHRONIZE = 3
  pure val OP_EXECUTE_ACTION = 4
  pure val OP_RECONCILE_ACTION = 5
  pure val OP_RECOVER = 6
  pure val OP_SIGN_OUT = 7

  pure val EFFECT_NONE = 0
  pure val EFFECT_LOAD_SESSION = 1
  pure val EFFECT_AUTHENTICATE = 2
  pure val EFFECT_SYNCHRONIZE = 3
  pure val EFFECT_EXECUTE_ACTION = 4
  pure val EFFECT_RECONCILE_ACTION = 5
  pure val EFFECT_RECOVER_SESSION = 6
  pure val EFFECT_CLEAR_SESSION = 7

  pure val MAX_GENERATION = 8
  pure val MAX_AUTHORITY = 4
  pure val BOOLS: Set[bool] = Set(true, false)

  var s: State

  pure def valid_phase(phase: int): bool =
    phase >= COLD and phase <= FAILED

  pure def valid_operation(operation: int): bool =
    operation >= OP_NONE and operation <= OP_SIGN_OUT

  pure def valid_effect(effect: int): bool =
    effect >= EFFECT_NONE and effect <= EFFECT_CLEAR_SESSION

  pure def busy_phase(phase: int): bool = Set(
    BOOTSTRAPPING,
    AUTHENTICATING,
    SYNCHRONIZING,
    EXECUTING_ACTION,
    RECONCILING_ACTION,
    RECOVERING,
    SIGNING_OUT
  ).contains(phase)

  pure def session_phase(phase: int): bool = Set(
    SELECTING_TENANT,
    SYNCHRONIZING,
    READY_ONLINE,
    READY_OFFLINE,
    CONFIRMING_ACTION,
    EXECUTING_ACTION,
    RECONCILIATION_REQUIRED,
    RECONCILING_ACTION
  ).contains(phase)

  pure def tenant_phase(phase: int): bool = Set(
    SYNCHRONIZING,
    READY_ONLINE,
    READY_OFFLINE,
    CONFIRMING_ACTION,
    EXECUTING_ACTION,
    RECONCILIATION_REQUIRED,
    RECONCILING_ACTION
  ).contains(phase)

  pure def online_phase(phase: int): bool = Set(
    SYNCHRONIZING,
    READY_ONLINE,
    CONFIRMING_ACTION,
    EXECUTING_ACTION,
    RECONCILING_ACTION
  ).contains(phase)

  pure def pending_phase(phase: int): bool = Set(
    CONFIRMING_ACTION,
    EXECUTING_ACTION,
    RECONCILIATION_REQUIRED,
    RECONCILING_ACTION
  ).contains(phase)

  action init: bool = {
    s' = {
      phase: COLD,
      generation: 0,
      authority_epoch: 0,
      has_session: false,
      has_tenant: false,
      online: false,
      operation: OP_NONE,
      active_token: 0,
      pending_action: false,
      pending_authority: 0,
      failure: false,
      effect: EFFECT_NONE,
      effect_token: 0,
      effect_authority: 0,
    }
  }

  action start_operation(
    next_phase: int,
    next_operation: int,
    next_effect: int,
    has_session: bool,
    has_tenant: bool,
    online: bool,
    pending_action: bool,
    authority_epoch: int,
    effect_authority: int,
  ): bool = all {
    s.generation < MAX_GENERATION,
    s' = {
      phase: next_phase,
      generation: s.generation + 1,
      authority_epoch: authority_epoch,
      has_session: has_session,
      has_tenant: has_tenant,
      online: online,
      operation: next_operation,
      active_token: s.generation + 1,
      pending_action: pending_action,
      pending_authority: if (pending_action) authority_epoch else 0,
      failure: false,
      effect: next_effect,
      effect_token: s.generation + 1,
      effect_authority: effect_authority,
    },
  }

  action settle(
    next_phase: int,
    has_session: bool,
    has_tenant: bool,
    online: bool,
    pending_action: bool,
    authority_epoch: int,
    failure: bool,
  ): bool = {
    s' = {
      ...s,
      phase: next_phase,
      authority_epoch: authority_epoch,
      has_session: has_session,
      has_tenant: has_tenant,
      online: online,
      operation: OP_NONE,
      active_token: 0,
      pending_action: pending_action,
      pending_authority: if (pending_action) authority_epoch else 0,
      failure: failure,
      effect: EFFECT_NONE,
      effect_token: 0,
      effect_authority: 0,
    }
  }

  action fence_to(
    next_phase: int,
    has_session: bool,
    has_tenant: bool,
    online: bool,
    pending_action: bool,
  ): bool = all {
    s.generation < MAX_GENERATION,
    s' = {
      ...s,
      phase: next_phase,
      generation: s.generation + 1,
      has_session: has_session,
      has_tenant: has_tenant,
      online: online,
      operation: OP_NONE,
      active_token: 0,
      pending_action: pending_action,
      pending_authority: if (pending_action) s.authority_epoch else 0,
      failure: false,
      effect: EFFECT_NONE,
      effect_token: 0,
      effect_authority: 0,
    },
  }

  action launch: bool = all {
    s.phase == COLD,
    start_operation(
      BOOTSTRAPPING, OP_BOOTSTRAP, EFFECT_LOAD_SESSION,
      false, false, false, false, s.authority_epoch, 0
    ),
  }

  action bootstrap_signed_out(online: bool): bool = all {
    s.phase == BOOTSTRAPPING,
    s.operation == OP_BOOTSTRAP,
    BOOLS.contains(online),
    settle(SIGNED_OUT, false, false, online, false, s.authority_epoch, false),
  }

  action bootstrap_select_tenant(online: bool): bool = all {
    s.phase == BOOTSTRAPPING,
    s.operation == OP_BOOTSTRAP,
    s.authority_epoch < MAX_AUTHORITY,
    BOOLS.contains(online),
    settle(
      SELECTING_TENANT, true, false, online, false,
      s.authority_epoch + 1, false
    ),
  }

  action bootstrap_ready_offline: bool = all {
    s.phase == BOOTSTRAPPING,
    s.operation == OP_BOOTSTRAP,
    s.authority_epoch < MAX_AUTHORITY,
    settle(
      READY_OFFLINE, true, true, false, false,
      s.authority_epoch + 1, false
    ),
  }

  action bootstrap_ready_online: bool = all {
    s.phase == BOOTSTRAPPING,
    s.operation == OP_BOOTSTRAP,
    s.authority_epoch < MAX_AUTHORITY,
    start_operation(
      SYNCHRONIZING, OP_SYNCHRONIZE, EFFECT_SYNCHRONIZE,
      true, true, true, false, s.authority_epoch + 1, 0
    ),
  }

  action sign_in: bool = all {
    s.phase == SIGNED_OUT,
    start_operation(
      AUTHENTICATING, OP_AUTHENTICATE, EFFECT_AUTHENTICATE,
      false, false, s.online, false, s.authority_epoch, 0
    ),
  }

  action auth_select_tenant(online: bool): bool = all {
    s.phase == AUTHENTICATING,
    s.operation == OP_AUTHENTICATE,
    s.authority_epoch < MAX_AUTHORITY,
    BOOLS.contains(online),
    settle(
      SELECTING_TENANT, true, false, online, false,
      s.authority_epoch + 1, false
    ),
  }

  action auth_ready_offline: bool = all {
    s.phase == AUTHENTICATING,
    s.operation == OP_AUTHENTICATE,
    s.authority_epoch < MAX_AUTHORITY,
    settle(
      READY_OFFLINE, true, true, false, false,
      s.authority_epoch + 1, false
    ),
  }

  action auth_ready_online: bool = all {
    s.phase == AUTHENTICATING,
    s.operation == OP_AUTHENTICATE,
    s.authority_epoch < MAX_AUTHORITY,
    start_operation(
      SYNCHRONIZING, OP_SYNCHRONIZE, EFFECT_SYNCHRONIZE,
      true, true, true, false, s.authority_epoch + 1, 0
    ),
  }

  action auth_rejected: bool = all {
    s.phase == AUTHENTICATING,
    settle(SIGNED_OUT, false, false, s.online, false, s.authority_epoch, false),
  }

  action select_tenant_offline: bool = all {
    s.phase == SELECTING_TENANT,
    not(s.online),
    s.authority_epoch < MAX_AUTHORITY,
    settle(
      READY_OFFLINE, true, true, false, false,
      s.authority_epoch + 1, false
    ),
  }

  action select_tenant_online: bool = all {
    s.phase == SELECTING_TENANT,
    s.online,
    s.authority_epoch < MAX_AUTHORITY,
    start_operation(
      SYNCHRONIZING, OP_SYNCHRONIZE, EFFECT_SYNCHRONIZE,
      true, true, true, false, s.authority_epoch + 1, 0
    ),
  }

  action synchronization_succeeded: bool = all {
    s.phase == SYNCHRONIZING,
    s.operation == OP_SYNCHRONIZE,
    settle(READY_ONLINE, true, true, true, false, s.authority_epoch, false),
  }

  action synchronization_failed_offline: bool = all {
    s.phase == SYNCHRONIZING,
    s.operation == OP_SYNCHRONIZE,
    settle(READY_OFFLINE, true, true, false, false, s.authority_epoch, false),
  }

  action reconnect_ready: bool = all {
    s.phase == READY_OFFLINE,
    start_operation(
      SYNCHRONIZING, OP_SYNCHRONIZE, EFFECT_SYNCHRONIZE,
      true, true, true, false, s.authority_epoch, 0
    ),
  }

  action go_offline: bool = all {
    s.phase == READY_ONLINE,
    settle(READY_OFFLINE, true, true, false, false, s.authority_epoch, false),
  }

  action request_action: bool = all {
    s.phase == READY_ONLINE,
    settle(
      CONFIRMING_ACTION, true, true, true, true,
      s.authority_epoch, false
    ),
  }

  action cancel_action: bool = all {
    s.phase == CONFIRMING_ACTION,
    settle(READY_ONLINE, true, true, true, false, s.authority_epoch, false),
  }

  action confirm_action: bool = all {
    s.phase == CONFIRMING_ACTION,
    start_operation(
      EXECUTING_ACTION, OP_EXECUTE_ACTION, EFFECT_EXECUTE_ACTION,
      true, true, true, true, s.authority_epoch, s.authority_epoch
    ),
  }

  action action_succeeded: bool = all {
    s.phase == EXECUTING_ACTION,
    s.operation == OP_EXECUTE_ACTION,
    settle(READY_ONLINE, true, true, true, false, s.authority_epoch, false),
  }

  action action_failed_definitively: bool = all {
    s.phase == EXECUTING_ACTION,
    s.operation == OP_EXECUTE_ACTION,
    settle(READY_ONLINE, true, true, true, false, s.authority_epoch, false),
  }

  action disconnect_during_confirm: bool = all {
    s.phase == CONFIRMING_ACTION,
    settle(READY_OFFLINE, true, true, false, false, s.authority_epoch, false),
  }

  action disconnect_during_sync: bool = all {
    s.phase == SYNCHRONIZING,
    fence_to(READY_OFFLINE, true, true, false, false),
  }

  action disconnect_during_action: bool = all {
    Set(EXECUTING_ACTION, RECONCILING_ACTION).contains(s.phase),
    fence_to(RECONCILIATION_REQUIRED, true, true, false, true),
  }

  action ambiguous_action_result: bool = all {
    s.phase == EXECUTING_ACTION,
    s.operation == OP_EXECUTE_ACTION,
    start_operation(
      RECONCILING_ACTION, OP_RECONCILE_ACTION, EFFECT_RECONCILE_ACTION,
      true, true, true, true, s.authority_epoch, s.authority_epoch
    ),
  }

  action reconnect_reconciliation: bool = all {
    s.phase == RECONCILIATION_REQUIRED,
    not(s.online),
    start_operation(
      RECONCILING_ACTION, OP_RECONCILE_ACTION, EFFECT_RECONCILE_ACTION,
      true, true, true, true, s.authority_epoch, s.authority_epoch
    ),
  }

  action reconciliation_succeeded: bool = all {
    s.phase == RECONCILING_ACTION,
    s.operation == OP_RECONCILE_ACTION,
    settle(READY_ONLINE, true, true, true, false, s.authority_epoch, false),
  }

  action sign_out: bool = all {
    s.phase != COLD,
    s.phase != SIGNED_OUT,
    s.phase != SIGNING_OUT,
    s.authority_epoch < MAX_AUTHORITY,
    start_operation(
      SIGNING_OUT, OP_SIGN_OUT, EFFECT_CLEAR_SESSION,
      false, false, s.online, false, s.authority_epoch + 1, 0
    ),
  }

  action sign_out_finished: bool = all {
    s.phase == SIGNING_OUT,
    s.operation == OP_SIGN_OUT,
    settle(SIGNED_OUT, false, false, s.online, false, s.authority_epoch, false),
  }

  action fail_closed: bool = all {
    s.phase != FAILED,
    s.generation < MAX_GENERATION,
    s.authority_epoch < MAX_AUTHORITY,
    s' = {
      ...s,
      phase: FAILED,
      generation: s.generation + 1,
      authority_epoch: s.authority_epoch + 1,
      has_session: false,
      has_tenant: false,
      online: false,
      operation: OP_NONE,
      active_token: 0,
      pending_action: false,
      pending_authority: 0,
      failure: true,
      effect: EFFECT_NONE,
      effect_token: 0,
      effect_authority: 0,
    },
  }

  action recover: bool = all {
    s.phase == FAILED,
    start_operation(
      RECOVERING, OP_RECOVER, EFFECT_RECOVER_SESSION,
      false, false, false, false, s.authority_epoch, 0
    ),
  }

  action recovery_signed_out(online: bool): bool = all {
    s.phase == RECOVERING,
    BOOLS.contains(online),
    settle(SIGNED_OUT, false, false, online, false, s.authority_epoch, false),
  }

  action recovery_ready_online: bool = all {
    s.phase == RECOVERING,
    s.authority_epoch < MAX_AUTHORITY,
    start_operation(
      SYNCHRONIZING, OP_SYNCHRONIZE, EFFECT_SYNCHRONIZE,
      true, true, true, false, s.authority_epoch + 1, 0
    ),
  }

  // Models either an unsupported request or a completion carrying any token
  // other than active_token. Both implementations return the state unchanged.
  action rejected_or_stale: bool = {
    s' = { ...s, effect: EFFECT_NONE, effect_token: 0, effect_authority: 0 }
  }

  action step: bool = {
    nondet online = BOOLS.oneOf()
    any {
      launch,
      bootstrap_signed_out(online),
      bootstrap_select_tenant(online),
      bootstrap_ready_offline,
      bootstrap_ready_online,
      sign_in,
      auth_select_tenant(online),
      auth_ready_offline,
      auth_ready_online,
      auth_rejected,
      select_tenant_offline,
      select_tenant_online,
      synchronization_succeeded,
      synchronization_failed_offline,
      reconnect_ready,
      go_offline,
      request_action,
      cancel_action,
      confirm_action,
      action_succeeded,
      action_failed_definitively,
      disconnect_during_confirm,
      disconnect_during_sync,
      disconnect_during_action,
      ambiguous_action_result,
      reconnect_reconciliation,
      reconciliation_succeeded,
      sign_out,
      sign_out_finished,
      fail_closed,
      recover,
      recovery_signed_out(online),
      recovery_ready_online,
      rejected_or_stale,
    }
  }

  val domain_bounds = and {
    valid_phase(s.phase),
    valid_operation(s.operation),
    valid_effect(s.effect),
    s.generation >= 0,
    s.generation <= MAX_GENERATION,
    s.authority_epoch >= 0,
    s.authority_epoch <= MAX_AUTHORITY,
    s.active_token >= 0,
    s.active_token <= MAX_GENERATION,
    s.pending_authority >= 0,
    s.pending_authority <= MAX_AUTHORITY,
    s.effect_token >= 0,
    s.effect_token <= MAX_GENERATION,
    s.effect_authority >= 0,
    s.effect_authority <= MAX_AUTHORITY,
  }

  val operation_matches_phase = and {
    (s.phase == BOOTSTRAPPING) == (s.operation == OP_BOOTSTRAP),
    (s.phase == AUTHENTICATING) == (s.operation == OP_AUTHENTICATE),
    (s.phase == SYNCHRONIZING) == (s.operation == OP_SYNCHRONIZE),
    (s.phase == EXECUTING_ACTION) == (s.operation == OP_EXECUTE_ACTION),
    (s.phase == RECONCILING_ACTION) == (s.operation == OP_RECONCILE_ACTION),
    (s.phase == RECOVERING) == (s.operation == OP_RECOVER),
    (s.phase == SIGNING_OUT) == (s.operation == OP_SIGN_OUT),
    busy_phase(s.phase) == (s.operation != OP_NONE),
    (s.operation == OP_NONE) == (s.active_token == 0),
    s.active_token == 0 or s.active_token == s.generation,
  }

  val authority_matches_phase = and {
    s.has_tenant.implies(s.has_session),
    s.has_session.implies(s.authority_epoch > 0),
    s.has_session == session_phase(s.phase),
    s.has_tenant == tenant_phase(s.phase),
    online_phase(s.phase).implies(s.online),
    (s.phase == READY_OFFLINE).implies(not(s.online)),
    s.pending_action == pending_phase(s.phase),
    s.pending_action.implies(s.pending_authority == s.authority_epoch),
    not(s.pending_action) == (s.pending_authority == 0),
    s.failure == (s.phase == FAILED),
  }

  val effect_is_authorized = and {
    (s.effect == EFFECT_NONE) == (s.effect_token == 0),
    (s.effect == EFFECT_NONE).implies(s.effect_authority == 0),
    (s.effect != EFFECT_NONE).implies(and {
      busy_phase(s.phase),
      s.effect_token == s.active_token,
      s.effect_token == s.generation,
    }),
    (s.effect == EFFECT_LOAD_SESSION).implies(and {
      s.phase == BOOTSTRAPPING,
      s.operation == OP_BOOTSTRAP,
    }),
    (s.effect == EFFECT_AUTHENTICATE).implies(and {
      s.phase == AUTHENTICATING,
      s.operation == OP_AUTHENTICATE,
    }),
    (s.effect == EFFECT_SYNCHRONIZE).implies(and {
      s.phase == SYNCHRONIZING,
      s.operation == OP_SYNCHRONIZE,
    }),
    (s.effect == EFFECT_RECOVER_SESSION).implies(and {
      s.phase == RECOVERING,
      s.operation == OP_RECOVER,
    }),
    (s.effect == EFFECT_EXECUTE_ACTION).implies(and {
      s.phase == EXECUTING_ACTION,
      s.has_session,
      s.has_tenant,
      s.online,
      s.pending_action,
      s.effect_authority == s.authority_epoch,
    }),
    (s.effect == EFFECT_RECONCILE_ACTION).implies(and {
      s.phase == RECONCILING_ACTION,
      s.has_session,
      s.has_tenant,
      s.online,
      s.pending_action,
      s.effect_authority == s.authority_epoch,
    }),
    (s.effect == EFFECT_CLEAR_SESSION).implies(and {
      s.phase == SIGNING_OUT,
      not(s.has_session),
      not(s.has_tenant),
      not(s.pending_action),
    }),
  }

  val app_lifecycle_safety = and {
    domain_bounds,
    operation_matches_phase,
    authority_matches_phase,
    effect_is_authorized,
  }

  val cold_reached = s.phase == COLD
  val bootstrapping_reached = s.phase == BOOTSTRAPPING
  val signed_out_reached = s.phase == SIGNED_OUT
  val authenticating_reached = s.phase == AUTHENTICATING
  val selecting_tenant_reached = s.phase == SELECTING_TENANT
  val synchronizing_reached = s.phase == SYNCHRONIZING
  val ready_online_reached = s.phase == READY_ONLINE
  val ready_offline_reached = s.phase == READY_OFFLINE
  val confirming_action_reached = s.phase == CONFIRMING_ACTION
  val executing_action_reached = s.phase == EXECUTING_ACTION
  val reconciliation_required_reached = s.phase == RECONCILIATION_REQUIRED
  val reconciling_action_reached = s.phase == RECONCILING_ACTION
  val recovering_reached = s.phase == RECOVERING
  val signing_out_reached = s.phase == SIGNING_OUT
  val failed_reached = s.phase == FAILED
}
