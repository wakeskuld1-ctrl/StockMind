# StockMind Handoff Procedure

## Purpose

This procedure defines the minimum repeatable handoff flow for the next AI or engineer.

It separates stable governance files from branch-local truth so that future sessions do not mistake historical notes for current repository health.

## Read Order For A New Session

1. `README.md`
2. `docs/product/project_intent.md`
3. `docs/governance/contract_registry.md`
4. `docs/governance/decision_log.md`
5. `docs/governance/acceptance_criteria.md`
6. `docs/governance/response_contract.md`
7. `docs/handoff/CURRENT_STATUS.md`
8. `docs/handoff/HANDOFF_ISSUES.md`
9. `docs/AI_HANDOFF.md`

## Handoff Update Flow

Use this sequence at the end of any meaningful engineering task:

1. record the actual branch and workspace status
2. run the smallest honest verification set
3. update `docs/handoff/CURRENT_STATUS.md` if branch health changed
4. update `docs/handoff/HANDOFF_ISSUES.md` if new blockers, drift, or unresolved gaps were found
5. update governance files if contracts, intent, decisions, acceptance, or response rules changed
6. append the task to `CHANGELOG_TASK.MD`

## What Goes Where

### Stable governance files

Use these for durable project rules:

- `docs/product/project_intent.md`
- `docs/governance/contract_registry.md`
- `docs/governance/decision_log.md`
- `docs/governance/acceptance_criteria.md`
- `docs/governance/response_contract.md`

### Branch-local truth files

Use these for current state:

- `docs/handoff/CURRENT_STATUS.md`
- `docs/handoff/HANDOFF_ISSUES.md`

### Historical or architectural context

Use these for broader background:

- `docs/AI_HANDOFF.md`
- `docs/handoff/AI_HANDOFF.md`
- `docs/architecture/stockmind-acceptance-checklist.md`
- `docs/architecture/stockmind-snapshot-manifest.md`

## Handoff Completion Checklist

- [ ] current branch and commit captured
- [ ] current verification results captured
- [ ] unresolved blockers recorded
- [ ] governance files updated if the task changed project rules
- [ ] `CHANGELOG_TASK.MD` appended

## Graphify Requirement

When a large feature phase or architecture checkpoint finishes, generate or refresh the graph audit for the repository.

Until that happens, keep the missing graph audit listed as an explicit gap instead of silently dropping it from handoff.
