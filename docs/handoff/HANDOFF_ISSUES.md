# StockMind Handoff Issues

## Purpose

This file tracks the handoff and standardization problems that are visible right now.

Use it for unresolved gaps, not for finished work history.

## Current Blocking Issues

- none in the latest verified branch state
- historical note: older `security_scorecard_training_nikkei_futures_*` artifacts still exist under runtime outputs and may still reflect the pre-futures 19-feature contract, but they do not describe the latest working-tree truth

## Current Active Gaps

- `P15` apply-bridge delivery is now green at both the focused-test layer and the latest repository-wide `cargo test --no-fail-fast` rerun for this exact working tree
- `P15` remains intentionally bounded to governed runtime execution recording through `security_execution_record`; it is not broker execution and does not introduce cross-symbol rollback

## Optional Enhancements

- [ ] if richer cross-document graph coverage is needed later, rerun Graphify with semantic document extraction instead of the current AST-only code audit
- [ ] continue normal historical-doc maintenance as older background files are next touched, but this is no longer a current branch-health issue

## Maintenance Rule

Remove an item from this file only after:

- the problem was actually fixed
- the current status file was refreshed if branch health changed
- the fix was recorded in `CHANGELOG_TASK.MD`
