# StockMind Acceptance Criteria

## Goal

This file defines what counts as done from a governance and handoff perspective.

Working code alone is not enough. A task is accepted only when design intent, contract truth, verification, and handoff traceability are aligned.

## Acceptance Levels

### 1. Design accepted

A task is design-accepted when:

- the task stays inside existing boundaries, or
- a changed boundary is documented in the relevant plan file under `docs/plans/`

### 2. Contract accepted

A task is contract-accepted when:

- request and output object changes are reflected in code
- public dispatcher or catalog exposure is updated when required
- the relevant contract entry in `docs/governance/contract_registry.md` still matches reality

### 3. Verification accepted

A task is verification-accepted when:

- the smallest honest command set was run
- command results are written down truthfully
- failures are recorded as failures, not postponed into vague language

### 4. Handoff accepted

A task is handoff-accepted when:

- `docs/handoff/CURRENT_STATUS.md` reflects any material branch-health change
- unresolved blockers are recorded in `docs/handoff/HANDOFF_ISSUES.md`
- `CHANGELOG_TASK.MD` captures what changed, why, what remains, and what was verified

### 5. Repository accepted

The branch can be described as repository-accepted only when:

- `cargo check` passes
- `cargo test -- --nocapture` passes
- no known blocking issue remains in `docs/handoff/HANDOFF_ISSUES.md`

If any one of those is false, the branch may still be usable for continued development, but it must not be described as fully green.

## Required Document Updates By Change Type

| Change type | Must update |
| --- | --- |
| boundary or architecture change | relevant `docs/plans/*`, `docs/AI_HANDOFF.md`, possibly `README.md` |
| formal contract change | `docs/governance/contract_registry.md`, possibly `docs/governance/decision_log.md` |
| verification truth changed | `docs/handoff/CURRENT_STATUS.md` |
| new blocker or known drift discovered | `docs/handoff/HANDOFF_ISSUES.md` |
| task closeout | `CHANGELOG_TASK.MD` |

## Graph Audit Expectation

For a major feature phase or architecture checkpoint, acceptance is stronger when the repository also includes a graph audit artifact.

If no graph audit exists yet, the task must keep that gap explicit instead of implying it was already completed.

## Honest Language Rules

Use these phrases carefully:

- "verified" only when the command was run in the current branch
- "accepted" only when the relevant acceptance level above was satisfied
- "historical baseline" when a document records an older successful state
- "current status" only for branch-local truth in `docs/handoff/CURRENT_STATUS.md`
