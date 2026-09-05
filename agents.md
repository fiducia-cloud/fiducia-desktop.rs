# Agent instructions

The parent `/Users/maca5/codes/fiducia-cloud/AGENTS.md` and `.github/agents.md`
apply. Work directly on `main`; coordinate ownership before editing; preserve
unrelated changes; and do not use destructive Git or filesystem commands.

`formal/app_lifecycle.qnt` is normative. Any lifecycle change must update the
Quint model, deterministic Quint traces, the Rust implementation, bounded Rust
tests, the Flutter companion implementation and tests, and both formal-model
copies together. The two `.qnt` source pairs must remain byte-identical.

Views must derive capabilities from `AppSnapshot`. Adapters may execute only
effects emitted by `AppLifecycleMachine` and must return completions with the
exact operation generation. Do not add independent authentication, tenant,
offline, confirmation, reconciliation, recovery, or sign-out booleans.

Never describe randomized or bounded verification as an unbounded proof. A
release requires native build and acceptance evidence in addition to the formal
and implementation checks.

## Repository-local Git worktrees

- Create or use a Git worktree only when the human operator explicitly authorizes it for the current task. Concurrency or a dirty checkout is not permission by itself.
- Put every authorized worktree at `<repository-root>/tmp/worktrees/<name>`; from the repository root, use `./tmp/worktrees/<name>`. Never place worktrees beside repositories or organization directories.
- Keep `tmp`, `temp`, `tmp/worktrees`, and `temp/worktrees` ignored in the repository-root `.gitignore`. Do not commit files from those directories.
- Relocate or remove a worktree only when the operator explicitly requests it. Before removal, preserve and publish intended changes, verify its commit is represented on the target branch, and confirm there are no tracked, untracked, ignored-sensitive, or in-use files that must survive. Remove it with `git worktree remove <path>` without `--force`; never delete a worktree directory with `rm`.
