# StockMind Project Intent

## Mission

StockMind exists to keep the current stock and ETF securities workflow buildable, testable, and maintainable as one standalone Rust repository.

The repository is meant to preserve the formal stock mainline without pulling the old generic foundation stack back into day-to-day delivery.

## What Success Looks Like

This project is successful when:

- the stock-only CLI surface remains buildable from one repository
- approved business flow boundaries remain explicit and testable
- governed runtime ownership stays inside the stock domain
- engineers and AI agents can continue work with low-context handoff material
- architecture growth happens through clear grouped modules instead of boundary drift

## Current Delivery Scope

The current repository scope includes:

- stock-only tool catalog and dispatcher wiring
- research, governance, execution, post-trade, and modeling flow slices that remain on the active stock mainline
- governed runtime stores and SQLite-backed stock-domain persistence
- source-guard and CLI tests that defend architecture and public-tool behavior

## Non-Goals

The current repository is not trying to:

- reopen the old foundation analytics stack
- restore the previous GUI shell
- reintroduce the old license gate
- absorb unrelated historical repos wholesale
- claim that every future stock architecture question is already settled

## Boundary Rules

- `src/ops/stock.rs` is a frozen formal boundary, not the default sink for every new capability.
- Shared and runtime zones remain allowed only for stock-facing ownership.
- Legacy committee compatibility remains frozen; new governance work stays on the formal chain `security_committee_vote -> security_chair_resolution`.
- Runtime DB ownership must continue through `src/runtime/formal_security_runtime_registry.rs`.

## Delivery Principles

- prefer minimal vertical slices over broad refactors
- verify with real commands instead of memory
- separate stable governance rules from branch-health snapshots
- keep handoff artifacts current enough that the next session can continue without rediscovering project truth

## Current Phase

As of 2026-04-20, the repository is in a standardization and handoff-hardening phase around the standalone stock mainline and the newly merged P10/P11 portfolio-core slice.

That means the main objective is not only shipping code, but also making project intent, contracts, acceptance, and handoff expectations explicit and durable.
