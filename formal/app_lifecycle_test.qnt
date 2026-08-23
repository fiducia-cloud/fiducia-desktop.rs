module app_lifecycle_test {
  import app_lifecycle.* from "./app_lifecycle"

  run authenticatedActionRequiresConfirmationTest =
    init
      .then(launch)
      .then(bootstrap_signed_out(true))
      .then(sign_in)
      .then(auth_select_tenant(true))
      .then(select_tenant_online)
      .then(synchronization_succeeded)
      .then(request_action)
      .then(confirm_action)
      .expect(and {
        s.phase == EXECUTING_ACTION,
        s.effect == EFFECT_EXECUTE_ACTION,
        s.effect_authority == s.authority_epoch,
        app_lifecycle_safety,
      })

  run actionCannotSkipConfirmationTest =
    init
      .then(launch)
      .then(bootstrap_ready_online)
      .then(synchronization_succeeded)
      .then(confirm_action)
      .fail()

  run offlineModeIsReadOnlyTest =
    init
      .then(launch)
      .then(bootstrap_ready_offline)
      .then(request_action)
      .fail()

  run staleCompletionCannotRestoreRevokedAuthorityTest =
    init
      .then(launch)
      .then(bootstrap_signed_out(true))
      .then(sign_in)
      .then(sign_out)
      .then(rejected_or_stale)
      .expect(and {
        s.phase == SIGNING_OUT,
        not(s.has_session),
        not(s.has_tenant),
        s.effect == EFFECT_NONE,
        app_lifecycle_safety,
      })

  run ambiguousActionMustReconcileTest =
    init
      .then(launch)
      .then(bootstrap_ready_online)
      .then(synchronization_succeeded)
      .then(request_action)
      .then(confirm_action)
      .then(disconnect_during_action)
      .then(reconnect_reconciliation)
      .expect(and {
        s.phase == RECONCILING_ACTION,
        s.pending_action,
        s.effect == EFFECT_RECONCILE_ACTION,
        s.effect_authority == s.authority_epoch,
        app_lifecycle_safety,
      })

  run signOutRevokesAuthorityBeforeCleanupTest =
    init
      .then(launch)
      .then(bootstrap_ready_online)
      .then(synchronization_succeeded)
      .then(sign_out)
      .expect(and {
        s.phase == SIGNING_OUT,
        not(s.has_session),
        not(s.has_tenant),
        not(s.pending_action),
        s.effect == EFFECT_CLEAR_SESSION,
        app_lifecycle_safety,
      })

  run controlledFailureCanRecoverTest =
    init
      .then(launch)
      .then(fail_closed)
      .then(recover)
      .expect(and {
        s.phase == RECOVERING,
        s.effect == EFFECT_RECOVER_SESSION,
        not(s.has_session),
        app_lifecycle_safety,
      })
}
