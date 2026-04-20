# StockMind Handoff Issues

## Purpose

This file tracks the handoff and standardization problems that are visible right now.

Use it for unresolved gaps, not for finished work history.

## Current Blocking Issues

- `tests/security_chair_resolution_builder_unit.rs` is now the first recorded blocking regression on this branch
- observed on 2026-04-20 during `$env:CARGO_TARGET_DIR='E:\SM\target_p12_enhanced'; cargo test -- --nocapture`
- failure shape: contextual fixture deserialization now errors with `missing field 'sma_20'`
- current assessment: this blocker appears unrelated to the new post-P12 preview/request-bridge slice and should be handled as a separate chair-fixture repair task, not folded silently into P13 delivery

## Current Active Gaps

- post-P12 preview and P13 request-bridge delivery are focused-green, but branch-health truth must now carry the unrelated chair-fixture blocker until it is fixed and re-verified

## Optional Enhancements

- [ ] if richer cross-document graph coverage is needed later, rerun Graphify with semantic document extraction instead of the current AST-only code audit
- [ ] continue normal historical-doc maintenance as older background files are next touched, but this is no longer a current branch-health issue

## Maintenance Rule

Remove an item from this file only after:

- the problem was actually fixed
- the current status file was refreshed if branch health changed
- the fix was recorded in `CHANGELOG_TASK.MD`
