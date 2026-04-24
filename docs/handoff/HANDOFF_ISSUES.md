# StockMind Handoff Issues

## Purpose

This file tracks the handoff and standardization problems that are visible right now.

Use it for unresolved gaps, not for finished work history.

## Current Blocking Issues

- none confirmed by fresh verification on 2026-04-24
- latest full regression evidence: `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_full_regression_20260424_c'; cargo test -- --nocapture`
- result: repository-wide `cargo test` completed green in this worktree after restoring the missing `docs/plans/design/` boundary docs and the `Stock/Foundation Decoupling Baseline` handoff marker

## Current Active Gaps

- the worktree remains intentionally dirty with unrelated runtime artifacts, generated fixtures, and parallel edits, so any Git delivery must keep staging narrowly scoped
- the `docs/plans/` to `docs/plans/design/` migration is still easy to drift; future guard additions must backfill the new-path design docs and handoff markers in the same change
- the current P15 direct adapter is intentionally bounded to governed runtime execution recording through `security_execution_record`; it is not broker execution, broker-fill replay, order-ledger exactness, or cross-symbol rollback

## Optional Enhancements

- [ ] if richer cross-document graph coverage is needed later, rerun Graphify with semantic document extraction instead of the current AST-only code audit
- [ ] continue normal historical-doc maintenance as older background files are next touched, but this is no longer a current branch-health issue

## Maintenance Rule

Remove an item from this file only after:

- the problem was actually fixed
- the current status file was refreshed if branch health changed
- the fix was recorded in `CHANGELOG_TASK.MD`
