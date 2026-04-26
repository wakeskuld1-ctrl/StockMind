# P20B Lifecycle Closeout Evidence Package Design

## Intent
- Goal: add P20B as a read-only lifecycle closeout evidence package after P20A readiness.
- Scope: consume one formal P20A `SecurityPortfolioExecutionLifecycleCloseoutReadinessDocument`, point-read target runtime execution records for P20A eligible rows, verify whether each row has machine-readable closed-position evidence, preserve P20A blockers, and emit one evidence document for a future archive or lifecycle writer.
- Non-goals: do not write runtime facts, do not call `security_execution_record`, do not call `security_post_trade_review`, do not call or depend on `security_closed_position_archive`, do not produce a closed-position archive, do not replay broker fills, do not create broker orders, do not materialize positions, and do not claim lifecycle closed.
- Success definition: callers can distinguish rows with enough closed execution-record evidence for a later closeout/archive phase from rows still blocked by open, missing, mismatched, or incomplete runtime facts, without mutating runtime state.
- Delivery form: design doc now; after approval, implementation plan, later Rust module, CLI tests, source guards, public stock boundary wiring, governance docs, handoff notes, and append-only task journal entry.

## Contract
- Tool name: `security_portfolio_execution_lifecycle_closeout_evidence_package`.
- Request contract: `SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageRequest`.
- Primary output contract: `SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageDocument` wrapped by `SecurityPortfolioExecutionLifecycleCloseoutEvidencePackageResult`.
- Required input:
  - one P20A `SecurityPortfolioExecutionLifecycleCloseoutReadinessDocument`
  - optional `created_at`
  - runtime execution store configuration through the existing store facade, only for point reads by `target_execution_record_ref`
- Required P20A identity:
  - `document_type == "security_portfolio_execution_lifecycle_closeout_readiness"`
  - `contract_version == "security_portfolio_execution_lifecycle_closeout_readiness.v1"`
  - `runtime_write_count == 0`
  - source P19E ref, P19D ref, P19C ref, and source non-atomicity notice are present
- Required P20A row evidence for rows entering runtime read:
  - `readiness_status == "eligible_for_closeout_preflight"`
  - `closeout_preflight_eligible == true`
  - non-empty `target_execution_record_ref`
  - non-empty `commit_idempotency_key`
  - non-empty `canonical_commit_payload_hash`
  - non-empty `source_p19c_ref`
  - runtime replay idempotency key, payload hash, and source P19C ref are present
- Runtime closeout evidence for an evidence-ready row:
  - runtime execution record exists at `target_execution_record_ref`
  - runtime `document_type == "security_execution_record"`
  - runtime `execution_record_id == target_execution_record_ref`
  - runtime `position_state == "closed"`
  - runtime `actual_exit_date` is present
  - runtime `actual_exit_price > 0`
  - runtime `exit_reason` is present and not `position_still_open`
  - runtime replay metadata matches the P20A row: idempotency key, payload hash, and source P19C ref
  - runtime symbol and account lineage match the P20A document/row
- Evidence row statuses:
  - `evidence_ready_for_closeout_archive_preflight`
  - `blocked_p20a_not_eligible`
  - `blocked_missing_runtime_record`
  - `blocked_runtime_record_identity_mismatch`
  - `blocked_runtime_record_not_closed`
  - `blocked_missing_exit_evidence`
  - `blocked_replay_metadata_mismatch`
  - `blocked_account_or_symbol_mismatch`
  - `blocked_unknown_p20a_readiness_status`
- Document statuses:
  - `no_closeout_evidence_candidates`
  - `closeout_evidence_ready`
  - `partial_closeout_evidence_ready`
  - `blocked`
  - `rejected`
- Output counts:
  - `evidence_row_count`
  - `evidence_ready_for_closeout_archive_preflight_count`
  - `blocked_p20a_not_eligible_count`
  - `blocked_missing_runtime_record_count`
  - `blocked_runtime_record_identity_mismatch_count`
  - `blocked_runtime_record_not_closed_count`
  - `blocked_missing_exit_evidence_count`
  - `blocked_replay_metadata_mismatch_count`
  - `blocked_account_or_symbol_mismatch_count`
  - `blocked_unknown_p20a_readiness_status_count`
  - `runtime_read_count`
  - `runtime_write_count`
- Runtime counts:
  - `runtime_read_count` may increase only for P20A eligible rows that require point-read evidence.
  - `runtime_write_count` must always be `0`.
- Rejection conditions:
  - unsupported or missing P20A document identity
  - unsupported P20A contract version
  - P20A `runtime_write_count != 0`
  - missing P20A source P19E/P19D/P19C refs
  - missing preserved source non-atomicity notice
  - an eligible P20A row lacks target execution ref, commit key, payload hash, source P19C ref, or runtime replay metadata evidence
  - request attempts to provide archive write, runtime write, post-trade review, broker-fill replay, position materialization, or lifecycle closure semantics
- Traceability requirements:
  - Preserve P20A readiness document ref, source P19E ref, P19D ref, P19C ref, P20A row readiness status, target execution ref, idempotency key, payload hash, runtime replay metadata, and runtime closeout evidence fields.
  - Preserve P19D/P19E/P20A non-atomic partial truth; do not convert partial evidence into batch-level closure.
  - Preserve blocked row reasons explicitly; do not roll them up into a generic failure.
  - Do not use free-text notes as closeout evidence.

## Hard Rejection Red Lines
- Do not call `security_execution_record` from P20B.
- Do not call `security_post_trade_review` from P20B.
- Do not call or depend on `security_closed_position_archive` from P20B.
- Do not call `open_session`, `upsert_execution_record`, `upsert_position_plan`, `upsert_adjustment_event`, repository upsert functions, SQLite `.execute`, SQLite `.execute_batch`, or any runtime mutation API from P20B.
- Do not use transaction-opening session APIs for P20B reads; use only the existing store facade point-read pattern needed to load a target execution record.
- Do not produce a closed-position archive document.
- Do not claim lifecycle closed.
- Do not treat P20A `eligible_for_closeout_preflight` as closed-position evidence.
- Do not treat P19E `verified` or P19D replay commit metadata as broker-fill evidence.
- Do not require all rows to be evidence-ready before preserving row-level eligible evidence.

## Decision
- Chosen approach: P20B as lifecycle closeout evidence package, not an archive writer.
- Why: P20A proves readiness to check closeout evidence, but it does not prove runtime execution records are closed. P20B creates the next governed read-only fact while preserving the boundary before archive/lifecycle mutation.
- Rejected alternative: direct P20B archive writer. It is premature because current local `src/ops` and `tests` searches do not show a callable `security_closed_position_archive` implementation or route.
- Rejected alternative: calling `security_post_trade_review` to infer closeout evidence. That path is outside P20B and may enter execution-record write semantics.
- Rejected alternative: treating P20A eligible rows as archive-ready. P20A only checks replay metadata readiness; P20B must verify closed execution-record evidence separately.
- Known tradeoff: P20B adds another read-only layer, but it prevents readiness and closure from collapsing into one ambiguous contract.
- Open question resolved for this design: P20B may report partial evidence readiness. It must not require all rows to be ready before emitting row-level evidence.

## Acceptance
- Before implementation starts:
  - this design document exists under `docs/plans/`
  - an implementation plan exists under `docs/plans/`
  - P20A repository-wide green verification is recorded in `docs/handoff/CURRENT_STATUS.md`
  - the public-boundary sync list names every file that must move with P20B
  - the risk pass limitations are incorporated honestly
- Before completion can be claimed:
  - P20B tests are written before production code and observed red for missing tool/module behavior
  - tests prove wrong P20A identity and contract version are rejected
  - tests prove P20A `runtime_write_count != 0` is rejected
  - tests prove missing P20A source refs and missing non-atomicity notice are rejected
  - tests prove P20A ineligible or blocked rows remain `blocked_p20a_not_eligible`
  - tests prove eligible P20A rows with missing runtime record become `blocked_missing_runtime_record`
  - tests prove open runtime records become `blocked_runtime_record_not_closed`
  - tests prove closed runtime records with missing exit date, missing/zero exit price, or `position_still_open` exit reason become `blocked_missing_exit_evidence`
  - tests prove replay metadata mismatches become `blocked_replay_metadata_mismatch`
  - tests prove account or symbol mismatches become `blocked_account_or_symbol_mismatch`
  - tests prove fully closed and metadata-matching runtime records become `evidence_ready_for_closeout_archive_preflight`
  - tests prove mixed evidence-ready and blocked rows produce `partial_closeout_evidence_ready`
  - tests prove `runtime_write_count == 0`
  - source guard proves P20B does not call `security_execution_record`, `security_post_trade_review`, `security_closed_position_archive`, SQLite `.execute`, SQLite `.execute_batch`, `open_session`, `upsert_execution_record`, repository upsert functions, or runtime mutation APIs
  - boundary, catalog, dispatcher, frozen manifest, contract registry, decision log, handoff, and task journal are synchronized
  - focused P20B tests, adjacent P20A tests, boundary guards, `cargo check`, and repository-wide regression pass

## Cross-Artifact Contract

| Boundary Item | Source Of Truth | Runtime Entrypoints | Frozen/Derived Artifacts | Guard Tests | Required Sync |
|---|---|---|---|---|---|
| `security_portfolio_execution_lifecycle_closeout_evidence_package` | this design, implementation module, P20B CLI tests | `src/ops/stock.rs`, `src/ops/stock_execution_and_position_management.rs`, `src/tools/catalog.rs`, `src/tools/dispatcher.rs`, `src/tools/dispatcher/stock_ops.rs` | `tests/stock_formal_boundary_manifest_source_guard.rs`, contract registry, decision log, current status, handoff issues, task journal | P20B CLI tests, stock formal boundary guard, catalog grouping guard, dispatcher grouping guard, cargo check, full regression | add module, grouped export, catalog entry, dispatcher route, frozen manifest entry, governance rows, handoff notes |
| P20B read-only runtime evidence boundary | P20B design, P20A readiness contract, execution-record document contract | existing store facade point-read by target execution record ref only | P20B source guard and evidence rows | P20B source guard, runtime-read evidence tests, adjacent P20A tests | forbid write APIs, session APIs, post-trade/archive calls, direct SQLite mutation, and free-text evidence |
| closeout evidence readiness | runtime execution record fields plus P20A row metadata | no runtime write entrypoint | P20B evidence row evidence | ready/blocked row tests and partial-evidence test | require closed position state, exit date, positive exit price, non-open exit reason, replay metadata match, account/symbol match |

## Independent Risk Pass
- Mode: attempted `user-approved-subagent`; blocked by external quota error; fallback `inline-fresh-pass` after user instructed to continue.
- Trigger: P20B adds a new public tool after P20A and introduces runtime reads close to lifecycle/archive semantics, so boundary drift and semantic overclaim risk are high.
- Fresh-context question: Can P20B safely consume P20A readiness and verify closeout evidence without writing runtime facts, producing archives, or claiming lifecycle closure?
- Findings:
  - No existing P20B design or implementation exists.
  - Current local `docs/plans` only contains P20A for P20.
  - Current local `src/ops` and `tests` searches did not find a `security_closed_position_archive` implementation or test file, even though handoff notes mention historical restored routes.
  - P19E already demonstrates the acceptable read-only pattern: use a store point read such as `load_execution_record` and emit `runtime_write_count = 0`.
  - `SecurityExecutionRecordDocument` has closed-state evidence fields: `position_state`, `actual_exit_date`, `actual_exit_price`, and `exit_reason`.
  - `SecurityExecutionStoreSession` exposes write-adjacent methods and opens a transaction; P20B should not use session APIs for read-only evidence packaging.
- Blocking gaps:
  - A genuinely independent subagent risk result is unavailable in this run due to quota failure.
  - Future archive/lifecycle writer remains blocked until the archive contract and route are present and frozen locally.
  - P20B must not be implemented until the user approves this design and a separate implementation plan is written.

## Next Skill
- Use `writing-plans` to create the P20B implementation plan after user approval.
- Do not use `test-driven-development` or edit production code until the design and plan are approved.
