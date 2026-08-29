// Abstract control model for Fiducia HTTPS deep-link admission.
//
// Raw URL syntax is mapped to INPUT_* classes by the byte-identical JSON
// vectors and the Dart/Rust parser tests. This model proves the state-control
// boundary after classification: total controlled outcomes, generation fencing,
// last-accepted intent preservation, and consumption only from PENDING.
module deep_link_admission {
  type State = {
    phase: int,
    generation: int,
    candidate: int,
    last_action: int,
  }

  pure val IDLE = 0
  pure val RESOLVING = 1
  pure val PENDING = 2

  pure val ACTION_NONE = -1
  pure val ACTION_ROTATE_API_KEY = 0
  pure val ACTION_REVIEW_RECONCILIATION = 1

  pure val INPUT_ROTATE_API_KEY = 0
  pure val INPUT_REVIEW_RECONCILIATION = 1
  pure val INPUT_MALFORMED = 2
  pure val INPUT_INSECURE_SCHEME = 3
  pure val INPUT_UNEXPECTED_ORIGIN = 4
  pure val INPUT_CREDENTIALS = 5
  pure val INPUT_UNEXPECTED_PORT = 6
  pure val INPUT_UNEXPECTED_PATH = 7
  pure val INPUT_FRAGMENT = 8
  pure val INPUT_UNEXPECTED_PARAMETER = 9
  pure val INPUT_DUPLICATE_PARAMETER = 10
  pure val INPUT_MISSING_ACTION = 11
  pure val INPUT_UNKNOWN_ACTION = 12
  pure val INPUT_NON_CANONICAL = 13
  pure val INPUT_NONE = -1

  pure val MAX_GENERATION = 6
  pure val INPUTS: Set[int] = Set(
    INPUT_ROTATE_API_KEY,
    INPUT_REVIEW_RECONCILIATION,
    INPUT_MALFORMED,
    INPUT_INSECURE_SCHEME,
    INPUT_UNEXPECTED_ORIGIN,
    INPUT_CREDENTIALS,
    INPUT_UNEXPECTED_PORT,
    INPUT_UNEXPECTED_PATH,
    INPUT_FRAGMENT,
    INPUT_UNEXPECTED_PARAMETER,
    INPUT_DUPLICATE_PARAMETER,
    INPUT_MISSING_ACTION,
    INPUT_UNKNOWN_ACTION,
    INPUT_NON_CANONICAL
  )
  pure val TOKENS: Set[int] = Set(0, 1, 2, 3, 4, 5, 6, 7)

  var s: State

  pure def valid_input(input: int): bool =
    Set(
      INPUT_ROTATE_API_KEY,
      INPUT_REVIEW_RECONCILIATION
    ).contains(input)

  pure def input_action(input: int): int =
    if (input == INPUT_ROTATE_API_KEY) ACTION_ROTATE_API_KEY
    else if (input == INPUT_REVIEW_RECONCILIATION)
      ACTION_REVIEW_RECONCILIATION
    else ACTION_NONE

  action init: bool = {
    s' = {
      phase: IDLE,
      generation: 0,
      candidate: INPUT_NONE,
      last_action: ACTION_NONE,
    }
  }

  // Empty and oversized byte strings are rejected before async resolution.
  // Their runtime transition is an exact state stutter.
  action preflight_rejected: bool = {
    s' = s
  }

  action begin(input: int): bool = all {
    INPUTS.contains(input),
    s.generation < MAX_GENERATION,
    s' = {
      phase: RESOLVING,
      generation: s.generation + 1,
      candidate: input,
      last_action: s.last_action,
    },
  }

  action resolve_current: bool = all {
    s.phase == RESOLVING,
    s' = if (valid_input(s.candidate)) {
      phase: PENDING,
      generation: s.generation,
      candidate: INPUT_NONE,
      last_action: input_action(s.candidate),
    } else {
      phase: IDLE,
      generation: s.generation,
      candidate: INPUT_NONE,
      last_action: s.last_action,
    },
  }

  action consume_current: bool = all {
    s.phase == PENDING,
    s.last_action != ACTION_NONE,
    s' = {
      phase: IDLE,
      generation: s.generation,
      candidate: INPUT_NONE,
      last_action: s.last_action,
    },
  }

  // Exhaustion must never reuse a generation token. It revokes the admitted
  // intent and remains closed at the maximum portable counter.
  action exhaustion_fail_closed: bool = all {
    s.generation == MAX_GENERATION,
    s' = {
      phase: IDLE,
      generation: MAX_GENERATION,
      candidate: INPUT_NONE,
      last_action: ACTION_NONE,
    },
  }

  action stale_or_rejected(token: int): bool = all {
    TOKENS.contains(token),
    token != s.generation or s.phase != RESOLVING,
    s' = s,
  }

  action begin_rotate_api_key: bool = begin(INPUT_ROTATE_API_KEY)
  action begin_review_reconciliation: bool =
    begin(INPUT_REVIEW_RECONCILIATION)
  action begin_unknown_action: bool = begin(INPUT_UNKNOWN_ACTION)
  action stale_completion: bool = stale_or_rejected(0)

  action step: bool = {
    nondet input = INPUTS.oneOf()
    nondet token = TOKENS.oneOf()
    any {
      preflight_rejected,
      begin(input),
      resolve_current,
      consume_current,
      exhaustion_fail_closed,
      stale_or_rejected(token),
    }
  }

  val domain_bounds = and {
    s.phase >= IDLE,
    s.phase <= PENDING,
    s.generation >= 0,
    s.generation <= MAX_GENERATION,
    s.candidate >= INPUT_NONE,
    s.candidate <= INPUT_NON_CANONICAL,
    s.last_action >= ACTION_NONE,
    s.last_action <= ACTION_REVIEW_RECONCILIATION,
  }

  val phase_shape = and {
    (s.phase == RESOLVING) == (s.candidate != INPUT_NONE),
    (s.phase == PENDING).implies(and {
      s.candidate == INPUT_NONE,
      s.last_action != ACTION_NONE,
    }),
    (s.phase == IDLE).implies(s.candidate == INPUT_NONE),
  }

  val accepted_action_is_allowlisted =
    (s.last_action != ACTION_NONE).implies(
      Set(
        ACTION_ROTATE_API_KEY,
        ACTION_REVIEW_RECONCILIATION
      ).contains(s.last_action)
    )

  val deep_link_admission_safety = and {
    domain_bounds,
    phase_shape,
    accepted_action_is_allowlisted,
  }

  val idle_reached = s.phase == IDLE
  val resolving_reached = s.phase == RESOLVING
  val pending_reached = s.phase == PENDING
}
