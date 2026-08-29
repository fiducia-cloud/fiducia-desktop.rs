module deep_link_admission_test {
  import deep_link_admission.* from "./deep_link_admission"

  run acceptedActionBecomesPendingTest =
    init
      .then(begin_rotate_api_key)
      .then(resolve_current)
      .expect(and {
        s.phase == PENDING,
        s.last_action == ACTION_ROTATE_API_KEY,
        deep_link_admission_safety,
      })

  run newerCaptureFencesStaleCompletionTest =
    init
      .then(begin_rotate_api_key)
      .then(begin_review_reconciliation)
      .then(stale_completion)
      .then(resolve_current)
      .expect(and {
        s.phase == PENDING,
        s.generation == 2,
        s.last_action == ACTION_REVIEW_RECONCILIATION,
        deep_link_admission_safety,
      })

  run rejectedInputPreservesLastAcceptedIntentTest =
    init
      .then(begin_rotate_api_key)
      .then(resolve_current)
      .then(consume_current)
      .then(begin_unknown_action)
      .then(resolve_current)
      .expect(and {
        s.phase == IDLE,
        s.last_action == ACTION_ROTATE_API_KEY,
        deep_link_admission_safety,
      })

  run preflightRejectionIsExactStutterTest =
    init
      .then(preflight_rejected)
      .expect(and {
        s.phase == IDLE,
        s.generation == 0,
        s.candidate == INPUT_NONE,
        s.last_action == ACTION_NONE,
        deep_link_admission_safety,
      })

  run consumptionRequiresPendingIntentTest =
    init
      .then(consume_current)
      .fail()

  run generationExhaustionRevokesIntentTest =
    init
      .then(begin_rotate_api_key)
      .then(resolve_current)
      .then(consume_current)
      .then(begin_rotate_api_key)
      .then(resolve_current)
      .then(consume_current)
      .then(begin_rotate_api_key)
      .then(resolve_current)
      .then(consume_current)
      .then(begin_rotate_api_key)
      .then(resolve_current)
      .then(consume_current)
      .then(begin_rotate_api_key)
      .then(resolve_current)
      .then(consume_current)
      .then(begin_rotate_api_key)
      .then(resolve_current)
      .then(exhaustion_fail_closed)
      .expect(and {
        s.phase == IDLE,
        s.generation == MAX_GENERATION,
        s.candidate == INPUT_NONE,
        s.last_action == ACTION_NONE,
        deep_link_admission_safety,
      })
}
