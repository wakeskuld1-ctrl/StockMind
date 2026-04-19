# StockMind AI Handoff

## 1. Current Mainline

- Date: 2026-04-17
- Active main project path: `D:\SM`
- Main upstream repo: `https://github.com/wakeskuld1-ctrl/StockMind.git`
- Mainline branch source: `main`
- Current working branch for the clean upload slice: `codex/p10-p11-clean-upload-20260420`

This repository is now the primary delivery line for the standalone stock and ETF analysis product.

The old local repository `D:\Rust\Excel_Skill` is no longer the main development flow. It remains a reference source for selective capability backport only.

## 2. Architecture Boundary

Follow this rule going forward:

1. Keep `D:\SM` as the only mainline implementation repo.
2. Do not perform broad refactors unless a change is truly blocked by the current structure.
3. If an old capability in `D:\Rust\Excel_Skill` is still valuable, backport only the minimum useful slice.
4. Do not re-import old Excel, table, or generic foundation packages into `StockMind` as a full bundle.

Reason:

- The user explicitly asked for a stable architecture after this boundary reset.
- Future work should continue on the chosen architecture instead of repeatedly reshaping it.
- Non-essential refactors create delivery drag and make AI handoff harder.

## 3. What Was Backported In This Round

The current round already merged a small governed chair-resolution slice from the old repository into `D:\SM`.

Updated files:

- `D:\SM\src\ops\security_chair_resolution.rs`
- `D:\SM\tests\security_chair_resolution_cli.rs`

Backported output fields on chair resolution:

- `entry_grade`
- `entry_reason`
- `entry_blockers`
- `target_gross_pct`
- `sizing_grade`
- `sizing_reason`

Implementation rule used:

- Reuse the existing shared helper path from `security_position_plan`
- Do not create a new chair-only heuristic branch

This was intentionally a minimal backport instead of a structural rewrite.

## 4. Verified Result

Focused tests already passed in `D:\SM`:

```powershell
cargo test security_chair_resolution_downgrades_to_abstain_when_scorecard_model_is_unavailable --test security_chair_resolution_cli -- --nocapture
cargo test security_chair_resolution_outputs_formal_final_action_separate_from_committee_and_scorecard --test security_chair_resolution_cli -- --nocapture
```

Important note:

- This round validated the chair-resolution slice only.
- Full repository regression has not been rerun yet.

## 5. Current Working Tree Status

At the time of this handoff, the upload branch was rebuilt in a clean isolated worktree so the pushed slice can exclude local runtime artifacts and database byproducts.

- Clean upload worktree path: `C:\wt\smu`
- Upload branch goal: ship `P10/P11` portfolio-core code, tests, and handoff docs only
- Explicitly excluded from this upload: `.stockmind_runtime/` databases, replay artifacts, and other machine-local training outputs

Important boundary:

- the original main workspace `D:\SM` may still contain unrelated local changes and runtime artifacts
- do not assume those runtime databases were uploaded; they were intentionally excluded from the clean Git delivery

## 6. Relationship Between Repos

### `D:\SM`

This is the active standalone product repo and should be treated as the only primary engineering line.

### `D:\Rust\Excel_Skill`

This is now a reference repo only.

Useful reference materials already written there:

- External historical reference only: `D:\Rust\Excel_Skill\docs\plans\2026-04-17-stockmind-mainline-reconciliation-design.md`
- External historical reference only: `D:\Rust\Excel_Skill\docs\plans\2026-04-17-stockmind-mainline-reconciliation-plan.md`

Current `D:\SM` canonical documents for this line of work:

- `D:\SM\docs\plans\design\2026-04-18-post-open-position-data-system-design.md`
- `D:\SM\docs\plans\execution\2026-04-18-post-open-position-data-system-implementation-plan.md`

Use the external repo only to inspect old capability behavior, then backport only the needed slice into `D:\SM`. Treat the `D:\SM\docs\...` documents above as the current authoritative path for this repository.

### `D:\Rust\Stock`

This is a separate Rust workspace with real data access skeleton and local persisted market data.

It is not the same project as `D:\SM`, and it should not be merged wholesale into `StockMind`.

Use it as a reference source for market data adapter patterns, storage shape, and service wiring.

## 7. Audit Of `D:\Rust\Stock`

### Workspace shape

`D:\Rust\Stock\Cargo.toml` defines a Rust workspace with:

- `server`
- `domain`
- `infra`

### Environment contract

`D:\Rust\Stock\.env.example` currently contains:

```env
DATABASE_URL=sqlite:stock.db
TUSHARE_TOKEN=your_tushare_token_here
```

This means the project is designed to run against a local SQLite database and optionally a Tushare token.

### Real data wiring

`D:\Rust\Stock\server\src\main.rs` shows that the running server uses:

- market source: `SinaMarketSource`
- repository: `SqliteStockRepository`

It also exposes these API routes:

- `/api/klines/:symbol`
- `/api/analyze/:symbol`
- `/api/stocks`
- `/api/trade/buy`
- `/api/trade/sell`
- `/api/backtest`
- `/api/sync`
- `/api/sync/status`

### Adapter inventory

`D:\Rust\Stock\infra\src\adapter\` currently contains:

- `sina.rs`
- `eastmoney.rs`
- `akshare.rs`
- `tushare.rs`
- `market_provider.rs`

Observed adapter relationship:

- `SinaMarketSource` is the active default source in the server path
- `sina.rs` uses `EastMoneyClient` as a fallback path for stock-list related coverage
- `akshare.rs` also reuses `EastMoneyClient`
- `tushare.rs` exists as another source option and depends on `TUSHARE_TOKEN`

### SQLite persistence shape

`D:\Rust\Stock\infra\src\repository\sqlite.rs` initializes and maintains a fairly broad local schema, including:

- `stocks`
- `klines`
- `stocks_daily`
- `security_info`
- `daily_valuation`
- `market_sentiment`
- `analysis_results`
- `portfolios`
- `positions`
- `trade_logs`
- `portfolio_opt_task`
- `portfolio_opt_result`

This confirms that `D:\Rust\Stock` is already more than a toy fetcher. It includes real local persistence for market data, analysis artifacts, and trade simulation records.

### Local data artifacts

The project root currently contains local runtime artifacts such as:

- `stock.db`
- `training_data.parquet`
- `stock_model.onnx`
- `onnxruntime.dll`

This further confirms that the workspace already carries a practical local-data and model-serving skeleton.

## 8. Recommended Use Of `D:\Rust\Stock`

Use `D:\Rust\Stock` for reference in these scenarios:

- when `D:\SM` needs a cleaner market data adapter design reference
- when we need to study SQLite table design for historical market data persistence
- when we want to copy a small proven pattern for Sina, EastMoney, or Tushare integration

Do not use `D:\Rust\Stock` for these actions by default:

- full repository merge into `D:\SM`
- direct module transplant without boundary review
- copying Python-side workflow assumptions into the mainline product path

## 9. Next AI Should Do

Default next-step order:

1. Continue implementation in `D:\SM`
2. Respect the current architecture boundary and avoid non-essential refactors
3. If more local capability is needed, inspect `D:\Rust\Excel_Skill` first for business logic and inspect `D:\Rust\Stock` for data adapter/storage patterns
4. Backport only the minimum vertical slice needed for the next user-facing capability
5. Run focused tests first, then decide whether broader regression is needed

## 10. Position-Management Process Lock

For any future work related to:

- post-open position management
- daily monitoring
- position evidence output
- adjustment simulation
- capital rebasing
- committee-review data handoff

the next AI must follow this document as the controlling business-flow reference:

- `D:\SM\docs\plans\design\2026-04-18-post-open-position-data-system-design.md`
- `D:\SM\docs\governance\post_open_position_data_graph.json`

This rule is mandatory unless the user explicitly approves a business-flow change.

That document currently fixes these non-negotiable points:

- it is the single source of truth for the whole post-open pure-data flow
- the companion JSON graph is the machine-readable source of truth for the same flow and the current file/method ownership map
- the system entry is `Approved Open Position Packet`, not a raw stock candidate
- the managed object is an account-level position, not an isolated stock
- daily monitoring must output `Monitoring Evidence Package`
- future LLM committee review is a downstream consumer, not part of the current pure-data implementation slice
- capital changes are first-class `CapitalEvent` rebasing events, not ordinary add / trim actions
- neither adjustment simulation data nor capital rebalance evidence may bypass the future committee / chair chain and directly create execution input
- Task 6 boundary hardening on 2026-04-19 also locked these cases at the CLI surface:
  - unsupported `event_type` must fail
  - oversized `withdraw_capital` must fail before capital goes negative
  - `dividend_reinvest` is an inflow event, not a position-add alias
  - `external_cash_out` is an outflow event, not a trim alias, and must carry governance warning context
  - `MonitoringEvidencePackage` and `PositionContract` must belong to the same account as the `CapitalEvent`

For Task 6 specifically, the next AI must remember:

- Task 6 ends at `CapitalRebalanceEvidencePackage`
- Task 6 is allowed to normalize capital-event semantics and rebase contracts
- Task 6 is not allowed to produce `AdjustmentInputPackage`
- Task 6 is not allowed to bypass future committee / chair review
- Task 6 is not allowed to collapse capital events into ordinary add / trim / replace / exit logic

The 2026-04-19 guard-first follow-up also fixed these operating rules:

- `D:\SM\tests\post_open_position_data_flow_guard.rs` is now the minimal source-guard for the post-open governance gate
- `MonitoringEvidencePackage` and `CapitalRebalanceEvidencePackage` must continue to flow into committee/chair review before `AdjustmentInputPackage`
- `security_record_position_adjustment.rs` remains a legacy compatibility recorder and must stay separate from the now-implemented `security_adjustment_input_package.rs`
- the 2026-04-19 Task 7 follow-up landed `security_committee_decision_package` as the first implemented post-open committee handoff
- `security_committee_decision_package` is evidence-only, may merge monitoring plus optional capital-rebalance evidence, and must keep `produces_adjustment_input=false`
- the 2026-04-19 Task 7 continuation also landed `security_adjustment_input_package` as the approved-only downstream bridge
- `security_adjustment_input_package` must reject non-approved governance input and account-mismatched committee packages
- the 2026-04-19 Task 7 continuation now also accepts one formal artifact-driven governance path
- the artifact-driven path must assemble refs from `submit_approval_output`, chair semantics from `chair_resolution`, lifecycle trigger linkage from `condition_review`, and explicit sizing math from `sizing_decision`
- `security_adjustment_input_package` must reject requests that provide both direct `governance_approval` and `governance_artifacts`, or neither
- `security_adjustment_input_package` must reject `condition_review` ref drift against `submit_approval_output`, and must reject symbol/date drift across the governance artifacts
- `security_adjustment_input_package` may only preview `SecurityExecutionRecordRequest` and legacy `SecurityRecordPositionAdjustmentRequest`; it must not directly execute or persist side effects
- the 2026-04-19 Task 9 follow-up landed `security_closed_position_archive` as the final formal object in the minimum post-open lifecycle
- `security_closed_position_archive` must anchor on a closed `SecurityExecutionRecordDocument` and may optionally enrich from `SecurityPostTradeReviewDocument` and `SecurityPositionContract`
- `security_closed_position_archive` must hard-fail on open execution records, missing exit date/reason, post-trade review symbol drift, post-trade review execution-record-ref drift, and position-contract symbol/account drift
- `Task 9` completion means the minimum coherent post-open lifecycle is now code-complete, but it still does not imply institution-grade portfolio, risk, execution, or adaptive-learning capability
- retrospective reporting was later corrected to remain outside the mathematical core
- the current approved boundary is: the core only provides structured data support from `ClosedPositionArchive` plus optional `SecurityPostTradeReviewDocument`, while actual audit/report writing belongs to later Skill-layer consumers
- the next approved expansion stage after `Task 9` is `P10-P12` portfolio core
- `P10-P12` is now decomposed in design as:
  - `P10` account objective normalization
  - `P11` unified portfolio replacement solver
  - `P12` governed portfolio allocation decision
- current implemented status on 2026-04-19 is:
  - `P10 / Task 1-2`: implemented
    - tool: `security_account_objective_contract`
    - outputs: `AccountObjectiveContract` + `PortfolioCandidateSet`
  - `P11 / Task 3-4`: implemented
    - tool: `security_portfolio_replacement_plan`
    - outputs: unified replacement plan with current/target weights, action sections, capital migration summary, rebase-aware context, conflict-resolution summary, and structured action summary
  - `P12`: not implemented yet
- future sessions must not treat `P12` as implemented just because `P10` and `P11` are now live
- the following focused verification commands were green at the current handoff point:
  - `cargo test --test security_account_objective_contract_cli -- --nocapture`
  - `cargo test --test security_portfolio_replacement_plan_cli -- --nocapture`
  - `cargo test --test security_account_objective_contract_cli --test security_portfolio_replacement_plan_cli -- --nocapture`
- `security_record_position_adjustment.rs` remains the downstream legacy compatibility recorder and must not be treated as the new mainline just because the bridge now exists
- the 2026-04-19 ETF/proxy-history chair failures are now fixed and must remain frozen as runtime rules:
  - ETF alias strings such as `treasury_etf` / `gold_etf` must normalize before scorecard subscope comparison
  - ETF proxy-complete evidence may replace stock-only fundamental/disclosure completeness with `governed_etf_proxy_information`
  - latest ETF runs without `as_of_date` must anchor `analysis_date` to the resolved governed proxy date
- if workspace-wide `cargo test` is blocked by unrelated active work, the source-guard may be verified with standalone `rustc --test` so long as the guard itself remains pure file-content validation and does not claim broader repository health

If a future AI believes a different process is better, it must:

1. stop and surface the conflict explicitly
2. cite the current design document
3. wait for user approval before changing the process

Do not silently invent a parallel flow, shortcut the intake contract, or collapse capital rebasing into ordinary position adjustment logic.

## 10.1 Single Source Set

For future work on this project, the following five files are now the compact single-source-of-truth set for intent, contract, decisions, acceptance, and answer delivery:

- `D:\SM\docs\product\project_intent.md`
- `D:\SM\docs\governance\contract_registry.md`
- `D:\SM\docs\governance\decision_log.md`
- `D:\SM\docs\governance\acceptance_criteria.md`
- `D:\SM\docs\governance\response_contract.md`

They answer:

- `project_intent.md`: what the project is for, what success means, what is out of scope, and which boundaries may not be crossed silently
- `contract_registry.md`: which formal objects exist, what they consume/produce, which fields are mandatory, and which failures are hard-fail boundaries
- `decision_log.md`: which major decisions are fixed, why they were chosen, which assumptions remain active, and which open questions still exist
- `acceptance_criteria.md`: what counts as done for design, contract, implementation, guard, and traceability work
- `response_contract.md`: what a high-quality answer must expose, cite, and refuse when necessary

## 10.2 Mandatory Update Rule

For every future development task in this project:

1. review these five files before implementation when the task touches project intent, contracts, decisions, acceptance, or answer behavior
2. update every affected file in the same task
3. if a file was reviewed but required no content change, say that explicitly in the completion summary
4. record the task in `.trae/CHANGELOG_TASK.md`

If code changes are made without checking and updating the relevant files, the task should be treated as incomplete from a handoff/governance perspective even if the code itself works.

## 11. Non-Negotiable Handoff Memory

These points should be treated as stable operating memory for the next AI session:

- Mainline is `D:\SM`, not `D:\Rust\Excel_Skill`
- `D:\Rust\Excel_Skill` is reference-only
- `D:\Rust\Stock` is a separate data-access reference project
- Do not keep refactoring the architecture unless there is a real blocker
- Future work should follow the established architecture, not restart it
- Prefer minimal backport, focused verification, and continuous delivery
- For post-open-position and position-management work, follow `D:\SM\docs\plans\design\2026-04-18-post-open-position-data-system-design.md` unless the user explicitly changes the business flow
