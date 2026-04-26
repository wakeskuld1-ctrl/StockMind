## 2026-04-17
### Modified
- Updated `src/ops/security_chair_resolution.rs` to expose governed chair-side entry and sizing fields:
  - `entry_grade`
  - `entry_reason`
  - `entry_blockers`
  - `target_gross_pct`
  - `sizing_grade`
  - `sizing_reason`
- Reused shared helpers from `security_position_plan` inside chair resolution building instead of adding a chair-only heuristic.
- Added a regression assertion in `tests/security_chair_resolution_cli.rs` for the model-unavailable downgrade case so the chair object must expose the same entry/sizing answer as the position-plan side.

### Why
- The new `StockMind` mainline already carries governed entry/sizing on the position-plan and approval side, but the final chair output was still missing the same formal answer.
- The user requested that this capability be merged back into the new standalone project rather than leaving it only in the old local repository.

### Remaining
- [ ] Decide whether the next backport slice should continue with `security_decision_submit_approval` or stop after the chair-side dual-anchor contract.
- [ ] Run a broader focused verification set if we continue to touch adjacent stock decision modules.

### Risks
- [ ] This round only verified focused chair-resolution regressions, not the full repository test suite.
- [ ] The active local project path is `D:\SM` instead of `D:\Rust\StockMind` because Windows path-length limits blocked checkout under the longer path.

### Closed
- Chair-side entry/sizing fields merged into `StockMind`
- Focused chair regression added
- Focused verification passed

## 2026-04-17
### Modified
- Rewrote `D:\SM\docs\AI_HANDOFF.md` into a practical AI handoff manual for the standalone `StockMind` mainline.
- Consolidated the mainline repo decision, current working branch, local working-tree status, and the already backported `security_chair_resolution` capability slice.
- Added an audit summary of `D:\Rust\Stock`, including workspace shape, `.env.example`, live data-source wiring, adapter inventory, SQLite schema scope, and local runtime data artifacts.

### Why
- The previous handoff file focused on architecture boundaries, but it was not strong enough as an execution handoff for the next AI session.
- The user explicitly asked for both a usable handoff manual and a check on the Rust `Stock` project because it already connects some data.

### Remaining
- [ ] Decide whether the next delivery step should continue as another minimal backport into `D:\SM` or start a fresh capability branch directly on the standalone mainline.
- [ ] If future work needs market-data integration in `D:\SM`, define which pieces of `D:\Rust\Stock` stay reference-only and which pieces are safe for minimal reuse.

### Risks
- [ ] This round updated documentation only and did not rerun repository tests in `D:\SM`.
- [ ] `D:\Rust\Stock` was audited as a reference project, but no code was merged from it in this round.

### Closed
- AI handoff manual rewritten for `D:\SM`
- `D:\Rust\Stock` data-access audit captured into the handoff manual
- Task journal updated for downstream AI continuity

## 2026-04-17
### Modified
- 将 `D:\Rust\Stock\stock.db` 剪切到 `D:\SM\.stockmind_runtime\incoming\stock.db`，未执行复制。
- 将 `D:\Rust\Stock\infra\stock.db` 剪切到 `D:\SM\.stockmind_runtime\incoming\infra_stock.db`，未执行复制。
- 先完成 `D:\SM` 本地训练底座的数据归集，保留 `Stock` 现成 SQLite 数据作为后续行情/估值/静态信息抽取来源。

### Why
- 用户明确要求先复用已有数据库数据源，再继续下载补齐，避免从零抓全量数据。
- 用户明确要求使用“剪切”而不是“复制”，避免 `D:\Rust\Stock` 与 `D:\SM` 同时保留大体积 SQLite 文件占用磁盘空间。

### Remaining
- [ ] 从 `D:\SM\.stockmind_runtime\incoming\stock.db` 抽取可直接进入训练准备的历史行情与估值数据，并映射到 `StockMind` 正式 runtime 分库。
- [ ] 使用本地采集工具补齐公告、消息与其它未覆盖的数据族后，再进入训练阶段。

### Risks
- [ ] `D:\Rust\Stock\stock.db` 是单库多表结构，不能直接当作 `StockMind` 的 `stock_history.db` 正式运行时替代，需要后续做结构化导入。
- [ ] `market_sentiment`、`analysis_results` 等表覆盖度较低，不能默认作为正式训练主数据源。

### Closed
- 已确认 `D:\SM\.stockmind_runtime\incoming\stock.db` 存在，且原路径 `D:\Rust\Stock\stock.db` 已不存在。
- 已确认 `D:\SM\.stockmind_runtime\incoming\infra_stock.db` 存在，且原路径 `D:\Rust\Stock\infra\stock.db` 已不存在。

## 2026-04-17
### Modified
- `tests/import_stock_price_history_legacy_db_cli.rs`：新增 legacy SQLite 历史桥接的 CLI 回归测试，并补一条 `NULL` OHLC 脏行跳过回归，目的：先锁住“单库 `stocks_daily` -> 正式 `stock_history.db`”以及“坏行不中断整批导入”的正式合同。
- `src/ops/import_stock_price_history_legacy_db.rs`：新增 `import_stock_price_history_legacy_db`，把 legacy `stocks_daily` 行按 symbol 正规化后导入正式 `StockHistoryStore`，并在遇到 `NULL` OHLC 行时跳过并统计 `skipped_row_count`，目的：复用已剪切进 `D:\SM` 的旧库数据而不把整库直接硬顶成正式 runtime。
- `src/ops/stock.rs` / `src/ops/stock_data_pipeline.rs` / `src/tools/catalog.rs` / `src/tools/dispatcher.rs` / `src/tools/dispatcher/stock_ops.rs`：补齐新桥接工具的 stock 边界导出、data-pipeline 分组、tool catalog 与 dispatcher 路由，目的：让旧库导入能力成为正式可调用 Tool，而不是一次性脚本。
- 运行正式补数：
  - 已把 taxonomy 相关 `41` 个 equity symbol 的 legacy 价格历史导入 `D:\SM\.stockmind_runtime\stock_history.db`
  - 已用 `sync_stock_price_history` 补齐 `510300.SH`、`510880.SH`、`512070.SH`、`512800.SH`、`159755.SZ`、`159928.SZ` 六个 market/sector 代理 ETF
  - 已为 `41` 个 equity symbol 批量执行 `security_fundamental_history_live_backfill`
  - 已为 `41` 个 equity symbol 批量执行 `security_disclosure_history_live_backfill`

### Why
- 用户明确要求先复用已存在数据源，再继续下载补齐，而不是从零重新抓全量训练数据。
- `incoming/stock.db` 是单库多表，不能直接替代 `StockMind` 正式 runtime 分库，因此需要一个最小桥接导入层。
- 真实导入时暴露出 legacy 脏行含 `NULL` OHLC，如果不先修 importer，正式导入会在真实库上中断。

### Remaining
- [ ] 在当前数据已补齐的基础上，重新评估训练标签与“强势/弱势分桶”目标，决定是否继续沿 `direction_head` 修补，还是切到新的分桶训练合同。
- [ ] 评估是否需要把 corporate action / 其它未纳入当前训练主链的数据族并入正式 runtime，再进入下一轮训练。

### Risks
- [ ] `security_fundamental_history.db` 当前每个 equity symbol 只有最近 `4` 个报告期；如果后续分桶训练需要更厚的财报时间轴，仍需继续补多期历史。
- [ ] `security_disclosure_history.db` 当前每个 equity symbol 本轮批量抓取为最近 `60` 条公告；如后续要做更长周期的事件强弱分桶，可能仍需继续扩页。
- [ ] `159755.SZ` 自身上市较晚，价格历史起点为 `2021-06-24`，这不是补数失败，而是产品实际历史长度限制。
- [ ] 当前还没有把 corporate action 历史纳入本轮补数；如果后续标签或特征要处理分红送转除权影响，需要单独补齐。

### Closed
- 已通过 `cargo test --test import_stock_price_history_legacy_db_cli -- --nocapture`
- 已通过 `cargo test --test stock_training_data_coverage_audit_cli -- --nocapture`
- 已确认正式 runtime 当前规模：
  - `stock_history.db / stock_price_history = 61302`
  - `security_fundamental_history.db / security_fundamental_history = 164`
  - `security_disclosure_history.db / security_disclosure_history = 2460`
- 已确认 taxonomy 相关 `41` 个 equity symbol 当前价格历史最后日期均已补到 `2026-04-17`
- 
## 2026-04-18
### Modified
- Added `tests/security_corporate_action_backfill_cli.rs` and completed the red-green cycle for the new governed corporate-action backfill tool.
- Added `src/ops/security_corporate_action_backfill.rs` and wired it into `src/ops/stock.rs`, `src/ops/stock_data_pipeline.rs`, `src/tools/catalog.rs`, `src/tools/dispatcher.rs`, and `src/tools/dispatcher/stock_ops.rs`.
- Added focused taxonomy-coverage tests for `sync_template_resonance_factors` and `template_factor_definitions`.
- Expanded resonance template sync and bootstrap coverage from bank-only to include `broker`, `insurance`, `consumer`, `manufacturing_growth`, and `dividend_soe`.
- Bootstrapped resonance factor definitions and synced active taxonomy template factor series into `D:\SM\.stockmind_runtime\security_resonance.db`.

### Why
- Scheme C2 required formal governed capability before continuing full data completion.
- The current A-share taxonomy already used templates beyond `bank`, so bank-only resonance coverage would leave training and research-sidecar data incomplete.
- `signal_outcome` backfill depends on resonance definitions and factor series being present in formal runtime.

### Remaining
- [ ] Source and import real governed records into `D:\SM\.stockmind_runtime\security_corporate_action.db`.
- [ ] Source and import real governed records into `D:\SM\.stockmind_runtime\security_external_proxy.db`.
- [ ] Design and execute historical snapshot generation so `signal_outcome_research.db` can contain real forward outcomes instead of remaining empty on the latest date.
- [ ] Re-run training-readiness and feature-completeness checks after the remaining data families are filled.

### Risks
- [ ] The new corporate-action tool is verified by contract tests, but the runtime still lacks real imported corporate-action rows.
- [ ] `signal_outcome_research.db` is still empty because latest-date snapshots do not have future return windows to backfill.
- [ ] Some resonance factor series are shorter than equity price history because source ETFs such as `159755.SZ` or newly synced bond proxies have shorter available histories.

### Closed
- `security_corporate_action_backfill` is now a formal stock tool with catalog and dispatcher coverage.
- Active taxonomy resonance templates now have both sync recipes and bootstrap definitions.
- Formal runtime resonance DB is no longer empty (`factor_registry = 33`, `factor_series = 22731`).
- Focused verification and regression targets passed.

## 2026-04-18
### Modified
- 使用现有正式工具链继续补齐训练底座数据，不复制大库，直接往 `D:\SM\.stockmind_runtime` 正式 runtime 回填真实数据。
- 通过 `akshare.stock_dividend_cninfo` 抓取 taxonomy 41 只股票的历史分红送转，生成正式 backfill 批次并写入 `D:\SM\.stockmind_runtime\security_corporate_action.db`。
- 基于已有 `D:\SM\.stockmind_runtime\stock_history.db` 为 41 只股票派生股价相对行业 ETF 的 5 日强弱与行业 ETF 5 日量能代理，生成正式 CSV 批次并写入 `D:\SM\.stockmind_runtime\security_external_proxy.db`。
- 追加更新 `task_plan.md`、`findings.md`、`progress.md`，把本轮真实补数结果和剩余训练缺口同步到项目上下文。

### Why
- 用户要求先把数据和指标尽量补全，再进入下一轮模型训练和方法切换。
- 当前正式 runtime 中最明显的两个空洞就是 `security_corporate_action.db` 和 `security_external_proxy.db`，如果不先补齐，后面的强弱分桶训练仍然容易反复回头补基础层。

### Remaining
- [ ] 评估是否需要把 `signal_outcome_research.db` 从当前月频历史样本继续加密到周频或更高密度，以支持更稳定的强弱分桶训练。
- [ ] 评估是否需要继续补 `external_proxy` 的宏观/利率/汇率等字段，而不只保留当前第一批 equity 代理字段。
- [ ] 如下一轮要正式开训，补一份“训练数据 + 指标完整度”总审计结果，避免只看单库行数。

### Risks
- [ ] `security_external_proxy.db` 当前虽然已不为空，但仅覆盖 equity 场景下最先可解释的 4 个代理字段，不能等同于全量宏观代理全部就绪。
- [ ] `signal_outcome_research.db` 目前仍以月频历史快照为主，若后续直接做更细粒度分桶，样本密度可能仍然不够。
- [ ] 这轮是数据补数，没有新增代码验证集；真实性主要靠正式 tool 写入结果和 SQLite 落库核对。

### Closed
- `security_corporate_action.db` 已补齐真实记录：`964` 条，覆盖 `41` 个 symbol。
- `security_external_proxy.db` 已补齐真实记录：`51480` 条，覆盖 `41` 个 symbol。
- 真实补数批次文件已留存在 `D:\SM\.stockmind_runtime\generated_backfills\` 目录，后续可复查和重跑。
### Why
- The user approved the direction that bank training should use hierarchical inheritance while high-dividend / high-bonus remains a horizontal overlay instead of a competing main chain.
- The next implementation slice must stay auditable and phase-based so each data completion step is followed by one real training round before continuing.
- Freezing the design and task order first reduces the risk of mixing B-stage label repair with later A-stage pool expansion and overlay work.

### Remaining
- [ ] Get explicit approval to start Task 1 of the plan and enter the red-green cycle for dividend-aware forward-outcome labels.
- [ ] Decide whether B-stage real training should widen the train window back to `2020-01-01` where symbol coverage permits, or keep a narrower governed window for the first repaired-label run.
- [ ] Confirm the governed all-A-share bank list before A1 implementation if any bank membership edge cases appear during taxonomy expansion.

### Risks
- [ ] The new design is frozen, but no implementation code has started yet.
- [ ] Some terminals still display Chinese markdown as garbled text even when the saved file is UTF-8, so file-content validation should rely on editor or downstream reads if needed.
- [ ] Overlay definitions may still need one more threshold discussion once the bank parent and child results are visible.

### Closed
- The bank main-chain and high-dividend overlay relationship is now formalized as `main inheritance + horizontal overlay`, not true multi-parent model inheritance.
- The implementation order is now locked as `B -> A1 -> A2 -> Overlay`.
- Both design and execution-plan documents are stored under `D:\SM\docs\plans\`.
- Both design and execution-plan documents are stored under `D:\SM\docs\plans\`.

## 2026-04-18
### Modified
- Added three new corporate-action regression tests to `D:\SM\tests\security_forward_outcome_cli.rs` covering:
  - cash dividend total return
  - bonus-ratio share-count uplift
  - combined cash dividend plus bonus compounding
- Added one new benchmark-relative regression to `D:\SM\tests\security_scorecard_training_cli.rs` proving dividend-aware subject labels can create strong samples that would otherwise be neutral.
- Updated `D:\SM\src\ops\security_forward_outcome.rs` so forward-return labels now:
  - read governed rows from `security_corporate_action.db`
  - accumulate `cash_dividend_per_share`
  - apply `bonus_ratio` and `split_ratio` through a share-adjustment factor
  - preserve the legacy adj-close path when no effective corporate action exists

### Why
- Scheme B starts with repairing label truth before running the next real `dividend_soe` training round.
- The earlier label path ignored governed dividend and bonus events, which understated shareholder-return-heavy names and caused benchmark-relative bucket sampling to drop valid strong cases.
- The new regression layer was needed so later training changes cannot silently fall back to close-to-close labels again.

### Remaining
- [ ] Run the real B-stage `dividend_soe` training round on formal runtime with the repaired label path.
- [ ] Record artifact, registry, diagnostics, and split metrics for that real run.
- [ ] Review whether the first repaired-label run should widen its train window farther back toward `2020-01-01`.

### Risks
- [ ] `max_drawdown` and `max_runup` still follow the older price-path contract; this round only repaired `forward_return`.
- [ ] Benchmark-side return still uses the plain benchmark price path; ETF distribution handling was not expanded in this slice.
- [ ] The new training regression depends on dense fixture dividends across the date window, so future edits to sampling cadence may require fixture maintenance.

### Closed
- TDD red-green cycle completed for dividend-aware subject forward-return labels.
- Focused and adjacent verification passed:
  - `cargo test --test security_forward_outcome_cli corporate_action -- --nocapture`
  - `cargo test --test security_scorecard_training_cli relative_benchmark -- --nocapture`
  - `cargo test --test security_forward_outcome_cli -- --nocapture`
  - `cargo test --test security_scorecard_training_cli -- --nocapture`
  - `cargo test --test security_scorecard_refit_cli -- --nocapture`

## 2026-04-18
### Modified
- Executed the real `A2` governed child-pool training round for `state-owned-major-bank-child.v1` on the same formal runtime root and the same date windows used by `A1`.
- Updated `D:\SM\task_plan.md`, `D:\SM\findings.md`, and `D:\SM\progress.md` with the bank parent-vs-child audit results, artifact paths, and next-step recommendation.
- Recorded that the first `A2` real run used the wrong runtime root and was explicitly discarded from the comparison baseline.

### Why
- The approved execution order already moved from `B` into `A1 -> A2`, so the next required step was a real governed child-pool training round rather than more design work.
- The parent-pool result was already weak, so we needed to verify whether `bank -> state-owned-major-bank` inheritance materially improved stability before deciding whether to continue into `Overlay`.
- Using the same runtime root and the same windows as `A1` was necessary so the parent-vs-child comparison stayed auditable.

### Remaining
- [ ] Backfill the added 30 bank symbols into the missing governed families and retrain `bank-parent.v1`.
- [ ] Re-check whether the added 30 bank symbols still need more formal `fundamental/disclosure` coverage after the governed-family audit.
- [ ] Decide whether to start `Overlay` only after the next parent retrain, or to hold it until the label-shift problem is reduced.

### Risks
- [ ] `A2` remains `production_readiness = blocked` even though `valid_accuracy` and walk-forward accuracy improved.

## 2026-04-21
### Modified
- Narrowed the Nikkei `future_*_bucket_head` training contract in `D:\SM\src\ops\security_scorecard_training.rs` behind a request-aware Scheme B selector.
- Removed these direct training features for Nikkei 10D future-bucket heads:
  - `market_regime`
  - `risk_note_count`
  - `breakout_signal`
  - `momentum_signal`
  - `mfi_14`
- Replaced broad breakout / momentum shortcuts with narrower technical fields:
  - added `breakout_direction`
  - added `breakout_stage`
  - kept `flow_status`
  - kept `volume_ratio_20`
  - kept `macd_histogram`
  - kept `rsi_14`
- Added and passed focused contract coverage in `D:\SM\tests\security_scorecard_training_cli.rs` and the training unit tests for the Nikkei future-bucket selector.
- Re-ran the governed Nikkei 10D future-bucket training heads and persisted new Scheme B artifacts, diagnostics, registries, and replay comparison files under `D:\SM\.stockmind_runtime\`.

### Why
- The prior Nikkei 10D future-bucket runs let broad regime / event-density / slow structural tags dominate the decision path, which matched the user's diagnosis that several top-level features were leading the model away from the real oversold-to-10D technical path.
- Scheme B was approved to keep the existing label contract unchanged while narrowing the technical feature contract so replay and attribution would become more interpretable.
- This round was intended to test whether removing the obviously biasing upper-layer fields could materially improve the Nikkei 10D training and oversold replay behavior.

### Remaining
- [ ] Decide whether the next Nikkei iteration should continue to compress the feature family further around a smaller `Q/V/T` subset, or whether the `future_down` head itself should stop being the main decision head.
- [ ] Review whether `flow_status` should also be downgraded or rebuilt, because it is currently retained but shows near-zero variance on this slice.
- [ ] If we continue on Nikkei, define the next approved contract before implementation so the next training round does not drift into ad hoc feature expansion again.

### Risks
- [ ] Scheme B successfully narrowed the feature contract, but training quality did not improve overall: `future_down` walk-forward accuracy fell from `0.7273` to `0.6061`, `future_neutral` stayed `0.0`, and `future_up` fell from `0.4242` to `0.3030`.
- [ ] All three Scheme B heads remain `production_readiness = blocked`, mainly because the slice is still extremely sparse (`46` samples, `35` features).
- [ ] The mainline oversold replay improved only slightly on the same `23` oversold dates: `future_down_bullish_transform` Pearson moved from `-0.0314` to `0.0592`, and threshold hit rate at `0.5` moved from `0.4706` to `0.5294`, which is directionally better but still weak.
- [ ] This round verified focused tests and the saved training artifacts, but it did not rerun a broader repository test suite.

### Closed
- Scheme B feature selector is now active for Nikkei `future_{down,neutral,up}_bucket_head`.
- The approved label path remains unchanged as `T0 oversold -> T+10 grouped future bucket`.
- Focused verification passed:
  - `cargo test training_feature_configs_for_nikkei_future_bucket_heads_apply_scheme_b_contract -- --nocapture`
  - `cargo test --test security_scorecard_training_cli security_scorecard_training_supports_future_neutral_bucket_head_for_oversold_nikkei_starts -- --nocapture`
- New governed outputs were confirmed in:
  - `D:\SM\.stockmind_runtime\nikkei_future_down_training_scheme_b\`
  - `D:\SM\.stockmind_runtime\nikkei_future_neutral_training_scheme_b\`
  - `D:\SM\.stockmind_runtime\nikkei_future_up_training_scheme_b\`
  - `D:\SM\.stockmind_runtime\analysis\nikkei_future_down_scheme_b_replay_oversold_2026_01_01_to_2026_04_20.json`
- [ ] The child-pool OOT test split collapsed into a one-sided label regime with `test positive_rate = 0.0`, so `test_accuracy = 0.0250` cannot support production sign-off.
- [ ] `A1` is still contaminated by incomplete governed-family coverage for the newly added 30 banks, so the parent result should be retrained after the next补数 round before drawing a long-term conclusion.

### Closed
- Real governed `A2` training completed on `D:\SM\.stockmind_runtime`.
- Verified real training by running:
  - `cargo run --quiet` with `tool = security_scorecard_training`
  - `training_contract_id = state-owned-major-bank-child.v1`
  - `train = 2021-01-04..2025-06-30`
  - `valid = 2025-07-01..2025-12-31`
  - `test = 2026-01-01..2026-01-20`
- Captured the final accepted `A2` outputs:
  - `sample_count = 166`
  - `train = 90`
  - `valid = 36`
  - `test = 40`
  - `valid_accuracy = 0.5000`
  - `test_accuracy = 0.0250`
  - `mean_walk_forward_accuracy = 0.7398`

## 2026-04-19
### Modified
- Reused the formal live history tools to close the remaining bank data gaps in `D:\SM\.stockmind_runtime` instead of adding a new ad-hoc importer.
- Probed `601658.SH` first and confirmed both `security_fundamental_history_live_backfill` and `security_disclosure_history_live_backfill` can fetch and persist real governed rows on the shared runtime root.
- Batch-backfilled the remaining missing bank `fundamental` coverage and saved the batch summary to:
  - `D:\SM\.stockmind_runtime\generated_backfills\security_fundamental_history_live_backfill_bank_missing30_summary.json`
- Batch-backfilled the remaining missing bank `disclosure` coverage and saved the batch summary to:
  - `D:\SM\.stockmind_runtime\generated_backfills\security_disclosure_history_live_backfill_bank_missing30_summary.json`
- Re-ran the real governed `bank-parent.v1` training round after the 42-bank pool became fully covered across `fundamental / disclosure / corporate_action / external_proxy`.
- Updated `D:\SM\task_plan.md`, `D:\SM\findings.md`, and `D:\SM\progress.md` with the full-data bank audit and the refreshed `A1` result.

### Why
- The user explicitly asked to continue data completion first and only then judge whether the main problem had shifted from missing data to the training method itself.
- The previous retrain had already closed `corporate_action` and `external_proxy`, but the added 30 bank symbols still lacked governed `fundamental` and `disclosure` history.
- A clean method diagnosis needed one retrain on a fully completed 42-bank governed foundation instead of continuing to infer from partially completed bank features.

### Remaining
- [ ] Run a dedicated training-method diagnosis slice before starting `Overlay`.
- [ ] Decide whether to remove or merge structurally redundant features such as `fundamental_status` and `data_gap_count`.
- [ ] Decide whether the current OOT window and benchmark-relative split geometry should be widened or redesigned before the next bank retrain.

### Risks
- [ ] The refreshed full-data `A1` run is still `production_readiness = blocked`.
- [ ] Full bank data completion did not improve `valid_accuracy` or `test_accuracy`, so the next bottleneck is likely methodological rather than another simple bank-family gap.
- [ ] This round focused on real data backfill and runtime retraining; it did not add new code changes or new regression targets.

### Closed
- Bank `fundamental` coverage is now `42 / 42`.
- Bank `disclosure` coverage is now `42 / 42`.
- The four current governed bank training families are now all complete at `42 / 42`.
- The refreshed full-data `bank-parent.v1` result has been captured and compared against the prior retrain baseline.

## 2026-04-18
### Modified
- Added one regression test on `tests/security_feature_snapshot_cli.rs` to prove that historical snapshots must keep `fundamental/disclosure` unavailable when governed history is missing, even if live mock providers can still return payloads.
- Updated `src/ops/security_analysis_fullstack.rs` so requests with `as_of_date` no longer fall back to live `fundamental/disclosure` providers after governed-history misses.
- Updated `D:\SM\task_plan.md`, `D:\SM\progress.md`, and `D:\SM\findings.md` with the leak diagnosis, repair boundary, and verification evidence.

### Why
- The training audit exposed a high-severity historical information leakage bug: past-dated samples were still consuming live information payloads when governed `fundamental/disclosure` history had not been backfilled yet.
- This leakage directly threatens label/feature integrity, so it had to be fixed before using the next training result as trustworthy evidence.

### Remaining
- [ ] Re-run at least one bank scorecard training / audit slice on top of the repaired historical information contract.
- [ ] Confirm whether any already-produced training artifacts should be marked as suspect because they were generated before this leakage fix.

### Risks
- [ ] The current repair treats any request with `as_of_date` as a replay-style request; if a future product flow expects dated-but-live enrichment, that flow will need an explicit contract instead of implicit fallback.
- [ ] This round verified the snapshot path thoroughly, but it did not yet rerun the downstream bank training artifacts after the fix.

### Closed
- Historical `fundamental/disclosure` live fallback is now blocked on the `security_feature_snapshot` replay path when governed history is missing.
- Fresh verification passed:
  - `cargo test --test security_feature_snapshot_cli security_feature_snapshot_keeps_historical_information_unavailable_when_governed_history_is_missing -- --exact --nocapture`
  - `cargo test --test security_feature_snapshot_cli -- --nocapture`

## 2026-04-18
### Modified
- Re-ran one real `bank-parent.v1` training / audit slice after the historical information leakage repair, using the unchanged parent windows:
  - `2021-01-04..2025-06-30`
  - `2025-07-01..2025-12-31`
  - `2026-01-01..2026-01-20`
- Captured the new artifact / registry / diagnostics:
  - `D:\SM\.stockmind_runtime\scorecard_artifacts\a_share_equity_10d_direction_head__candidate_2026_04_18T11_40_00_08_00.json`
  - `D:\SM\.stockmind_runtime\scorecard_model_registry\a_share_equity_10d_direction_head__candidate_2026_04_18T11_40_00_08_00.json`
  - `D:\SM\.stockmind_runtime\scorecard_training_diagnostics\a_share_equity_10d_direction_head__candidate_2026_04_18T11_40_00_08_00.json`
- Updated `D:\SM\task_plan.md`, `D:\SM\progress.md`, and `D:\SM\findings.md` with the post-fix rerun result and the new diagnosis boundary.

### Why
- After the historical information leak was repaired, the next model result had to be regenerated before it could be trusted as evidence.
- The user explicitly asked to run it again and verify whether the information surface is now aligned.

### Remaining
- [ ] Decide whether to first prune / merge the new `disclosure_status + announcement_count + event_density_bucket + data_gap_count` redundancy cluster.
- [ ] Decide whether earlier-time governed information coverage should be thickened further before the next parent rerun.
- [ ] Decide whether to delay `Overlay` until the current parent model stops amplifying information-surface redundancy.

### Risks
- [ ] The post-fix rerun is still `production_readiness = blocked`.
- [ ] `high_correlation_pair_count` rose from `2` to `9`, so the repaired information surface is now exposing a stronger event-feature coupling problem.
- [ ] `valid_accuracy` still did not improve, even though `test_accuracy` and walk-forward accuracy ticked up slightly.

### Closed
- One real parent rerun now exists on the repaired historical information contract.
- The stricter historical information contract did not reduce the bank parent sample base: the rerun stayed at `1015` samples with `604 / 236 / 175` splits.
- Fresh execution verification passed:
  - `cargo run --quiet` with `tool = security_scorecard_training`
  - `training_contract_id = bank-parent.v1`
  - `training_runtime_root = D:\SM\.stockmind_runtime`

## 2026-04-18
### Modified
- Added `D:\SM\docs\plans\2026-04-18-interleaved-training-split-plan.md` to document the approved shift from regime-heavy sequential splits to interleaved market-calendar sampling.
- Updated `D:\SM\src\ops\security_scorecard_training.rs` so the benchmark-relative bucket training path now:
  - resolves one shared market-calendar anchor,
  - builds a combined training span,
  - assigns dates by `20` trading-day interleaved blocks,
  - inserts governed trading-day purge gaps,
  - intersects those split dates with each symbol's local qualified history before sampling.
- Added unit regressions in `D:\SM\src\ops\security_scorecard_training.rs` to freeze the new `train -> valid -> train -> test -> train` block rotation and the purge-gap behavior.

### Why
- The user explicitly approved replacing the old year-style `2-1-1` split geometry because it can overfit a single bull/bear regime and bias validation conclusions.
- The benchmark-relative bank training line needed a safer split contract that still preserves the existing legacy absolute-label path and current regression fixtures.

### Remaining
- [ ] Re-run one real bank benchmark-relative training slice on the new interleaved split contract and inspect whether the diagnostic stability improves.
- [ ] Decide whether the current governed purge default should stay at the bounded half-horizon rule or become contract-configurable in a later round.
- [ ] Decide whether the same interleaved split contract should be extended from the benchmark-relative bank line to other future training families.

### Risks
- [ ] This round changes the benchmark-relative split geometry only inside the trainer; downstream diagnostics still display the original configured window strings rather than the derived interleaved block map.
- [ ] The current governed purge default is a bounded compromise for sample efficiency, not a mathematically exhaustive leakage barrier for every horizon.
- [ ] The working tree already contained many unrelated runtime and research artifacts before this change, so follow-up staging still needs selective review.

### Closed
- Fresh verification passed:
  - `cargo test relative_benchmark_split_ -- --nocapture`
  - `cargo test security_scorecard_training::tests -- --nocapture`
  - `cargo test --test security_scorecard_training_cli security_scorecard_training_supports_relative_benchmark_bucket_labels_with_denser_sampling -- --nocapture`
  - `cargo test --test security_scorecard_training_cli -- --nocapture`

## 2026-04-19
### Modified
- Updated `D:\SM\src\ops\security_forward_outcome.rs` to expose one shared governed forward-return helper so training can reuse dividend-aware / split-aware return math without replaying the full snapshot stack.
- Updated `D:\SM\src\ops\security_decision_evidence_bundle.rs` and `D:\SM\src\ops\security_feature_snapshot.rs` to derive and persist:
  - `market_cycle_status`
  - `market_fund_flow_status`
  - `sector_fund_flow_status`
  - `market_risk_appetite_status`
- Updated `D:\SM\src\ops\security_scorecard_training.rs` so:
  - `1d` relative labels now use cross-sectional top/bottom `20%` buckets,
  - the `1d` relative benchmark resolves to the sector benchmark first,
  - the new market-state fields enter the training feature contract,
  - tail dates without future rows are skipped instead of aborting the whole run.
- Added / updated regressions in:
  - `D:\SM\src\ops\security_scorecard_training.rs`
  - `D:\SM\tests\security_feature_snapshot_cli.rs`
  - `D:\SM\tests\security_scorecard_training_cli.rs`
- Re-ran one real `bank-parent.v1` `1d` training round and produced:
  - `D:\SM\.stockmind_runtime\scorecard_model_registry\a_share_equity_1d_direction_head__candidate_2026_04_19T00_12_00_08_00.json`
  - `D:\SM\.stockmind_runtime\scorecard_training_diagnostics\a_share_equity_1d_direction_head__candidate_2026_04_19T00_12_00_08_00.json`

### Why
- The user approved moving the `1d` objective from sparse fixed-threshold prediction to denser cross-sectional strong/weak buckets.
- The user also asked to make bull/bear and market / sector flow context explicit inside the training chain.
- The first real `1d` rerun exposed a boundary bug where one symbol without `T+1` rows aborted the full retraining run.

### Remaining
- [ ] Decide whether `10d` should also migrate from fixed threshold to a cross-sectional bucket contract, or stay as the current excess-threshold baseline for comparison.
- [ ] Decide whether the new market / sector flow proxies are strong enough for production, or whether a richer market-wide capital-flow source should be backfilled next.
- [ ] Decide whether walk-forward folding should be further hardened, because the current two-fold `1d` result is usable for comparison but still thin.

### Risks
- [ ] The real `1d` rerun still ended with `production_readiness = blocked`.
- [ ] `valid_accuracy` is still weak at roughly `0.43`, so the new label / state contract fixed sparsity but did not fully fix generalization.
- [ ] `high_correlation_pair_count = 7`, which means the thicker state/event feature surface still has redundancy pressure.

### Closed
- The `1d` relative benchmark training path is now sector-relative, cross-sectional, and sample-dense enough to run end-to-end.
- Real rerun no longer aborts on a tail date without future rows.
- Fresh verification passed:
  - `cargo test --test security_scorecard_training_cli -- --nocapture`
  - `cargo test --test security_feature_snapshot_cli -- --nocapture`
  - `cargo test security_scorecard_training::tests -- --nocapture`
  - real `bank-parent.v1` `1d` rerun via `target\debug\excel_skill.exe`

## 2026-04-18
### Modified
- Added `D:\SM\docs\plans\2026-04-18-layered-market-sector-stock-design.md` to freeze the approved shift from one-step stock-vs-HS300 prediction to a layered market -> sector -> stock design.
- Added `D:\SM\docs\plans\2026-04-18-layered-market-sector-stock-prediction.md` to define the first executable implementation plan for the layered return-composition path, including probability and `10000` principal earnings outputs.

### Why
- The user approved stopping the current single-layer model line and requested a direct design for a more business-aligned prediction framework.
- The latest audit showed the current bank-heavy benchmark-relative model is not tradable, so the next step had to be a frozen design + execution plan instead of more ad-hoc retraining.

### Remaining
- [ ] Confirm the first implementation slice should stay bank-only before widening to other sectors.
- [ ] Confirm the exact first-phase market / sector anchor symbols and whether any board-level breadth fields need to be added before coding starts.
- [ ] Start implementation from the new layered plan after the user chooses the execution mode.

### Risks
- [ ] The new design is approved but not implemented yet, so all current production/runtime behavior still reflects the old single-layer path.
- [ ] The layered plan increases orchestration complexity; if one layer is weak, the final composed return can still drift unless calibration is verified separately.

### Closed
- The approved layered design is now frozen in repo docs and can be used as the implementation baseline for the next coding round.
- The implementation plan now exists with explicit files, tests, and verification commands, so follow-up work no longer needs to re-derive the high-level architecture.
## 2026-04-18
### Modified
- Updated `D:\SM\src\ops\security_scorecard_training.rs` to add the first formal `security_forward_outcome.layered_market_sector.v1` training path.
- Added three governed layered heads to the training request contract:
  - `market_return_head`
  - `sector_excess_head`
  - `stock_excess_vs_sector_head`
- Added shared layered forward-return decomposition inside training so labels can be built from:
  - market forward return
  - sector excess return versus market
  - stock excess return versus sector
- Added layered request / label regressions in `D:\SM\src\ops\security_scorecard_training.rs`.
- Added end-to-end CLI regression coverage in `D:\SM\tests\security_scorecard_training_cli.rs` for the three new layered heads.
- Updated `D:\SM\tests\security_composite_scorecard_unit.rs` to keep the expanded `SecurityMasterScorecardDocument` fixture compiling after `layered_return_summary` became part of the contract.

### Why
- The user approved starting implementation of the layered `market -> sector -> stock` path and asked to continue coding directly.
- The previous scorecard line had already been judged not tradable, so the next safe step was to open a new layered label family without breaking the existing relative-benchmark route.
- The repository also needed one small fixture repair so broader verification could compile after the earlier scorecard contract expansion.

### Remaining
- [ ] Train and persist three real layered runtime artifacts on the formal bank universe instead of stopping at fixture-level contract coverage.
- [ ] Connect the layered prediction-side composer to real per-head artifacts so `layered_return_summary` is no longer a placeholder in prediction mode.
- [ ] Decide whether the layered bank path should use dedicated governed `training_contract_id` values in addition to the new label family.

### Risks
- [ ] The new layered path currently trains three binary heads, but the prediction-side composition is not yet reading real layered model outputs.
- [ ] Layered labels now depend on both market and sector anchors having aligned future rows; sparse anchors near range tails can still reduce sample coverage.
- [ ] This round verified focused suites only, not the full repository test matrix.

### Closed
- Layered training request validation now accepts the first three governed layered heads.
- Layered positive-label definitions are now explicit and target-specific.
- Fresh focused verification passed:
  - `cargo test --lib layered_market_sector_label_family_ -- --nocapture`
  - `cargo test --test security_scorecard_training_cli -- --nocapture`
  - `cargo test --test security_feature_snapshot_cli -- --nocapture`
  - `cargo test --test security_master_scorecard_cli -- --nocapture`
  - `cargo test --test security_composite_scorecard_unit -- --nocapture`

## 2026-04-19
### Modified
- Executed the first real 42-bank layered three-head training batch on `D:\SM\.stockmind_runtime` using:
  - `label_definition_version = security_forward_outcome.layered_market_sector.v1`
  - `market_symbol = 510300.SH`
  - `sector_symbol = 512800.SH`
  - `market_profile = a_share_core_v1`
  - `sector_profile = a_share_bank`
- Produced and audited three formal layered artifacts:
  - `a_share_equity_10d_market_return_head__candidate_2026_04_18T23_58_00_08_00`
  - `a_share_equity_10d_sector_excess_head__candidate_2026_04_18T23_59_00_08_00`
  - `a_share_equity_10d_stock_excess_vs_sector_head__candidate_2026_04_19T00_00_00_08_00`
- Updated `D:\SM\findings.md`, `D:\SM\progress.md`, and `D:\SM\task_plan.md` with:
  - artifact / registry / diagnostics paths
  - shared sample coverage
  - per-head positive-rate drift
  - per-head valid/test accuracy
  - the current failure ordering across the three-layer stack

### Why
- The user approved moving from implementation into one real bank-only layered training round on the formal runtime.
- The first real run was needed before touching prediction composition so we could identify which layer fails first on governed data rather than guessing from fixture tests.
- Capturing the runtime outputs in repo records keeps the next prediction-side implementation slice auditable.

### Remaining
- [ ] Connect `security_scorecard` / `security_master_scorecard` prediction mode to these three real layered artifacts.
- [ ] Decide whether to repair the sector layer first through feature pruning, label redesign, or split-geometry changes.
- [ ] Add one end-to-end prediction regression that composes the three real heads into the formal layered return summary.

### Risks
- [ ] All three layered heads remain `production_readiness = blocked`.
- [ ] The sector layer currently shows the heaviest `label_distribution_shift_is_large` signal and the largest correlation pressure.
- [ ] This round executed real training and documentation updates, but did not yet change the prediction-side layered placeholder logic.

### Closed
- The first formal bank-only layered baseline now exists on the governed runtime.
- The current failure ordering is clearer: `sector_excess_head` is the first unstable layer.
- Fresh execution evidence was captured from real runtime training outputs rather than fixture-only regressions.

## 2026-04-19
### Modified
- Updated `D:\SM\src\ops\security_master_scorecard.rs` so prediction mode now accepts three explicit layered artifact paths:
  - `layered_market_return_head_model_path`
  - `layered_sector_excess_head_model_path`
  - `layered_stock_excess_vs_sector_head_model_path`
- Added layered prediction artifact loading and validation on the master-scorecard prediction branch.
- Updated prediction-mode layered composition so it now uses the real three-head values instead of the placeholder `0 / 0 / expected_return` fallback.
- Updated `D:\SM\src\ops\security_decision_submit_approval.rs` to keep the new request contract compiling with explicit `None` values for the layered prediction fields.
- Added a new CLI regression in `D:\SM\tests\security_master_scorecard_cli.rs` that proves prediction mode consumes three real layered artifacts end to end.

### Why
- The previous step already produced three real layered bank artifacts, so the next approved move was to connect prediction mode to those formal outputs.
- Leaving the layered summary on placeholder math would make the new training line impossible to validate on real symbol/date predictions.
- The approval path also needed a small constructor repair because the request contract expanded.

### Remaining
- [ ] Run one real symbol/date prediction report with the real bank layered artifacts and inspect whether the composed output is business-readable.
- [ ] Decide whether the sector layer should be repaired before using the composed prediction for broader bank screening.
- [ ] Add a stronger end-to-end verification slice that reads real runtime artifacts instead of fixture artifacts for prediction-mode regression.

### Risks
- [ ] Prediction-mode `beat_market_probability` is still derived from a deterministic heuristic over composed excess return, not from a dedicated layered classification head.
- [ ] The current layered profit range still uses the earlier lightweight `+/- 0.02` spread and is not yet calibrated from per-head uncertainty.
- [ ] This round integrated prediction mode only; replay and fallback branches were intentionally left on their existing behavior.

### Closed
- Prediction-mode layered summary now reads real three-head artifact values when all three paths are provided.
- The top-line expected return now stays aligned with the layered composed stock return in prediction mode.
- Fresh verification passed:
  - `cargo test --test security_master_scorecard_cli -- --nocapture`
  - `cargo test --test security_scorecard_cli -- --nocapture`

## 2026-04-18
### Modified
- Audited the repository for post-selection position-management logic instead of only stock-picking logic.
- Read and correlated the main position-management modules:
  - `D:\SM\src\ops\security_position_plan.rs`
  - `D:\SM\src\ops\security_portfolio_position_plan.rs`
  - `D:\SM\src\ops\security_execution_journal.rs`
  - `D:\SM\src\ops\security_execution_record.rs`
  - `D:\SM\src\ops\security_account_open_position_snapshot.rs`
  - `D:\SM\src\ops\security_account_open_position_snapshot_assembler.rs`
  - `D:\SM\src\ops\security_record_position_adjustment.rs`
  - `D:\SM\src\runtime\security_execution_store_schema.rs`
- Updated `D:\SM\task_plan.md`, `D:\SM\findings.md`, and `D:\SM\progress.md` with the investigation trace and conclusions.

### Why
- The user asked whether the project already contains complete holding-management logic beyond stock selection, and how that logic currently works.
- This required separating plan generation, portfolio allocation, runtime persistence, and autonomous trigger execution into different layers.
- Recording the audit result makes the next implementation step focus on the real missing layer instead of repeating the same repository scan.

### Remaining
- [ ] Add a real trigger-evaluation engine that compares current market data against add/reduce/stop/take-profit thresholds and emits governed actions automatically.
- [ ] Connect runtime open-position snapshots back into an active rebalance loop instead of only using them for reconstruction and reporting.
- [ ] Decide whether automatic position actions should stay advisory-first or be allowed to create execution-ready adjustment events directly.

### Risks
- [ ] Current position-management coverage is strong on planning, allocation, persistence, and review, but still weak on autonomous trigger execution.
- [ ] Some execution thresholds in the composite adapter are explicitly placeholder values and are not yet indicator-derived production logic.
- [ ] Repository-wide search in this environment could not use `rg.exe` because the executable returned `Access is denied`, so the audit relied on PowerShell-native search.

### Closed
- The repository already contains real position-management logic, not just stock-picking logic.
- The current implemented chain is: single-name plan -> portfolio allocation suggestion -> execution record/journal persistence -> open-position reconstruction -> adjustment-event recording/review.
- The missing layer is an automatic live decision engine that continuously turns thresholds into actual add/reduce/exit actions.
## 2026-04-19
### Modified
- Updated `D:\SM\src\ops\security_scorecard.rs` to extend the model artifact contract with `prediction_calibration` and a shared `predict_numeric_head_value(...)` decoder for regression plus calibrated direction heads.
- Updated `D:\SM\src\ops\security_scorecard_training.rs` so layered training samples now retain numeric `target_value`, and layered artifacts now persist `direction_probability_calibrated_return` plus baseline / positive / negative expected-return anchors.
- Updated `D:\SM\src\ops\security_master_scorecard.rs` so layered prediction loading now accepts calibrated direction artifacts instead of regression-only artifacts.
- Added a new calibrated-layered prediction regression in `D:\SM\tests\security_master_scorecard_cli.rs` and kept the existing layered training regression green.
- Re-trained three real bank-layer artifacts on `D:\SM\.stockmind_runtime` and re-ran the live example for `601916.SH / 2026-04-01 / 10d` with the new calibrated artifacts.

### Why
- The previous implementation could read three layered artifact paths, but real runtime artifacts still returned all-zero layered predictions because they were direction classifiers without numeric calibration.
- The user asked to keep iterating on real training plus real validation instead of stopping at fixture-only integration.
- The smallest stable bridge was to keep the existing classifier training flow and add governed probability-to-return calibration inside the artifact contract.

### Remaining
- [ ] Decide whether the current piecewise-linear calibration should evolve into bucket-level or isotonic calibration after we collect more real validation slices.
- [ ] Re-run the same calibrated layered flow on more bank symbols and more dates, then compare the error concentration by market / sector / stock layer.
- [ ] Decide whether `beat_market_probability` should later come from a dedicated layered classifier instead of the current return-derived heuristic.

### Risks
- [ ] The new calibration currently uses train-split averages, so it can still under-estimate strong market legs when regime shift is large.
- [ ] `security_master_scorecard` replay mode for the old example did not automatically surface the realized layered decomposition, so actual-vs-predicted comparison still relied on direct `security_forward_outcome` queries for market / sector / stock legs.
- [ ] The first real calibrated example for `601916.SH / 2026-04-01 / 10d` produced the correct positive sign but still materially under-estimated the realized 10d market return.

### Closed
- Real layered bank artifacts now deserialize with `prediction_mode = direction_probability_calibrated_return` and non-empty `prediction_calibration` payloads.
- Prediction-mode layered summary now emits non-zero numeric returns from real calibrated artifacts instead of collapsing to zero.
- Fresh verification passed:
  - `cargo test --test security_scorecard_training_cli security_scorecard_training_supports_layered_market_sector_stock_target_heads -- --exact --nocapture`
  - `cargo test --test security_master_scorecard_cli security_master_scorecard_prediction_mode_supports_calibrated_layered_head_artifacts -- --exact --nocapture`
  - `cargo test --test security_master_scorecard_cli -- --nocapture`
  - `cargo test --test security_scorecard_cli -- --nocapture`
## 2026-04-19
### Modified
- Updated `D:\SM\src\ops\security_master_scorecard.rs` to add two new formal outputs:
  - `layered_prediction_replay_comparison`
  - `pipeline_payload`
- Extended replay mode so when three layered artifacts are provided on a historical date, the tool now compares predicted market / sector / stock layered returns against realized replay returns inside the same formal master-scorecard document.
- Extended prediction mode and replay mode with one normalized pipeline payload for downstream position-management orchestration, including:
  - `pipeline_stage`
  - `allocation_signal`
  - `conviction_score`
  - `risk_adjustment_hint`
  - `position_management_ready`
  - validation-related optional fields
- Added new regressions in `D:\SM\tests\security_master_scorecard_cli.rs` to lock:
  - replay-mode layered prediction vs realized replay comparison
  - prediction-mode pipeline payload contract
- Added implementation plan file `D:\SM\docs\plans\2026-04-19-master-scorecard-replay-pipeline.md` for this execution slice.

### Why
- The user approved scheme B: finish replay-side layered validation first, then expose one standardized payload that can later feed position-management as a pipeline.
- Before this round, replay mode could show realized layered returns and prediction mode could show predicted layered returns, but there was no governed object comparing the two.
- The future position-management workflow also needed one stable upstream contract instead of reverse-parsing multiple master-scorecard branches.

### Remaining
- [ ] Wire `pipeline_payload` into `security_position_plan` or `security_portfolio_position_plan` as a real sizing / tranche adjustment input instead of leaving it advisory-only.
- [ ] Decide whether replay validation should also publish layer-specific confidence grades, not just raw error fields.
- [ ] Add multi-symbol replay audits so the new validation payload can summarize error concentration by market / sector / stock layer across a batch run.

### Risks
- [ ] The current `pipeline_payload` uses deterministic rule thresholds, so it is orchestration-ready but not yet a learned sizing policy.
- [ ] The first real bank replay comparison showed low total-return error but still large market-layer underestimation, so downstream sizing should not trust the market leg too aggressively yet.
- [ ] Replay comparison is only populated when all three layered artifact paths are present and valid; missing any one path still downgrades the comparison to `None`.

### Closed
- Replay mode now publishes predicted-vs-realized layered comparison in the formal master-scorecard output when layered artifacts are provided.
- Prediction mode now publishes a stable position-management pipeline payload that downstream tools can consume without reinterpreting raw master-scorecard fields.
- Fresh verification passed:
  - `cargo test --test security_master_scorecard_cli security_master_scorecard_replay_mode_compares_layered_prediction_with_realized_replay -- --exact --nocapture`
  - `cargo test --test security_master_scorecard_cli security_master_scorecard_prediction_mode_emits_position_pipeline_payload -- --exact --nocapture`
  - `cargo test --test security_master_scorecard_cli -- --nocapture`
  - `cargo test --test security_scorecard_cli -- --nocapture`
- Real runtime example re-check completed for `601916.SH / 2026-04-01 / 10d` with the latest three calibrated layered artifacts.

## 2026-04-18
### Modified
- Added helper-level coverage around `D:\SM\src\ops\security_master_scorecard.rs` for:
  - `build_layered_prediction_replay_comparison(...)`
  - `build_pipeline_payload(...)`
  - `resolve_pipeline_allocation_signal(...)`
  - `resolve_pipeline_risk_adjustment_hint(...)`
- Updated `D:\SM\tests\security_composite_scorecard_unit.rs` and `D:\SM\tests\security_composite_committee_payload_adapter_unit.rs` fixtures so `layered_return_summary`, `layered_prediction_replay_comparison`, and `pipeline_payload` are covered explicitly.

### Why
- The user approved scheme B for this slice: freeze replay-side comparison and pipeline payload behavior at the helper/unit layer before expanding downstream orchestration.
- CLI coverage alone was not enough because the replay and pipeline branches are assembled through `security_master_scorecard.rs` helpers.
- The fixture layer also needed to stop drifting now that the composite scorecard document carries additional layered payload fields.

### Remaining
- [ ] Decide whether `resolve_pipeline_validation_status(...)` and `resolve_pipeline_conviction_score(...)` should move into the same governed pipeline-helper surface.
- [ ] Add an end-to-end check that exercises the composed pipeline payload through the full downstream adapter path.
- [ ] Decide whether `replay_unavailable` should remain a soft downgrade or become a stronger downstream gating signal.

### Risks
- [ ] The helper logic still freezes threshold-style branch behavior around values such as `0.03`, `0.05`, and `0.08`; if those business thresholds change, tests must move with them.
- [ ] The helper assertions still depend on small floating-point tolerances such as `1e-12`, so future refactors should avoid making them brittle.
- [ ] Local search still prefers PowerShell over `rg.exe` in this workspace because `rg.exe` has produced `Access is denied` intermittently.

### Closed
- `security_master_scorecard.rs` replay/pipeline helper behavior is now frozen with focused unit and adapter coverage.
- Composite fixture expectations now explicitly include the layered replay and pipeline payload fields.
- Focused verification passed:
  - `cargo test --lib build_layered_prediction_replay_comparison_computes_expected_replay_deltas -- --nocapture`
  - `cargo test --lib build_pipeline_payload_prediction_stage_marks_position_management_ready -- --nocapture`
  - `cargo test --lib build_pipeline_payload_replay_stage_uses_validation_artifact_outputs -- --nocapture`
  - `cargo test --lib resolve_pipeline_allocation_signal_maps_prediction_and_replay_branches -- --nocapture`
  - `cargo test --lib resolve_pipeline_risk_adjustment_hint_maps_prediction_and_replay_branches -- --nocapture`
  - `cargo test --test security_composite_scorecard_unit -- --nocapture`
  - `cargo test --test security_composite_committee_payload_adapter_unit -- --nocapture`
  - `cargo test --test security_master_scorecard_cli -- --nocapture`

## 2026-04-18
### Modified
- Added the formal LLM-facing packet tool `security_investment_manager_entry`.
- Created `D:\SM\src\ops\security_investment_manager_entry.rs` with the compact delivery artifact `SecurityInvestmentManagerEntryPacket`.
- Created `D:\SM\src\ops\stock_investment_manager_entry.rs` as the dedicated scenario entry shell.
- Wired the new packet through `stock.rs`, `stock_pre_trade.rs`, `catalog.rs`, `dispatcher.rs`, and `dispatcher/stock_ops.rs`.
- Added `D:\SM\tests\security_investment_manager_entry_cli.rs` to lock tool discovery and compact packet output.
- Added `D:\SM\docs\plans\2026-04-18-stock-investment-manager-entry-plan.md`.
### Why
- The user approved scheme B: keep the LLM entry upstream and separate from the pure mathematical position-management engine.
- The user also required one clear delivery artifact for the LLM and emphasized that the packet should contain only just-enough material.
- The code graph confirmed that the position-management and execution chain should stay untouched for this slice.
### Remaining
- [ ] Tune packet density after real LLM usage and decide whether additional market-state fields are necessary.
- [ ] Decide whether the future daily stock-evaluation hook should replace the current evidence-bundle source or extend this packet.
### Risks
- [ ] The first packet version is intentionally compact, so some future workflows may still require a second retrieval step for deeper evidence.
- [ ] Compact focus points are deterministic projections from the evidence bundle and are not yet tuned by live LLM usage feedback.
### Closed
- Added the first formal LLM entry delivery artifact: `SecurityInvestmentManagerEntryPacket`.
- Added the public tool route: `security_investment_manager_entry`.
- Verification passed:
  - `cargo test --test security_investment_manager_entry_cli -- --nocapture`
  - `cargo test --test security_decision_evidence_bundle_cli -- --nocapture`
## 2026-04-18
### Modified
- Updated `D:\SM\tests\security_scorecard_training_cli.rs` so the training CLI contract now follows the retained disclosure feature set instead of the removed legacy announcement proxies.
- Replaced the hard-coded `feature_count = 40` assertion with a retained-feature contract list that currently resolves to 32 governed training features.
- Added explicit exclusion checks for the pruned disclosure features: `announcement_count`, `disclosure_risk_keyword_count`, `has_risk_warning_notice`, `negative_attention_score`, `event_net_impact_score`, `risk_note_count`, `event_density_bucket`, and `shareholder_return_status`.
- Switched the unseen categorical fallback-bin regression from removed feature `has_risk_warning_notice` to retained feature `disclosure_status`.
### Why
- The approved disclosure thinning rule keeps only the symmetric pair `positive_support_score` and `hard_risk_score`, so the old CLI test contract had become stale and caused false failures.
- The task needed a governed include/exclude contract instead of another fixed-count snapshot so later retraining rounds can be audited more safely.
### Remaining
- [ ] Consider extracting one shared canonical feature-contract helper so CLI tests do not need to duplicate the retained feature list.
- [ ] Re-check whether any non-CLI audit scripts still assume the removed disclosure feature names.
### Risks
- [ ] The CLI test still mirrors the retained feature vocabulary locally, so future feature-contract changes will require synchronized test updates.
- [ ] `.trae/CHANGELOG_TASK.md` still uses a legacy local encoding path, which can make direct patch-based updates fragile.
### Closed
- Aligned the training CLI regression contract with the current disclosure-thinning rule.
- Verification passed:
  - `cargo test --test security_scorecard_training_cli -- --nocapture`
  - `cargo test --test security_scorecard_cli -- --nocapture`
  - `cargo test --test security_master_scorecard_cli -- --nocapture`
## 2026-04-18
### Modified
- Added the formal design document `D:\SM\docs\plans\2026-04-18-master-scorecard-position-management-integration-design.md`.
- The document freezes the approved integration path from the prediction engine into the position-management system.
- The document explicitly limits the current slice to the chain ending at `security_record_position_adjustment` and excludes LLM flow, report output, chair execution, and post-trade review.
### Why
- The user asked to stop discussing broad concepts and first organize the approved engine-to-position-management flow as a formal document.
- The user also clarified that post-trade review and LLM governance logic must not be mixed into the current position-management design slice.
### Remaining
- [ ] Convert the document into a concrete field-mapping table from `pipeline_payload` to position-plan fields.
- [ ] Convert the approved integration order into a detailed implementation task list.
### Risks
- [ ] The current document fixes architecture direction, but the exact mapping thresholds and sizing translations are still undecided.
### Closed
- Formal integration design document now exists for the approved path:
  - `security_master_scorecard.pipeline_payload -> security_position_plan -> security_portfolio_position_plan -> execution-state modules`
## 2026-04-19
### Modified
- Implemented approved plan B in `D:\SM\src\ops\security_scorecard_training.rs`.
- Added `assign_four_day_rotation_split_dates` for the 1d relative-benchmark path and `assign_non_overlapping_anchor_split_dates` for the multi-day relative-benchmark path.
- Added train-time sparse-feature governance so ordinary low-support features are filtered before WOE/logit fitting, while `hard_risk_score` stays behind an explicit exemption hook.
- Added `feature_governance_summary` into training diagnostics and routed walk-forward diagnostics through retained governed features.
- Updated `D:\SM\tests\security_scorecard_training_cli.rs` so the CLI contract now audits candidate vs retained feature counts and the new 10d split geometry.
### Why
- The user approved scheme B and explicitly asked to stop using the old large 2-1-1 style split semantics for these retraining paths.
- The user also required ordinary low-support features to stay out of the main model, while extreme risk fields must keep a separate survival path.
- Existing training CLI regressions were still asserting the old fixed-count feature contract and old relative-benchmark sample geometry, so they needed to be realigned with the governed training behavior.
### Remaining
- [ ] Decide whether categorical sparse governance should stay on the current shared `insufficient_bin_support` rule or move to a dedicated collapsed-category policy.
- [ ] Expand the current high-risk exemption hook from `hard_risk_score` to a formal independent risk-layer contract.
### Risks
- [ ] The current sparse-feature governance uses a conservative support threshold, so very small retraining samples may filter more ordinary features than expected and should be watched during the next real rerun.
- [ ] The new 10d anchor split reduces sample count by design, so downstream expectations that still assume dense overlapping windows may need follow-up alignment.
### Closed
- Approved plan B is now wired into the trainer and diagnostic surface.
- Verification passed:
  - `cargo test relative_benchmark_1d_split_uses_four_day_rotation --lib`
  - `cargo test relative_benchmark_10d_split_uses_non_overlapping_anchor_dates --lib`
  - `cargo test sparse_training_features_are_filtered_before_model_building --lib`
  - `cargo test --test security_scorecard_training_cli`
  - `cargo test --test security_scorecard_cli`
  - `cargo test --test security_master_scorecard_cli`
## 2026-04-19
### Modified
- Added `D:\SM\docs\plans\2026-04-19-data-quality-feature-main-model-exclusion-design.md` and `D:\SM\docs\plans\2026-04-19-data-quality-feature-main-model-exclusion-implementation.md`.
- Updated `D:\SM\src\ops\security_scorecard_training.rs` so `disclosure_status` and `data_gap_count` stay in the raw feature contract but are policy-filtered out of the main model.
- Added unit coverage proving both features are excluded from `retained_feature_configs` while still appearing in `feature_governance_summary.filtered_features`.
- Updated `D:\SM\tests\security_scorecard_training_cli.rs` so the artifact contract excludes the two data-quality features and diagnostics must report them with the policy reason.
- Re-ran the real `bank-parent.v1` 10d retrain on the governed runtime and captured the new registry and diagnostic outputs.
### Why
- The user explicitly chose option 1: remove `disclosure_status` and `data_gap_count` from the main training model but keep them in diagnostics and audit output.
- The previous real retrain showed the two features were overlapping quality proxies, produced a near-perfect correlation pair, and contributed to a blocked readiness state.
### Remaining
- [ ] Decide whether the next cleanup round should also compress other quality-adjacent proxies such as `fundamental_status` or `hard_risk_score` into a clearer risk/audit split.
- [ ] Decide whether the policy exclusion should stay fixed or become contract-specific by training pool / horizon.
### Risks
- [ ] Because both features are now excluded before model fitting, any downstream report that assumed they still appear in the artifact feature list will need the diagnostic summary instead.
- [ ] The current fix removes overlap cleanly, but it does not yet address whether the remaining macro-state features are still partially redundant.
### Closed
- Removed `disclosure_status` and `data_gap_count` from the main scorecard model while keeping both in governed diagnostics via `excluded_from_main_model_data_quality_overlap`.
- Real retrain improved from `blocked` to `candidate` on the same bank-parent window.
- Verification passed:
  - `cargo test governance_policy_excludes_data_quality_overlap_features_from_main_model --lib -- --nocapture`
  - `cargo test --test security_scorecard_training_cli security_scorecard_training_generates_artifact_and_registers_refit_outputs -- --nocapture`
  - `cargo test --test security_scorecard_training_cli -- --nocapture`
  - `cargo test sparse_training_features_are_filtered_before_model_building --lib -- --nocapture`
  - `cargo test --test security_scorecard_cli -- --nocapture`
  - `cargo test --test security_master_scorecard_cli -- --nocapture`
## 2026-04-18
### Modified
- Added `D:\SM\docs\plans\2026-04-18-post-open-position-data-system-design.md` as one merged design document for the post-open-position pure-data system.
- Updated `D:\SM\progress.md` and `D:\SM\task_plan.md` to record the merged business boundary and next schema-expansion gap.
### Why
- The user explicitly asked to stop spreading this work across too many design files and consolidate the approved business flow into one document.
- The final approved boundary now includes approved open-position intake, daily monitoring evidence output, future LLM committee hook, and capital-event rebasing.
### Remaining
- [ ] Expand the merged design into field-level schema documents or field tables for `ApprovedOpenPositionPacket`, `PositionContract`, `PerPositionEvaluation`, `MonitoringEvidencePackage`, and `CapitalEvent`.
- [ ] Map the merged design objects onto concrete Rust modules before starting implementation planning.
### Risks
- [ ] The current document fixes business boundaries and flow, but it does not yet freeze field-level contracts or persistence schema names.
### Closed
- The post-open-position pure-data design is now consolidated into one formal reference document and no longer split across three separate design threads.
## 2026-04-18
### Modified
- Expanded `D:\SM\docs\plans\2026-04-18-post-open-position-data-system-design.md` with field-level schema baselines for `ApprovedOpenPositionPacket`, `PositionContract`, `PerPositionEvaluation`, `MonitoringEvidencePackage`, and `CapitalEvent`.
- Updated `D:\SM\docs\AI_HANDOFF.md` to lock future AI position-management work to the merged post-open-position design document unless the user explicitly approves a flow change.
- Updated `D:\SM\progress.md` and `D:\SM\task_plan.md` to reflect the schema-expansion and handoff-lock completion.
### Why
- The user asked to continue expanding the merged design into concrete fields and also required that the AI handoff manual enforce alignment with the approved report.
- The project now needs a stable business-contract baseline before mapping these objects to Rust modules.
### Remaining
- [ ] Map the approved business objects onto concrete Rust modules and document which existing structs can be evolved versus which new structs are required.
- [ ] Freeze persistence-layer naming and storage boundaries for the new pure-data objects before implementation starts.
### Risks
- [ ] The new field tables are a business-schema baseline, but they are not yet a final storage schema, so naming or optionality may still need one more pass during module mapping.
### Closed
- The merged post-open-position design now includes field-level schema definitions and the AI handoff manual now explicitly requires future AI sessions to follow that design.
## 2026-04-18
### Modified
- Updated `D:\SM\docs\plans\2026-04-18-post-open-position-data-system-design.md` with a new `Single Source Data-Side Flow` section.
- Added one end-to-end pure-data master flowchart, one stage table, and one set of data-side rules so the whole process can be read in one place.
- Updated `D:\SM\docs\AI_HANDOFF.md` to state that the merged post-open-position design is the single source of truth for the whole pure-data flow.
- Updated `D:\SM\progress.md` and `D:\SM\task_plan.md` to reflect the consolidation.
### Why
- The user explicitly asked to consolidate the whole data-side flow because scattered process descriptions were likely to cause mistakes in later implementation or AI handoff.
### Remaining
- [ ] Map the single-source business objects and stages onto concrete Rust modules and file ownership.
- [ ] Decide whether the next document should freeze persistence schema boundaries or go directly into implementation mapping.
### Risks
- [ ] The unified flow is now centralized at the business-design level, but downstream code-module mapping is still pending, so different implementers could still diverge if module ownership is not frozen next.
### Closed
- The pure-data process is now consolidated into one single-source section inside the merged design document, and the AI handoff manual now names that document as the authoritative process reference.
## 2026-04-18
### Modified
- Corrected `D:\SM\docs\plans\2026-04-18-post-open-position-data-system-design.md` so both `Adjustment Simulation Data` and `Capital Rebalance Evidence Package` must flow through the future committee/chair chain before `AdjustmentInputPackage` can be created.
- Updated the unified flowchart, lifecycle flow, daily monitoring flow, capital rebasing flow, and data-side rules to reflect the corrected governance line.
- Updated `D:\SM\docs\AI_HANDOFF.md`, `D:\SM\progress.md`, and `D:\SM\task_plan.md` so future AI sessions inherit the corrected process.
### Why
- The user explicitly pointed out that the previous version was missing the governance line: both ordinary adjustment measurement and capital rebalance evidence must be judged by the committee/chair chain rather than flowing directly into action input.
### Remaining
- [ ] Translate the corrected single-source flow into concrete Rust module ownership and object boundaries.
- [ ] Decide whether the next step should freeze Chinese-localized companion docs or move directly into implementation mapping.
### Risks
- [ ] The governing business flow is now corrected at the document level, but older conversational summaries may still contain the previous shortcut path and should not be reused as references.
### Closed
- The authoritative design and AI handoff documents now enforce that no adjustment or capital rebalance path may bypass future committee/chair review.
## 2026-04-18
### Modified
- Expanded `D:\SM\docs\plans\2026-04-18-post-open-position-data-system-design.md` with a concrete Rust module mapping section.
- Added recommended new files, recommended existing files to modify, and a file-level method map for the post-open pure-data design.
- Updated `D:\SM\progress.md` and `D:\SM\task_plan.md` to record that the module mapping is now fixed at the design-document level.
- Normalized the numbering inside the `Current Code Seams` section so the subsection labels now match section 9.
### Why
- The user explicitly asked for the design to name which files, structs, and methods should change, ideally down to which file should add which method.
### Remaining
- [ ] Decide whether the recommended file ownership should be frozen as final before creating the implementation plan.
- [ ] If frozen, translate the file-level map into a step-by-step implementation plan with tests.
### Risks
- [ ] The mapping is now explicit at the design level, but some file ownership choices such as `security_position_contract.rs` versus deeper reuse of `security_position_plan.rs` may still need one final approval before code changes start.
### Closed
- The authoritative post-open pure-data design now includes a file-level Rust mapping and method-level ownership guidance instead of only business objects and flow descriptions.
## 2026-04-18
### Modified
- Added `D:\SM\docs\architecture\post_open_position_data_graph.json` as a graphify-style machine-readable graph for the post-open pure-data system.
- The graph includes process stages, business objects, existing code files and symbols, planned files and methods, and governance edges for committee/chair review.
- Updated `D:\SM\docs\plans\2026-04-18-post-open-position-data-system-design.md` to reference the JSON graph as the machine-readable companion artifact.
- Updated `D:\SM\docs\AI_HANDOFF.md` so future AI sessions must treat both the narrative design doc and the companion JSON graph as the authoritative references.
- Updated `D:\SM\progress.md` and `D:\SM\task_plan.md` to record the new artifact.
### Why
- The user explicitly asked for a graph-style associated JSON so future work can anchor on a machine-readable structure and reduce hallucination as the design grows.
### Remaining
- [ ] Decide whether to extend the graph with persistence-store nodes and runtime schema nodes before implementation begins.
- [ ] Decide whether the next step should generate an implementation plan directly from this graph or first freeze the planned file ownership one more time.
### Risks
- [ ] The current graph is a governed design graph rather than an auto-extracted code graph, so it must stay aligned with future code changes and should be updated whenever the ownership map changes.
### Closed
- A graphify-style machine-readable companion source now exists for the post-open pure-data design, and the AI handoff manual now points future sessions at that artifact.
## 2026-04-18
### Modified
- Added `D:\SM\docs\plans\2026-04-18-post-open-position-data-system-implementation-plan.md` as the executable implementation plan for the approved post-open pure-data system.
- Updated `D:\SM\progress.md` and `D:\SM\task_plan.md` to record the new implementation-plan artifact and execution choice gap.
### Why
- The user approved moving from design into implementation planning and asked for a concrete plan tied back to the approved graph/design artifacts.
### Remaining
- [ ] Choose whether to execute the plan in this session or in a separate executing-plans session.
- [ ] Freeze any remaining file-ownership question before code changes start if the user wants one final review.
### Risks
- [ ] The plan is intentionally test-first and module-sliced, so if implementation starts without following the task order, the design and graph guardrails may drift.
### Closed
- The post-open pure-data system now has a concrete implementation plan aligned with the approved design document and companion graph JSON.
## 2026-04-19
### Modified
- Updated `D:\SM\src\ops\security_scorecard_training.rs` so `hard_risk_score` is excluded from the main scorecard model through governed feature filtering.
- Updated `D:\SM\tests\security_scorecard_training_cli.rs` so the training artifact diagnostics must report `hard_risk_score` with `excluded_from_main_model_event_risk_overlap`.
- Re-ran the focused governance unit test and the scorecard training CLI regression test to confirm the exclusion policy is active in both code paths.
### Why
- The user confirmed option 1: `hard_risk_score` overlaps with negative event information and was amplifying downside weight inside the main model.
### Remaining
- [ ] Repair runtime raw snapshot construction in `D:\SM\src\ops\security_scorecard.rs` so market-state features used during training are also present during replay/scoring.
- [ ] Re-run the `601916.SH` 1d replay after the runtime snapshot repair and confirm whether `feature_incomplete` is removed.
### Risks
- [ ] The training path now excludes `hard_risk_score`, but replay outputs can still degrade to `feature_incomplete` until runtime snapshot fields such as market cycle and risk appetite are aligned with training.
### Closed
- `hard_risk_score` no longer enters the main model governance path, and diagnostics now record the exclusion as `excluded_from_main_model_event_risk_overlap`.

## 2026-04-19
### Modified
- Added `D:\SM\src\ops\security_approved_open_position_packet.rs` as the formal `ApprovedOpenPositionPacket` intake module for the post-open pure-data system.
- Wired the new intake contract through `D:\SM\src\ops\stock.rs`, `D:\SM\src\ops\stock_execution_and_position_management.rs`, `D:\SM\src\tools\catalog.rs`, `D:\SM\src\tools\dispatcher.rs`, and `D:\SM\src\tools\dispatcher\stock_ops.rs`.
- Kept `D:\SM\tests\security_approved_open_position_packet_cli.rs` as the Task 1 red-green contract and verified it passes after implementation.
- Updated `D:\SM\task_plan.md` and `D:\SM\progress.md` with the Task 1 execution status and flow-design alignment note.
### Why
- The approved implementation plan requires the post-open pure-data chain to start from one governed approved packet instead of implicit reuse of pre-trade approval artifacts.
- The user selected Scheme B, which requires minimal Task 1 implementation plus formal placement inside the execution-and-position-management grouping.
- Freezing the intake contract first reduces the risk of mixing later `PositionContract`, monitoring, or LLM governance work into the first delivery slice.
### Remaining
- [ ] Start Task 2 and add the `PositionContract` layer on top of the frozen approved packet.
- [ ] Keep the next step limited to contract-building logic and avoid pulling daily monitoring or capital rebasing forward too early.
### Risks
- [ ] This round verified the focused Task 1 CLI contract only, not the full repository test suite.
- [ ] The current intake module intentionally enforces only the hard governance gates from Task 1; later business invariants still need to be frozen in `PositionContract`.
### Closed
- Task 1 red-green cycle for `ApprovedOpenPositionPacket` is complete.
- Focused verification passed: `cargo test --test security_approved_open_position_packet_cli -- --nocapture`
- Flow and design remain aligned: the pure data post-open chain still starts from one approved intake packet and downstream layers were not merged forward.
## 2026-04-18
### Modified
- Extended `D:\SM\tests\security_approved_open_position_packet_cli.rs` with follow-up Task 1 coverage for normalization and blank identity fields.
- Updated `D:\SM\src\ops\security_approved_open_position_packet.rs` so normalized blank `packet_id`, `account_id`, `approval_session_id`, and `source_packet_version` now fail fast.
- Updated `D:\SM\progress.md` with the hardening pass and the flow-design alignment note.
### Why
- The first Task 1 delivery froze the main approved intake path, but several identity anchors were still implicitly accepted as blank after normalization.
- The user explicitly asked to add the missing boundary tests before continuing deeper into the post-open flow.
- Tightening the intake contract now reduces the risk of building `PositionContract` on top of ambiguous packet identity.
### Remaining
- [ ] Start Task 2 and build `PositionContract` on top of the hardened approved intake packet.
- [ ] Decide later whether `direction` and `recommended_entry_mode` should remain normalization-only or become explicit enum gates.
### Risks
- [ ] This round still verifies the focused Task 1 CLI contract only, not the full repository test suite.
- [ ] Enum-like fields are normalized but not yet restricted to an explicit allowed-value set.
### Closed
- Task 1 boundary hardening is complete for blank identity anchors and secondary normalization coverage.
- Focused verification passed: `cargo test --test security_approved_open_position_packet_cli -- --nocapture`
- Flow and design remain aligned: the pure data chain still starts from one approved intake packet, now with stricter traceability anchors.
## 2026-04-19
### Modified
- Updated `D:\SM\src\ops\security_scorecard.rs` so runtime `build_raw_feature_snapshot(...)` now persists `market_cycle_status`, `market_fund_flow_status`, `sector_fund_flow_status`, and `market_risk_appetite_status` beside the existing market regime and flow fields.
- Added one focused runtime regression test in `D:\SM\src\ops\security_scorecard.rs` to lock that the scorecard raw snapshot exposes the same market-state layer already used by training.
- Replayed `601916.SH` on `2026-04-01` with the explicit model artifact `a_share_equity_1d_direction_head__candidate_2026_04_18T21_15_00_08_00.json` and confirmed the scorecard moved to `ready` instead of falling into `feature_incomplete`.
### Why
- The user approved plan A after the root cause was traced to runtime/training feature drift: training consumed the new market-state fields, but runtime scorecard replay still dropped them from `raw_feature_snapshot`.
### Remaining
- [ ] Decide whether to extract the market-state derivation into one shared helper so snapshot generation and runtime scorecard stop maintaining parallel logic.
- [ ] Re-check the broader ready-case approval regression after the unrelated committee-status drift is isolated, because one existing CLI test currently expects `ready_for_review` but now returns `needs_more_evidence` before scorecard assertions begin.
### Risks
- [ ] `D:\SM\src\ops\security_scorecard.rs` and `D:\SM\src\ops\security_feature_snapshot.rs` still derive the same market-state layer in two places, so future field additions could drift again unless the shared contract is centralized.
### Closed
- Runtime scorecard replay now carries the market-state fields required by the trained 1d bank model, and the `601916.SH / 2026-04-01` replay no longer degrades to `feature_incomplete` when the explicit model artifact is supplied.

## 2026-04-18
### Modified
- Added `D:\SM\tests\security_position_contract_cli.rs` as the Task 2 red-green contract for the new live `PositionContract` layer.
- Added `D:\SM\src\ops\security_position_contract.rs` and wired it through `D:\SM\src\ops\stock.rs`, `D:\SM\src\ops\stock_execution_and_position_management.rs`, `D:\SM\src\tools\catalog.rs`, `D:\SM\src\tools\dispatcher.rs`, and `D:\SM\src\tools\dispatcher\stock_ops.rs`.
- Updated `D:\SM\src\ops\security_position_plan.rs` to keep `SecurityPositionPlanDocument` as the pre-trade seed while adding `SecurityPositionContractSeed` plus the Task 2 seed-builder helpers.
- Updated `D:\SM\progress.md` and `D:\SM\task_plan.md` with the Task 2 execution result and flow-design alignment note.
### Why
- The approved post-open design requires `PositionContract` to be the only formal live-governance object after `ApprovedOpenPositionPacket`.
- The user explicitly confirmed that the pre-trade plan document must remain a seed and must not be reused as the live contract itself.
- Freezing the live contract now gives later monitoring, rebasing, and adjustment tasks one governed object to build on.
### Remaining
- [ ] Start Task 3 and evolve the open snapshot into `ActivePositionBook` semantics.
- [ ] Decide later whether the live contract should enforce explicit allowed-value enums for fields such as `direction` and `entry_mode`.
### Risks
- [ ] This round verified the focused Task 2 contract path and adjacent Task 1 / position-plan regressions, not the full repository test suite.
- [ ] The first Task 2 delivery keeps `correlation_guardrail` empty and leaves richer rebasing behavior for later dedicated tasks.
### Closed
- Task 2 red-green cycle for `PositionContract` is complete.
- Focused and adjacent verification passed:
  - `cargo test --test security_position_contract_cli -- --nocapture`
  - `cargo test --test security_position_plan_cli -- --nocapture`
  - `cargo test --test security_approved_open_position_packet_cli -- --nocapture`
- Flow and design remain aligned: `ApprovedOpenPositionPacket -> PositionContract` is now explicit, while active-position-book and monitoring layers remain separate future tasks.
## 2026-04-18
### Modified
- Extended `D:\SM\tests\security_position_contract_cli.rs` with follow-up Task 2 coverage for symbol mismatch, merged risk-budget capping, and legacy risk-budget fallback behavior.
- Added a unit test in `D:\SM\src\ops\security_position_contract.rs` for `rebase_security_position_contract(...)`.
- Updated the older fixture in `D:\SM\src\ops\security_execution_record.rs` so `SecurityPositionPlanDocument` includes the new `risk_budget_pct` field.
### Why
- The user asked to harden Task 2 before moving on, especially around contract consistency, risk-budget merge rules, and rebasing semantics.
- The new rebasing test also revealed one adjacent compile-time fixture gap caused by the Task 2 document-shape change.
### Remaining
- [ ] Start Task 3 and evolve the open snapshot into `ActivePositionBook` semantics.
- [ ] Decide later whether Task 2 should add explicit enum validation for fields such as `direction` and `entry_mode`.
### Risks
- [ ] The hardening pass verifies focused Task 2 and one rebasing unit path, not the full repository test suite.
- [ ] The first `PositionContract` still leaves richer correlation/rebase package semantics for later tasks.
### Closed
- Task 2 hardening coverage is now in place for symbol consistency, risk-budget merge rules, and rebasing state mutation.
- Focused verification passed:
  - `cargo test --test security_position_contract_cli -- --nocapture`
  - `cargo test rebase_security_position_contract_updates_status_capital_and_timestamp -- --nocapture`
## 2026-04-18
### Modified
- Extended `D:\SM\tests\security_account_open_position_snapshot_cli.rs` with Task 3 coverage for `ActivePositionBook` exposure and multi-position refresh stability.
- Updated `D:\SM\src\ops\security_account_open_position_snapshot.rs` to add `SecurityActivePositionDocument`, `SecurityActivePositionBookDocument`, `build_active_position_book(...)`, and `refresh_active_position_book(...)`.
- Kept the existing compatibility snapshot output and added `active_position_book` beside it instead of replacing the older shell.
### Why
- The approved plan requires Task 3 to evolve the open snapshot into explicit active-position-book semantics before Task 4 can evaluate positions.
- The user approved Scheme B, which keeps the existing snapshot path but adds a clearer live-state document in the same owner file.
### Remaining
- [ ] Start Task 4 and add the per-position evaluation layer on top of the active-position book and position contracts.
- [ ] Decide later whether active-position-book should add richer fields such as current market value or contract refs directly.
### Risks
- [ ] This round verified the focused Task 3 CLI path only; it did not run the full repository suite.
- [ ] The current active-book layer sorts by symbol for stability, which may differ from execution insertion order if any future consumer expects original order.
### Closed
- Task 3 red-green cycle for `ActivePositionBook` semantics is complete.
- Focused verification passed: `cargo test --test security_account_open_position_snapshot_cli -- --nocapture`
- Flow and design remain aligned: snapshot compatibility is preserved while `ActivePositionBook` is now explicit for later monitoring tasks.
## 2026-04-18
### Modified
- Added `D:\SM\task_plan.md`, `D:\SM\findings.md`, and `D:\SM\progress.md` to persist the bank-pool `1d` replay audit process and findings for this session.
- Generated `D:\SM\tests\runtime_fixtures\exports\bank_1d_q1_2026_backtest_summary.json` by replaying the governed `security_master_scorecard` flow across the full bank pool for `2026Q1`.
### Why
- The user asked to verify whether the current bank `1d` strong-vs-weak model can be used in practice, so we needed a real replay instead of another qualitative discussion.
- Persisting the audit plan and summary now keeps the next retraining/debugging round anchored to one explicit evidence file.
### Remaining
- [ ] Decide whether the next round should fix direction inversion first or expand the replay export so daily leader concentration and subindustry breakdowns are stored directly.
- [ ] If the model remains bank-only, add one governed diagnostic view for parent/child inheritance buckets before the next retrain.
### Risks
- [ ] This replay audit validated the existing runtime and model artifact on `2026Q1` only; it does not prove stability outside that window.
- [ ] The current export is summary-oriented and does not yet persist every daily leaderboard row, so some deeper diagnostics still require one extra export pass.
### Closed
- Bank-pool `1d` replay audit completed with `56` dates, `42` bank stocks, `2352` scoring calls, and `0` runtime failures.
- The current ranking signal did not show usable separation in this window: `Top1 hit rate = 0.50`, `Top3-Bottom3 excess spread = -0.00217`, and low-probability names slightly outperformed high-probability names.

## 2026-04-18
### Modified
- Extended `D:\SM\tests\security_account_open_position_snapshot_cli.rs` with two Task 3 regression tests for closed-or-zero-weight filtering and empty-account active-book behavior.
- Added `D:\SM\tests\security_per_position_evaluation_cli.rs` as the Task 4 red-green CLI contract.
- Added `D:\SM\src\ops\security_per_position_evaluation.rs` and wired it through `D:\SM\src\ops\stock.rs`, `D:\SM\src\ops\stock_execution_and_position_management.rs`, `D:\SM\src\tools\catalog.rs`, `D:\SM\src\tools\dispatcher.rs`, and `D:\SM\src\tools\dispatcher\stock_ops.rs`.
### Why
- The approved post-open pure-data flow requires `PerPositionEvaluation` to sit between `ActivePositionBook` and the later account aggregation / monitoring evidence layers.
- The user asked to finish Task 3 cleanup and Task 4 together in one pass before moving on.
### Remaining
- [ ] Start Task 5 and build the monitoring evidence package on top of per-position evaluations.
- [ ] Decide later whether Task 4 should add richer risk-pressure fields such as concentration or correlation proxies before account aggregation.
### Risks
- [ ] This round verified the focused Task 3 / Task 4 / adjacent Task 2 paths, not the full repository test suite.
- [ ] The first Task 4 scoring model is intentionally deterministic and lightweight; it is suitable for the pure-data layer but not yet a full portfolio optimizer.
### Closed
- Task 3 boundary hardening and Task 4 red-green cycle are complete.
- Focused verification passed:
  - `cargo test --test security_account_open_position_snapshot_cli -- --nocapture`
  - `cargo test --test security_per_position_evaluation_cli -- --nocapture`
  - `cargo test --test security_position_contract_cli -- --nocapture`
- Flow and design remain aligned: `ApprovedOpenPositionPacket -> PositionContract -> ActivePositionBook -> PerPositionEvaluation` is now explicit, while account aggregation and monitoring evidence remain the next separate tasks.
## 2026-04-18
### Modified
- Added `D:\SM\tests\security_monitoring_evidence_package_cli.rs` as the Task 5 red-green CLI contract.
- Added `D:\SM\src\ops\security_monitoring_evidence_package.rs` and wired it through `D:\SM\src\ops\stock.rs`, `D:\SM\src\ops\stock_execution_and_position_management.rs`, `D:\SM\src\tools\catalog.rs`, `D:\SM\src\tools\dispatcher.rs`, and `D:\SM\src\tools\dispatcher\stock_ops.rs`.
- Updated `D:\SM\src\ops\security_portfolio_position_plan.rs` with reusable monitoring account aggregation and action-candidate helpers.
### Why
- The approved post-open data flow requires `MonitoringEvidencePackage` to sit after `PerPositionEvaluation` and before later governance packaging.
- The user approved scheme B and asked to keep this slice pure-data and separate from committee / chair execution logic.
### Remaining
- [ ] Start Task 6 and add the capital rebase layer.
- [ ] Decide later whether Task 5 should add richer correlation-pressure fields before the capital and adjustment tasks consume the package.
### Risks
- [ ] This round verified the focused Task 2 / Task 3 / Task 4 / Task 5 paths, not the full repository test suite.
- [ ] Windows kept locking the default `target\debug\excel_skill.exe`, so verification used `--target-dir D:\SM\target_task5`; the main code path is verified, but the default local build artifact lock issue remains an environment concern.
### Closed
- Task 5 red-green cycle for `MonitoringEvidencePackage` is complete.
- Focused verification passed:
  - `cargo test --target-dir D:\SM\target_task5 --test security_monitoring_evidence_package_cli -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task5 --test security_per_position_evaluation_cli -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task5 --test security_account_open_position_snapshot_cli -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task5 --test security_position_contract_cli -- --nocapture`
- Flow and design remain aligned: `ApprovedOpenPositionPacket -> PositionContract -> ActivePositionBook -> PerPositionEvaluation -> MonitoringEvidencePackage` is now explicit, while capital rebasing and adjustment input remain later separate tasks.
## 2026-04-18
### Modified
- Ran one real governed `bank-parent.v1` `1d` retrain with `exclude_main_model_subindustry_bucket = true` and wrote the experiment outputs under:
  - `D:\SM\.stockmind_runtime\ablation_runs\subindustry_off_20260418_2240\scorecard_artifacts\`
  - `D:\SM\.stockmind_runtime\ablation_runs\subindustry_off_20260418_2240\scorecard_training_diagnostics\`
- Replayed the full `2026Q1` bank pool on both:
  - baseline model `a_share_equity_1d_direction_head__candidate_2026_04_18T21_15_00_08_00`
  - ablation model `a_share_equity_1d_direction_head__candidate_2026_04_18T22_40_00_08_00`
- Saved the A/B replay outputs to:
  - `D:\SM\tests\runtime_fixtures\exports\bank_1d_q1_2026_backtest_summary_subindustry_off.json`
  - `D:\SM\tests\runtime_fixtures\exports\bank_1d_q1_2026_backtest_compare_baseline_vs_subindustry_off.json`
- Updated `D:\SM\findings.md` and `D:\SM\progress.md` with the real ablation evidence.
### Why
- The user approved Scheme 1 to test whether the bank `1d` failure was mainly caused by an overly strong `subindustry_bucket` prior.
- The earlier conclusion was still only a hypothesis until the team had one real retrain plus one real replay under the same bank-pool and `2026Q1` audit window.
### Remaining
- [ ] Decide whether the next debug slice should first repair weak dynamic feature directionality or move directly to per-subindustry training.
- [ ] Decide whether coarse high-weight buckets such as `fundamental_quality_bucket` should be clipped/merged before the next bank rerun.
### Risks
- [ ] The ablation run kept `mean_walk_forward_accuracy = 0.5396`, so removing the static prior did not improve the core model body.
- [ ] This A/B replay still covers `2026-01-05..2026-03-31` only, so it does not prove stability in other market windows.
### Closed
- The real ablation retrain completed and diagnostics explicitly recorded `subindustry_bucket` as `excluded_from_main_model_subindustry_ablation`.
- The full-bank `2026Q1` A/B replay proved that removing `subindustry_bucket` alone does not fix the bank `1d` ranking problem and in this window makes the replay metrics worse.

## 2026-04-19
### Modified
- Extended `D:\SM\tests\security_monitoring_evidence_package_cli.rs` with four Task 5 hardening tests covering account mismatches, risk-budget pressure warnings, and ranked candidate ordering.
### Why
- The user asked whether Task 5 had enough tests and approved the richer scheme B hardening pass.
- These tests lock the highest-value Task 5 boundaries before capital rebasing and later adjustment tasks build on the package.
### Remaining
- [ ] Start Task 6 and add the capital rebase layer when the user confirms.
- [ ] Decide later whether Task 5 should add explicit correlation-pressure coverage once that field stops being a placeholder.
### Risks
- [ ] Verification again used an alternate `--target-dir` because the default Windows build artifact can remain locked by local processes.
- [ ] This round hardened Task 5 tests only; it did not expand the full repository suite.
### Closed
- Task 5 hardening tests all passed without requiring production-code changes.
- Focused verification passed:
  - `cargo test --target-dir D:\SM\target_task5_tests --test security_monitoring_evidence_package_cli -- --nocapture`
## 2026-04-19
### Modified
- Added `D:\SM\tests\security_capital_rebase_cli.rs` as the Task 6 red-green CLI contract.
- Added `D:\SM\src\ops\security_capital_rebase.rs` and wired it through `D:\SM\src\ops\stock.rs`, `D:\SM\src\ops\stock_execution_and_position_management.rs`, `D:\SM\src\tools\catalog.rs`, `D:\SM\src\tools\dispatcher.rs`, and `D:\SM\src\tools\dispatcher\stock_ops.rs`.
- Updated `D:\SM\src\ops\security_position_contract.rs` with an override-aware rebasing helper.
- Updated `D:\SM\src\ops\security_portfolio_position_plan.rs` with reusable capital rebalance simulation rows.
### Why
- The approved post-open flow requires capital changes to remain first-class account events and not be collapsed into ordinary position adjustments.
- The user approved scheme B, which keeps target weights stable by default while allowing event-level max-weight and risk-budget tightening.
### Remaining
- [ ] Start Task 7 / Task 8 follow-up for approved downstream adjustment bridging when the user confirms the next slice.
- [ ] Decide later whether Task 6 should include richer account-goal override fields beyond the current return/drawdown/cash/risk caps.
### Risks
- [ ] This round verified focused Task 6 plus adjacent Task 5 / Task 4 / Task 2 paths, not the full repository test suite.
- [ ] Verification again used an alternate `--target-dir` because the default Windows build artifact can remain locked by local processes.
### Closed
- Task 6 red-green cycle for capital rebasing is complete.
- Focused verification passed:
  - `cargo test --target-dir D:\SM\target_task6 --test security_capital_rebase_cli -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task6 --test security_monitoring_evidence_package_cli -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task6 --test security_position_contract_cli -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task6 --test security_per_position_evaluation_cli -- --nocapture`
- Flow and design remain aligned: `ApprovedOpenPositionPacket -> PositionContract -> ActivePositionBook -> PerPositionEvaluation -> MonitoringEvidencePackage -> CapitalEvent -> AccountRebaseSnapshot -> CapitalRebalanceEvidencePackage` is now explicit, while committee approval and adjustment input remain downstream work.
## 2026-04-19
### Modified
- Implemented the bank-first fundamental stack rebuild B1 slice.
- Extended governed `FundamentalMetrics` with `roa_pct`, `pe_ttm`, `pb`, `dividend_yield`, `log_revenue`, `log_net_profit`, and `pb_vs_roe_gap`.
- Added deterministic derived-metric finalization for both live fullstack parsing and governed fundamental-history replay.
- Rebuilt the main-model `F` block to include the new numeric features and exclude `fundamental_status`, `profit_signal`, and `fundamental_quality_bucket` from alpha training.
- Added/updated regressions in `security_analysis_fullstack_cli`, `security_scorecard_training_cli`, `security_stock_history_governance_cli`, and corrected stale `security_feature_snapshot_cli` expectations.
- Added design/implementation docs for the approved bank-first rebuild.
### Why
- The existing bank fundamental layer was too shallow and overweighted coarse summary buckets.
- The approved direction was to stabilize banks first with normalized general/value fundamentals before expanding to other industries.
- Governed live parsing and governed replay needed the same derived metric contract to avoid training/runtime drift.
### Remaining
- [ ] Add bank-only prudential metrics such as NIM, NPL, provision coverage, CET1, loan/deposit growth, and cost-to-income.
- [ ] Run one real bank retrain/backtest on the rebuilt B1/B2 contract and compare walk-forward and cross-sectional spread metrics.
### Risks
- [ ] Upstream financial aliases for `pe_ttm`, `pb`, `dividend_yield`, and later bank-only fields may still vary by provider.
- [ ] `pb_vs_roe_gap` is currently a first-pass deterministic proxy and may need sector-relative refinement after real retraining evidence.
- [ ] Existing historical rows written before this change may not contain all raw source fields, so replay quality still depends on available stored payload depth.
### Closed
- Focused and related regression suites passed for the modified paths.

## 2026-04-19
### Modified
- Extended `D:\SM\tests\security_capital_rebase_cli.rs` with six Task 6 boundary-hardening tests for unsupported event types, oversized withdrawals, inflow/outflow normalization, and cross-account rejection.
- Updated `D:\SM\progress.md`, `D:\SM\task_plan.md`, and `D:\SM\docs\AI_HANDOFF.md` to record the hardened Task 6 boundary and downstream separation rules.
### Why
- The user asked to harden Task 6 tests first and then sort out the boundary before moving deeper into later adjustment work.
- Task 6 is the first account-level capital-rebasing layer, so its non-execution boundary must stay explicit for future AI handoff.
### Remaining
- [ ] Start Task 7 / Task 8 only after keeping the Task 6 no-execution boundary intact.
- [ ] Decide later whether capital-goal override fields need richer coverage once downstream governance bridging is scheduled.
### Risks
- [ ] Verification again used an alternate `--target-dir` because the default Windows build artifact can remain locked by local processes.
- [ ] This round hardened the focused Task 6 boundary only; it did not expand the full repository suite.
### Closed
- The new Task 6 boundary tests all passed without requiring production-code changes.
- Focused verification passed:
  - `cargo test --target-dir D:\SM\target_task6_tests --test security_capital_rebase_cli -- --nocapture`
- Flow and handoff remain aligned: Task 6 ends at `CapitalRebalanceEvidencePackage` and does not create execution input.
## 2026-04-19
### Modified
- Ran one real `bank-parent.v1` `1d` retraining pass through `D:\SM\target\debug\excel_skill.exe` and compared it against the `2026-04-18T21:15:00+08:00` bank baseline.
- Audited `D:\SM\.stockmind_runtime\security_fundamental_history.db`, `D:\SM\findings.md`, `D:\SM\progress.md`, and `D:\SM\task_plan.md` to capture the B1 retrain result and the B2 data-gap verdict.
### Why
- The user approved a fresh retrain first and then asked whether B2 still needs after the latest B1 implementation.
- A trustworthy B2 decision required checking not just the new training artifact, but also whether governed historical financial payloads had actually been refreshed onto the new B1 field contract.
### Remaining
- [ ] Refresh governed fundamental-history payloads so existing historical rows actually contain the newly added B1 fields before judging B2 by another retrain.
- [ ] Add bank-only prudential metrics such as NIM, NPL, provision coverage, CET1, loan growth, deposit growth, and cost-to-income once the user approves the next code change slice.
### Risks
- [ ] The current retrain reused stale governed historical financial rows, so most new B1 financial fields were filtered for `insufficient_distinct_values` and the result understates the intended B1 impact.
- [ ] No bank-only prudential metric ingestion exists yet, so B2 cannot improve bank differentiation until that source-and-parser gap is closed.
### Closed
- The fresh retrain completed successfully and matched the weak baseline training quality instead of improving it.
- The audit established a concrete root cause: governed historical financial storage is still on the legacy payload shape for existing rows, and B2 is still required.

## 2026-04-19
### Modified
- Added `D:\SM\tests\post_open_position_data_flow_guard.rs` as a standalone source-guard test for the post-open governance gate.
- Updated `D:\SM\progress.md`, `D:\SM\task_plan.md`, and `D:\SM\docs\AI_HANDOFF.md` to record the guard-first boundary freeze and the separation from the legacy adjustment recorder.
### Why
- The user approved scheme B to keep moving without touching the unrelated modeling workstream that is currently blocking workspace-wide `cargo test`.
- The biggest immediate drift risk was the old adjustment recorder being mistaken for the planned `AdjustmentInputPackage` bridge.
### Remaining
- [ ] Resume formal Task 7 implementation only after the unrelated `security_analysis_fullstack.rs` compile blocker is cleared by the other workstream.
- [ ] Keep `security_record_position_adjustment` as legacy/manual-only until the real adjustment-input bridge lands.
### Risks
- [ ] This round verified a standalone source-guard only; it did not verify the full workspace through `cargo test`.
- [ ] The approved Task 7 code implementation is still pending, so only the boundary is frozen in this round.
### Closed
- Standalone guard verification passed:
  - `rustc --edition=2021 --test D:\SM\tests\post_open_position_data_flow_guard.rs -o D:\SM\target_task7_guard\post_open_position_data_flow_guard.exe`
  - `D:\SM\target_task7_guard\post_open_position_data_flow_guard.exe --nocapture`
- The post-open mainline now has an explicit test freezing the committee/chair gate before `AdjustmentInputPackage`.

## 2026-04-19
### Modified
- Updated `D:\SM\docs\plans\2026-04-18-post-open-position-data-system-design.md` with a post-Task-9 expansion blueprint.
- Added new sections for: system position after Task 9, top-tier capability map, staged roadmap `P10-P22`, and non-negotiable long-term expansion rules.
### Why
- The user asked to write the previously discussed top-tier evolution plan back into the original position-management design document.
- The current minimum lifecycle design needed one explicit explanation that Task 9 is only the coherent baseline, not the institution-grade endpoint.
### Remaining
- [ ] If approved later, synchronize the companion graph and implementation plan with the new long-term roadmap instead of leaving the blueprint narrative-only.
- [ ] Decide later whether the `P10-P22` roadmap should be split into separate dedicated design docs per phase.
### Risks
- [ ] This round updated design narrative only; it did not change the machine-readable graph or any implementation code.
- [ ] Future sessions could overreach and pull LLM into the core sizing chain unless they continue following the new non-negotiable expansion rules.
### Closed
- The original post-open design doc now explicitly distinguishes the minimum governed lifecycle from the later top-tier portfolio/risk/execution/learning/LLM roadmap.
- The long-term boundary is now documented: math stays in the core chain, and LLM stays in the governance layer.

## 2026-04-19
### Modified
- Extended `D:\SM\src\ops\security_analysis_fullstack.rs`, `D:\SM\src\ops\security_decision_evidence_bundle.rs`, and `D:\SM\src\ops\security_scorecard_training.rs` so the governed bank pipeline carries six B2 proxy metrics: `total_assets`, `total_asset_growth_pct`, `equity_growth_pct`, `asset_liability_ratio`, `equity_ratio_pct`, and `liability_to_equity_ratio_pct`.
- Fixed the live backfill path so Eastmoney success no longer prevents Sina-only bank fields from being merged, and tightened the Sina `typecode` matcher to avoid alias collisions.
- Added focused regression coverage in `D:\SM\tests\security_fundamental_history_live_backfill_cli.rs`, `D:\SM\tests\security_scorecard_training_cli.rs`, and `D:\SM\tests\stock_training_data_backfill_cli.rs`, then completed a real 42-bank refresh plus one real `1d` retrain audit.
### Why
- The user approved the `1 -> 2 -> 3` path: refresh governed fundamentals first, land B2 bank proxy metrics second, and retrain/audit third.
- The earlier B2 verdict was unreliable because the governed history path could return early on Eastmoney success and silently miss Sina-only bank balance-sheet fields required by the new contract.
### Remaining
- [ ] Run the next approved feature-governance slice before adding more bank fields: remove or consolidate the strongest collinear pairs such as `asset_liability_ratio` vs `liability_to_equity_ratio_pct` and likely `total_assets` vs `log_net_profit`.
- [ ] Decide after the next retrain whether bank-only prudential fields like NIM, NPL, provision coverage, CET1, loan/deposit growth, and cost-to-income still need sourcing.
- [ ] Re-check missing upstream valuation and ROA fields because `pb`, `pe_ttm`, `dividend_yield`, and `roa_pct` are still empty in governed storage after this round.
### Risks
- [ ] The B2 metrics were retained by the model, but the real `2026-04-19 12:35 +08:00` retrain did not improve generalization: bank test accuracy fell from `0.484375` to `0.4296875`.
- [ ] Two new high-correlation pairs are now explicit in diagnostics and can distort feature attribution: `asset_liability_ratio` vs `liability_to_equity_ratio_pct` (~1.0) and `log_net_profit` vs `total_assets` (~0.924).
- [ ] The current weakness is no longer a pure data-missing problem; continuing to add same-family balance-sheet fields without governance will likely worsen redundancy instead of improving prediction.
### Closed
- Real governed coverage for all 42 bank symbols now contains the six new B2 proxy fields across 168 historical rows.
- Focused regression suites passed for the modified paths, and the real retrain confirmed the B2 fields truly enter the retained feature set.
## 2026-04-19
### Modified
- Added the compact single-source-of-truth set under `D:\SM\docs\`: `project_intent.md`, `contract_registry.md`, `decision_log.md`, `acceptance_criteria.md`, and `response_contract.md`.
- Updated `D:\SM\docs\AI_HANDOFF.md` with `10.1 Single Source Set` and `10.2 Mandatory Update Rule` so future AI sessions must review and update these files when a task changes intent, contract, decisions, acceptance, or response behavior.
- Re-checked the new documents against the current post-open position-management mainline and confirmed they align with `D:\SM\docs\plans\2026-04-18-post-open-position-data-system-design.md` and `D:\SM\docs\architecture\post_open_position_data_graph.json`.
### Why
- The user required the project to institutionalize the three-layer structure of `Intent`, `Contract`, and `Execution Plan`, plus explicit management of decisions, assumptions, open questions, acceptance gates, and answer contract.
- Future AI handoff quality was at risk because critical constraints had been spread across natural-language discussion instead of being frozen into a small governed source set.
### Remaining
- [ ] If later approved, add lightweight guards or checklist tooling so task completion can mechanically verify that the five single-source files were reviewed and updated when needed.
- [ ] Keep these five files synchronized whenever future work changes project boundaries, formal objects, design decisions, acceptance rules, or answer behavior.
### Risks
- [ ] The current governance layer is documentation-enforced, not yet tool-enforced, so future sessions could still drift if they ignore the documented update rule.
- [ ] The five-file source set currently governs the post-open controlled slice rather than every repo subsystem, so later expansion will need explicit scope management instead of silent overreach.
### Closed
- The project now has a compact governed source set for intent, contract, decision, acceptance, and response constraints.
- The handoff rules now explicitly require every future development task to review/update these files and record the task in `.trae/CHANGELOG_TASK.md`.

## 2026-04-19
### Modified
- Added `D:\SM\tests\security_analysis_fullstack_fundamental_metrics_source_guard.rs` to guard the expanded `FundamentalMetrics` constructor sites inside `D:\SM\src\ops\security_analysis_fullstack.rs`.
- Verified the current bank fundamental rebuild branch still compiles with `cargo check -q` and the new source-guard test suite.
### Why
- The user asked to unblock the unrelated compile drift left by the bank fundamental rebuild while keeping the current Task 7 workstream untouched.
- The investigated initializer sites are already compile-true in the current workspace, so the safest fix was to freeze them with a regression guard instead of touching business logic again.
### Remaining
- [ ] If the bank fundamental contract expands again, update the new source-guard markers together with the constructor changes in the same patch.
- [ ] Return to the post-open / Task 7 line after this compile guard handoff, because this round did not change that business flow.
### Risks
- [ ] The new guard is source-based, so it protects constructor drift but does not validate semantic correctness of each parsed field value.
- [ ] `cargo check -q` passed in the current workspace, but wider integration behavior still depends on the other in-flight changes already present in the dirty tree.
### Closed
- Focused verification passed:
  - `cargo test --test security_analysis_fullstack_fundamental_metrics_source_guard -- --nocapture`
  - `cargo check -q`
- The current workspace no longer has an observable `FundamentalMetrics` compile gap on the bank fundamental rebuild line.
## 2026-04-19
### Modified
- Added `D:\SM\src\ops\security_committee_decision_package.rs` as the independent post-open committee decision package module and exposed it through `D:\SM\src\ops\stock.rs` plus `D:\SM\src\ops\stock_governance_and_positioning.rs`.
- Updated `D:\SM\src\tools\catalog.rs`, `D:\SM\src\tools\dispatcher.rs`, and `D:\SM\src\tools\dispatcher\stock_ops.rs` so `security_committee_decision_package` is now catalog-visible and callable from the formal stock tool bus.
- Removed the temporary committee-package shell from `D:\SM\src\ops\security_monitoring_evidence_package.rs` so the governance handoff no longer remains hidden inside the monitoring module.
- Added backward-compatible serde defaults on late-added fields in `D:\SM\src\ops\security_portfolio_position_plan.rs` so older capital-rebalance evidence samples still deserialize when Task 7 merges optional rebalance evidence.
- Updated `D:\SM\docs\contract_registry.md`, `D:\SM\docs\decision_log.md`, `D:\SM\docs\AI_HANDOFF.md`, and `D:\SM\docs\architecture\post_open_position_data_graph.json` to reflect the implemented `CommitteeDecisionPackage` boundary and the new code ownership links.
### Why
- The unrelated `FundamentalMetrics` compile blocker was already cleared, so the real remaining Task 7 gap became the missing `security_committee_decision_package` implementation and stock tool wiring.
- The approved scheme B required `CommitteeDecisionPackage` to become a first-class formal object instead of staying as a thin helper inside the monitoring module.
- Task 7 also needed to preserve the governance gate explicitly: monitoring evidence and optional capital-rebalance evidence may merge into one committee handoff, but they still must not create `AdjustmentInputPackage`.
### Remaining
- [ ] The later `AdjustmentInputPackage` bridge is still not implemented and must remain downstream of committee/chair approval.
- [ ] The broader companion graph still contains historical planned nodes outside this task slice; only the Task 7 committee-package portion was brought into minimum consistency in this round.
### Risks
- [ ] `SecurityCapitalRebalanceSimulationItem` now accepts missing late-added fields during deserialization for backward compatibility, so future callers must still prefer emitting the full richer simulation shape instead of relying on defaults.
- [ ] The current committee package is an evidence-only governance handoff; future sessions could still drift if they mistake this implementation for permission to emit execution-facing adjustment input.
### Closed
- Focused Task 7 verification passed:
  - `cargo test --target-dir D:\SM\target_task7_impl --test security_committee_decision_package_cli -- --nocapture`
- Neighbor-flow verification stayed green:
  - `cargo test --target-dir D:\SM\target_task7_impl --test security_capital_rebase_cli -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task7_impl --test security_monitoring_evidence_package_cli -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task7_impl --test security_analysis_fullstack_fundamental_metrics_source_guard -- --nocapture`
- The committee package now matches the approved business flow: evidence merges into governance review, stays evidence-only, and keeps the gate before any future `AdjustmentInputPackage`.

## 2026-04-19
### Modified
- Extended `D:\SM\tests\security_committee_decision_package_cli.rs` with two additional governance-boundary tests.
- Added one richer-payload test so the committee package now exercises a full `SecurityCapitalRebalanceSimulationItem` shape instead of relying only on the backward-compatible default-field path.
- Added one governance-gate test so upstream evidence carrying execution-like flags still results in an evidence-only `security_committee_decision_package` output.
### Why
- After Task 7 landed, two residual risks remained: future callers could depend only on the default-field compatibility path for capital rebalance evidence, and later changes could accidentally relay execution-facing flags through the committee package.
- The user explicitly asked to freeze these two risks with focused tests before moving on.
### Remaining
- [ ] The real `AdjustmentInputPackage` bridge is still a later task and remains intentionally unimplemented.
- [ ] If future richer capital-rebalance payload fields are added again, expand the richer-payload fixture in the same test file so committee-package coverage stays aligned.
### Risks
- [ ] These new tests freeze the current committee-package boundary, but they do not yet verify a downstream adjustment bridge because that bridge still does not exist.
- [ ] The richer payload is still a fixture-level sample; if the capital-rebalance contract expands further, the sample can drift unless updated together with the contract.
### Closed
- Focused verification passed:
  - `cargo test --target-dir D:\SM\target_task7_tests2 --test security_committee_decision_package_cli -- --nocapture`
- The five single-source-of-truth files were re-checked for this round and required no content change because the task only strengthened test coverage around already-documented boundaries.

## 2026-04-19
### Modified
- Re-ran the approved `bank-parent.v1` `1d` scheme-A experiment with `exclude_main_model_fundamental_group = true` through `D:\SM\target\debug\excel_skill.exe`.
- Produced an isolated ablation output set under `D:\SM\.stockmind_runtime\ablation_runs\fundamental_off_20260419_1545`.
- Updated `D:\SM\findings.md`, `D:\SM\progress.md`, and `D:\SM\task_plan.md` with the new comparison against the latest `2026-04-19T12:35:00+08:00` bank baseline.
### Why
- The user explicitly asked to test whether the current bank `1d` failure mainly comes from dirty or overweighted fundamental features before continuing the larger bank fundamental rebuild.
- The safest way to answer that question was one governed ablation rerun that removes the whole `F` block from the main model while keeping diagnostics auditable.
### Remaining
- [ ] Shift the next root-cause pass toward label design, split coverage, and market-state interaction, because the F-group ablation did not restore generalization.
- [ ] Decide whether the bank fundamental rebuild should continue as a secondary data-quality track or pause until the label/split line is audited further.
### Risks
- [ ] Although explicit high-correlation pairs dropped from `2` to `0`, `mean_walk_forward_accuracy` worsened from `0.5396341463414634` to `0.5304878048780488`, so removing fundamentals alone can make the model thinner without making it smarter.
- [ ] The current bank `1d` weakness still appears regime-sensitive: `technical_only` and `bull_trend` slices remain weak even after the F block is removed.
### Closed
- The real F-group ablation rerun is complete and its outputs are isolated for future audit.
- The project record now explicitly states that current fundamentals are not the primary explanation for the bank `1d` model failure.

## 2026-04-19
### Modified
- Audited the whole retained technical feature family (`T/Q/V`) in the latest bank `1d` baseline instead of only the top four technical drivers.
- Updated `D:\SM\findings.md`, `D:\SM\progress.md`, and `D:\SM\task_plan.md` with the new technical-feature governance audit conclusions.
### Why
- The user clarified that the issue is systemic: even if the current top technical drivers are fixed, other retained indicators can surface with the same forward-meaning problem afterward.
- The safest next step was to audit the technical family as one governed surface and identify which features are current-state descriptors versus forward-direction signals.
### Remaining
- [ ] Turn the current audit into one explicit technical-feature governance proposal before any code change.
- [ ] After the technical governance method is validated on bank `1d`, extend the same audit method to fundamentals and event/disclosure features.
### Risks
- [ ] Several retained technical features still compress opposite forward meanings into one coarse bucket, especially `volume_confirmation`.
- [ ] `momentum_signal` currently appears semantically reversed for the bank `1d` task, so point-fixing one feature without family-level governance can still leave the model unstable.
### Closed
- The current project record now distinguishes a technical-feature governance problem from a narrow one bad indicator problem.
- The bank `1d` next-step recommendation is now grounded in a full `T/Q/V` audit instead of only head-driver intuition.
## 2026-04-19
### Modified
- Added the formal `D:\SM\src\ops\security_adjustment_input_package.rs` module and exposed it on the stock boundary, grouped execution gateway, public catalog, dispatcher, and stock dispatcher route.
- Added the execution-preview adapter `adapt_adjustment_input_package_to_execution_record_request()` in `D:\SM\src\ops\security_execution_record.rs`.
- Updated the post-open single-source documents and graph companion so `AdjustmentInputPackage` is now recorded as an implemented approved-only bridge instead of a planned object.
### Why
- Task 7 needed the real approved downstream bridge after `CommitteeDecisionPackage`, while keeping the legacy adjustment recorder frozen as a compatibility-only consumer.
- The user explicitly approved scheme B: formal contract first, execution and legacy recorder as downstream preview shapes, with no silent side effects.
### Remaining
- [ ] The bridge still previews downstream request shapes only; it does not yet orchestrate runtime execution or persistence side effects.
- [ ] Future work still needs the real governance-output payload shape from the committee/chair line instead of the current minimal approved-governance fixture.
### Risks
- [ ] If later governance payload fields are renamed without updating the bridge contract, the adapter can drift even though the current CLI coverage stays green.
- [ ] `requested_adjustment_type` and `plan_alignment` are currently validated against a fixed string set; new business labels must update both contract code and tests together.
### Closed
- Focused verification passed:
  - `cargo test --target-dir D:\SM\target_task8_red --test security_adjustment_input_package_cli -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task8_red --test security_committee_decision_package_cli -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task8_red --test security_execution_record_cli -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task8_red --test security_monitoring_evidence_package_cli -- --nocapture`
- Graph companion validation passed:
  - `Get-Content -Raw D:\SM\docs\architecture\post_open_position_data_graph.json | ConvertFrom-Json`
- The five source-of-truth files were reviewed in this round; `contract_registry.md`, `decision_log.md`, and `acceptance_criteria.md` changed, while `project_intent.md` and `response_contract.md` required no content change.

## 2026-04-19
### Modified
- Fixed the four `security_chair_resolution_cli` ETF/proxy-history regressions by updating:
  - `D:\SM\src\ops\security_scorecard.rs`
  - `D:\SM\src\ops\security_decision_evidence_bundle.rs`
  - `D:\SM\src\ops\security_external_proxy_backfill.rs`
  - `D:\SM\src\ops\security_decision_committee.rs`
- Added focused unit coverage in `D:\SM\src\ops\security_decision_evidence_bundle.rs` for:
  - ETF alias normalization
  - ETF proxy-complete evidence quality
  - latest ETF proxy date anchoring
- Updated `D:\SM\docs\contract_registry.md`, `D:\SM\docs\decision_log.md`, `D:\SM\docs\acceptance_criteria.md`, `D:\SM\docs\AI_HANDOFF.md`, and `D:\SM\docs\architecture\post_open_position_data_graph.json` to freeze the new ETF runtime rules.
### Why
- Task 7 verification exposed four old ETF/proxy-history failures that blocked broader chair-line confidence and would have made later Task 8 handoff drift-prone.
- The user explicitly required these fixes to be written back as source-of-truth rules instead of staying as undocumented tactical patches.
### Remaining
- [ ] `project_intent.md` and `response_contract.md` were reviewed and did not need content changes in this round.
- [ ] Full-workspace `cargo test` was not run; verification stayed on the affected chair/evidence/committee surfaces.
### Risks
- [ ] New ETF families will still need explicit alias normalization and proxy-family definitions; otherwise the same drift class can return under different labels.
- [ ] The ETF proxy-complete replacement rule is intentionally narrow; future sessions must not over-extend it to ordinary equities.
### Closed
- Focused verification passed:
  - `cargo test --target-dir D:\SM\target_task7_final --lib security_decision_evidence_bundle::tests -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task7_final --test security_chair_resolution_cli -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task7_final --test security_decision_committee_cli -- --nocapture`
- Machine-readable graph validation should continue to parse after the ETF runtime note update.

## 2026-04-19
### Modified
- Upgraded `D:\SM\tests\post_open_position_data_flow_guard.rs` from the pre-bridge assumption set to the current Task 7 mainline state.
- Replaced the old adjustment bridge must not be wired yet assertions with current guard rules:
  - `security_adjustment_input_package` must stay exposed on the public stock boundary, catalog, and dispatcher
  - the bridge must remain preview-only
  - the legacy adjustment recorder must remain separate compatibility infrastructure
  - design / graph / handoff / contract texts must still preserve the committee/chair governance gate
### Why
- Task 8 is the integration-guard slice for the whole post-open flow, and the old guard had become stale after Task 7 formally landed `AdjustmentInputPackage`.
- Leaving the old guard in place would make the test suite reject the now-approved mainline instead of protecting it.
### Remaining
- [ ] `project_intent.md`, `contract_registry.md`, `decision_log.md`, `acceptance_criteria.md`, and `response_contract.md` were reviewed for this round and did not require content changes.
- [ ] Task 9 still needs the broader focused-regression bundle and handoff refresh beyond the guard itself.
### Risks
- [ ] If future sessions change the bridge from preview-only into execution orchestration without updating this guard, the test will become stale in the opposite direction.
- [ ] The guard intentionally checks contract-level strings and ownership edges, so file/method renames must be updated together with the graph/doc sources.
### Closed
- Focused verification passed:
  - `cargo test --target-dir D:\SM\target_task8_guard_red --test post_open_position_data_flow_guard -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task8_guard_green --test post_open_position_data_flow_guard -- --nocapture`

## 2026-04-19
### Modified
- Synced `D:\SM\docs\project_intent.md` wording so the intent now says committee/chair governance must complete before `AdjustmentInputPackage` may be built or consumed, instead of describing it as not yet existing.
### Why
- The formal bridge landed in code during the same task, so the old intent phrasing became stale and could mislead the next AI session.
### Remaining
- [ ] Keep future intent wording aligned when the bridge moves from preview-only into real execution orchestration.
### Risks
- [ ] If future sessions update contracts without rechecking `project_intent.md`, the intent layer can drift even when code and tests are correct.
### Closed
- The intent layer now matches the implemented approved-only bridge boundary.

## 2026-04-19
### Modified
- Extended `D:\SM\src\ops\security_adjustment_input_package.rs` so `security_adjustment_input_package` now accepts either one compatibility `governance_approval` payload or one formal `governance_artifacts` bundle.
- Added artifact-driven governance assembly types and helpers in `D:\SM\src\ops\security_adjustment_input_package.rs`: `SecurityAdjustmentGovernanceArtifacts`, `SecurityAdjustmentSizingDecision`, `resolve_governance_approval()`, and `build_governance_approval_from_artifacts()`.
- Expanded `D:\SM\tests\security_adjustment_input_package_cli.rs` with red-green coverage for the new real-artifact path and ref-drift rejection semantics.
- Updated `D:\SM\docs\contract_registry.md`, `D:\SM\docs\decision_log.md`, `D:\SM\docs\acceptance_criteria.md`, `D:\SM\docs\AI_HANDOFF.md`, and `D:\SM\docs\architecture\post_open_position_data_graph.json` to record the new contract boundary.
### Why
- Task 7 still needed to replace the hand-filled governance fixture with a formal assembly path rooted in real approval/chair/condition artifacts.
- The user fixed the business rule that governance lineage and mathematical sizing must stay connected but not over-fused, so the bridge now assembles approval lineage from artifacts while keeping `sizing_decision` explicit.
### Remaining
- [ ] The direct `governance_approval` path is still a compatibility route and can be retired only after upstream callers switch to the artifact-driven path.
- [ ] Runtime execution and persistence orchestration are still out of scope; the bridge remains preview-only.
- [ ] The unrelated `security_chair_resolution_cli` ETF/proxy-history failures are still open outside this task.
### Risks
- [ ] If `submit_approval_output` changes its `decision_ref` / `approval_ref` / `position_plan` field shape without updating this bridge, the artifact-driven path will hard-fail.
- [ ] Future sessions may incorrectly assume `chair_resolution` or `condition_review` already contain the full adjustment math and try to remove `sizing_decision`.
### Closed
- Focused red-green verification passed for the new bridge path:
  - `cargo test --target-dir D:\SM\target_task7_artifact_red --test security_adjustment_input_package_cli -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task7_artifact_green --test security_adjustment_input_package_cli -- --nocapture`
  - `cargo test --target-dir D:\SM\target_task7_artifact_green --test security_condition_review_cli --test security_committee_decision_package_cli --test security_execution_record_cli -- --nocapture`
- Machine-readable graph validation should continue to parse through PowerShell JSON conversion after the graph companion update.
## 2026-04-19
### Modified
- Added `D:\SM\docs\plans\2026-04-19-technical-feature-governance-design.md` and `D:\SM\docs\plans\2026-04-19-technical-feature-governance-implementation-plan.md` to freeze the approved scheme-B technical-governance slice.
- Extended `D:\SM\src\ops\technical_consultation_basic.rs` with the first direction-aware technical family:
  - `trend_direction_strength`
  - `volume_confirmation_direction`
  - `momentum_continuation_signal`
  - `volatility_regime_signal`
- Updated `D:\SM\src\ops\security_decision_evidence_bundle.rs` to project the new technical fields into the governed raw feature seed.
- Updated `D:\SM\src\ops\security_scorecard_training.rs` so the trainer now:
  - registers the new direction-aware technical fields in the formal feature contract
  - excludes `trend_strength`, `volume_confirmation`, `momentum_signal`, and `atr_ratio_14` from the main model with `excluded_from_main_model_legacy_technical_semantics`
- Updated `D:\SM\tests\security_scorecard_training_cli.rs` and added new unit coverage in `technical_consultation_basic.rs` / `security_scorecard_training.rs` for the new technical-governance contract.
- Ran one real `bank-parent.v1` `1d` retrain and produced:
  - `D:\SM\.stockmind_runtime\scorecard_artifacts\a_share_equity_1d_direction_head__candidate_2026_04_19T20_30_00_08_00.json`
  - `D:\SM\.stockmind_runtime\scorecard_training_diagnostics\a_share_equity_1d_direction_head__candidate_2026_04_19T20_30_00_08_00.json`
  - `D:\SM\.stockmind_runtime\scorecard_model_registry\a_share_equity_1d_direction_head__candidate_2026_04_19T20_30_00_08_00.json`
### Why
- The user approved scheme B: stop treating the bank `1d` problem as one or two bad indicators and instead build a governed technical-feature recovery method.
- The latest audit showed the old coarse technical fields compress opposite forward meanings, so they needed to stay auditable but leave the main model.
### Remaining
- [ ] Fix bucket-support sparsity so the new direction-aware technical fields can survive the real train-time support gate instead of being filtered for `insufficient_bin_support`.
- [ ] Re-run the next real retrain after support geometry is adjusted and compare whether the model finally retains the new technical family.
### Risks
- [ ] The real rerun did not improve generalization yet: `mean_walk_forward_accuracy` fell from `0.5396341463414634` to `0.4878048780487805`, although `test.accuracy` improved slightly from `0.4296875` to `0.4375`.
- [ ] The semantic rewrite is wired correctly, but the new technical fields are still too sparse for the current support threshold, so the main model is not truly using them yet.
### Closed
- Focused and adjacent verification passed:
  - `cargo test technical_consultation_basic --lib -- --nocapture`
  - `cargo test security_scorecard_training --lib -- --nocapture`
  - `cargo test --test security_scorecard_training_cli -- --nocapture`
  - `cargo test --test security_feature_snapshot_cli -- --nocapture`
  - `cargo test --test security_analysis_fullstack_cli -- --nocapture`
- The legacy coarse technical fields now leave the main model cleanly while staying visible in diagnostics.
## 2026-04-19
### Modified
- Extended `D:\SM\docs\plans\2026-04-19-technical-feature-governance-design.md` and `D:\SM\docs\plans\2026-04-19-technical-feature-governance-implementation-plan.md` with the approved Scheme C follow-up.
- Updated `D:\SM\src\ops\technical_consultation_basic.rs` to coarsen the new directional technical vocabulary:
  - `trend_direction_strength -> bullish_directional / bearish_directional / range_or_weak`
  - `volume_confirmation_direction -> bullish_participation / bearish_participation / neutral_or_fading`
  - `momentum_continuation_signal -> bullish_continuation / bearish_continuation / exhausted_or_neutral`
  - `volatility_regime_signal -> stable_trend / stress_regime / range_chop`
- Updated `D:\SM\src\ops\security_scorecard_training.rs` to resolve sparse-support threshold per feature and apply a dedicated relaxed threshold to:
  - `trend_direction_strength`
  - `volume_confirmation_direction`
  - `momentum_continuation_signal`
  - `volatility_regime_signal`
- Added/updated TDD coverage in:
  - `D:\SM\src\ops\technical_consultation_basic.rs`
  - `D:\SM\src\ops\security_scorecard_training.rs`
- Rebuilt `excel_skill` and completed one real `bank-parent.v1` `1d` rerun:
  - `D:\SM\.stockmind_runtime\scorecard_artifacts\a_share_equity_1d_direction_head__candidate_2026_04_19T22_45_00_08_00.json`
  - `D:\SM\.stockmind_runtime\scorecard_training_diagnostics\a_share_equity_1d_direction_head__candidate_2026_04_19T22_45_00_08_00.json`
  - `D:\SM\.stockmind_runtime\scorecard_model_registry\a_share_equity_1d_direction_head__candidate_2026_04_19T22_45_00_08_00.json`
### Why
- The first direction-aware technical rerun was wired correctly but failed to change the real main model because all four new technical fields were filtered for `insufficient_bin_support`.
- The approved Scheme C response was to solve both sides of the blocker together:
  - coarsen buckets
  - relax sparse support only for the new directional technical family
### Remaining
- [ ] Decide whether to remove retained `trend_bias` now that `trend_direction_strength` is alive and currently perfectly correlated with it in diagnostics.
- [ ] Decide whether to do one more coarsening pass for `volume_confirmation_direction` and `volatility_regime_signal`, which still fail sparse support even after Scheme C.
### Risks
- [ ] `production_readiness` remains `caution` after the Scheme C rerun.
- [ ] `trend_bias` and `trend_direction_strength` now appear as a perfect high-correlation pair in the real retained model.
- [ ] `volume_confirmation_direction` and `volatility_regime_signal` still do not survive the real bank-pool support gate.
### Closed
- TDD cycle completed and verified with:
  - `cargo test trend_direction_strength --lib -- --nocapture`
  - `cargo test volume_confirmation_direction --lib -- --nocapture`
  - `cargo test momentum_continuation_signal --lib -- --nocapture`
  - `cargo test volatility_regime_signal --lib -- --nocapture`
  - `cargo test governance_policy_uses_dedicated_sparse_support_for_directional_technical_features --lib -- --nocapture`
  - `cargo test governance_policy_keeps_global_sparse_support_for_ordinary_features --lib -- --nocapture`
  - `cargo test technical_consultation_basic --lib -- --nocapture`
  - `cargo test security_scorecard_training --lib -- --nocapture`
  - `cargo test --test security_scorecard_training_cli -- --nocapture`
- Real rerun improved the previous `2026-04-19T20:30:00+08:00` result:
  - `test.accuracy = 0.4375 -> 0.46875`
  - `mean_walk_forward_accuracy = 0.4878048780487805 -> 0.5060975609756098`
  - retained feature count `17 -> 19`
## 2026-04-19
### Modified
- Normalized `.trae/CHANGELOG_TASK.md` into a stable UTF-8 task journal shape with English section headers (`Modified`, `Why`, `Remaining`, `Risks`, `Closed`).
- Restored the first clean historical segment from `HEAD`, repaired later malformed heading blocks, and rewrote one fully corrupted replay/pipeline entry into an English summary so the document remains readable.
- Split merged date headings and removed several structure-drift artifacts that had caused later sections to collapse into the wrong heading labels.
### Why
- The task journal had accumulated mojibake and malformed section boundaries, which made downstream AI handoff and human review unreliable.
- The user explicitly asked to clean the file, and allowed English replacements where damaged Chinese could not be trusted.
### Remaining
- [ ] If desired, we can still do a second-pass editorial cleanup that converts the remaining older Chinese content to English prose for fully uniform style.
### Risks
- [ ] PowerShell `Get-Content` can still render some valid UTF-8 Chinese as mojibake in this terminal, so file-content verification should prefer UTF-8-aware editors or Python reads over console appearance alone.
- [ ] The journal now has stable structure again, but historical wording remains mixed Chinese/English by design because this pass prioritized integrity over stylistic rewriting.
### Closed
- Verified there are no merged date headings left in `.trae/CHANGELOG_TASK.md`.
- Verified there are no obviously malformed `### Closed` sections immediately followed by unchecked bullets.
- Verified the cleaned file reads as UTF-8 through Python and preserves the updated task history.
## 2026-04-19
### Modified
- Added `D:\SM\tests\security_closed_position_archive_cli.rs` as the Task 9 red-green contract for catalog visibility, closed-record success, optional review/contract enrichment, and hard-fail identity drift.
- Added `D:\SM\src\ops\security_closed_position_archive.rs` and wired the new tool through `D:\SM\src\ops\stock.rs`, `D:\SM\src\ops\stock_execution_and_position_management.rs`, `D:\SM\src\tools\catalog.rs`, `D:\SM\src\tools\dispatcher.rs`, and `D:\SM\src\tools\dispatcher\stock_ops.rs`.
- Updated post-open source-of-truth documents so `ClosedPositionArchive` is no longer planned-only:
  - `D:\SM\docs\contract_registry.md`
  - `D:\SM\docs\decision_log.md`
  - `D:\SM\docs\acceptance_criteria.md`
  - `D:\SM\docs\AI_HANDOFF.md`
  - `D:\SM\docs\architecture\post_open_position_data_graph.json`
  - `D:\SM\docs\plans\2026-04-18-post-open-position-data-system-implementation-plan.md`
  - `D:\SM\task_plan.md`
  - `D:\SM\progress.md`
### Why
- Task 9 was the last missing formal object in the minimum coherent post-open lifecycle, so leaving `ClosedPositionArchive` as design-only would keep the lifecycle incomplete.
- The approved seam is closure-state driven, which means the archive must anchor on the closed execution record and only optionally enrich from review or contract context.
### Remaining
- [ ] If later sessions add more archive-specific runtime persistence or reporting, keep the graph JSON line-level mappings aligned with the new symbols in the same task.
### Risks
- [ ] The archive currently depends on execution-record/account identity being populated correctly; if future callers produce blank execution-record `account_id`, archive creation will hard-fail by design.
- [ ] Future refactors must update both the graph JSON and source-of-truth contract docs together, otherwise graph-first code reading may drift again.
### Closed
- Verified RED first with `cargo test --test security_closed_position_archive_cli -- --nocapture`, which failed on `unsupported tool: security_closed_position_archive` before implementation.
- Verified GREEN and focused regression with:
  - `cargo test --test security_closed_position_archive_cli -- --nocapture`
  - `cargo test --test security_account_open_position_snapshot_cli -- --nocapture`
  - `cargo test --test security_adjustment_input_package_cli -- --nocapture`
  - `cargo test --test security_post_trade_review_cli -- --nocapture`
## 2026-04-19
### Modified
- Reorganized the live `D:\SM\docs\` tree into the approved audit taxonomy:
  - `overview`
  - `governance`
  - `product`
  - `plans/design`
  - `plans/execution`
  - `evidence`
  - `handoff`
  - `skills`
  - `archive`
- Rewrote the repository `README.md` as a pure entry document for navigation and reading order, and added bucket-level `README.md` files so each document class has an explicit audit entry point.
- Moved the existing live documents into their new buckets, removed the old `docs/architecture` and `docs/fix` layout, and updated high-signal repository references to the new canonical paths.
- Clarified `D:\SM\docs\handoff\AI_HANDOFF.md` so the two `D:\Rust\Excel_Skill\...` paths are explicitly marked as external historical references, while the current `D:\SM` canonical documents are listed beside them.
### Why
- The previous `docs/` layout had become too flat and mixed design, governance, handoff, and archive material together, which made quality audit and AI handoff harder than necessary.
- The approved Scheme B goal was not only to move files, but to turn documentation layout into a stable navigation contract that later agents can follow without guessing.
### Remaining
- [ ] Decide later whether `docs/evidence/` should absorb additional verification artifacts beyond its current index-only placeholder role.
### Risks
- [ ] Historical records in `.trae/CHANGELOG_TASK.md` intentionally still contain legacy paths because rewriting history would damage traceability.
- [ ] Some design and execution plans still describe the old layout as migration background; that wording is intentional and should not be mistaken for active repository navigation.
### Closed
- Verified the `docs/` tree matches the approved taxonomy buckets and contains bucket-level entry `README.md` files.
- Verified active markdown references no longer point to the old `docs/architecture` or `docs/fix` locations.
- Verified `D:\SM\docs\governance\post_open_position_data_graph.json` still parses successfully after the document moves.
## 2026-04-19
### Modified
- Hardened Task 9 closure-archive coverage in `D:\SM\tests\security_closed_position_archive_cli.rs` with five additional CLI boundary tests for:
  - missing `actual_exit_date`
  - missing `exit_reason`
  - missing execution-record `account_id`
  - `post_trade_review.execution_record_ref` mismatch
  - `position_contract.account_id` mismatch
### Why
- `ClosedPositionArchive` was already implemented, but the CLI regression layer still did not lock several explicit hard-fail branches that exist in the archive validator.
- This round keeps Task 9 honest as the final closed-position object in the minimum coherent post-open lifecycle by freezing incomplete closure facts and cross-source identity drift at the test boundary.
### Remaining
- [ ] If later sessions add archive consumers such as query/report/export tools, keep these same hard-fail assumptions visible instead of reinterpreting the archive as a soft summary object.
### Risks
- [ ] This round adds coverage only; it does not yet add downstream archive indexing, reporting, or persistence beyond the current formal object builder.
### Closed
- Verified with:
  - `cargo test --test security_closed_position_archive_cli -- --nocapture`
  - `cargo test --test security_post_trade_review_cli -- --nocapture`
- Result: all `10` Task 9 archive tests passed, so no production-code change was required in this hardening slice.
## 2026-04-19
### Modified
- Added the approved report-layer design document `D:\SM\docs\plans\design\2026-04-19-account-closed-position-review-report-design.md` for the first account-interval retrospective report object.
- Wrote the same constraints back into the governed source-of-truth set:
  - `D:\SM\docs\product\project_intent.md`
  - `D:\SM\docs\governance\contract_registry.md`
  - `D:\SM\docs\governance\decision_log.md`
  - `D:\SM\docs\governance\acceptance_criteria.md`
  - `D:\SM\docs\governance\response_contract.md`
- Updated `D:\SM\docs\handoff\AI_HANDOFF.md` so future sessions know the next approved downstream consumer after `Task 9` is `AccountClosedPositionReviewReport`, and that it is design-approved but not yet implemented.
### Why
- The user explicitly required the report work to be split into auditable documents instead of leaving key rules inside one natural-language discussion.
- This round freezes the first report-layer boundary before any implementation planning so later AI sessions can distinguish archive truth, optional review enrichment, and account-interval reporting responsibilities.
### Remaining
- [ ] Create the implementation plan for `AccountClosedPositionReviewReport` after the user confirms the design baseline is complete.
- [ ] When implementation starts, add the report object to the graph/ownership artifacts if the user wants the machine-readable flow extended to include downstream consumers.
### Risks
- [ ] The report-layer object is approved only at design level; no code or tests exist yet.
- [ ] Future sessions could still over-fuse report generation with archive mutation or governance execution if they skip the updated source-of-truth files.
### Closed
- Verified the new design document exists and is referenced by the relevant governed source-of-truth files.
- Verified handoff now explicitly distinguishes `ClosedPositionArchive` from the planned downstream `AccountClosedPositionReviewReport`.
## 2026-04-19
### Modified
- Corrected the retrospective-reporting direction after the user clarified that this line must stay pure mathematical / pure data support.
- Rewrote `D:\SM\docs\plans\design\2026-04-19-account-closed-position-review-report-design.md` into a boundary note that freezes:
  - structured retrospective data support stays in the core
  - final audit/report writing belongs to later Skill-layer consumers
- Wrote the correction back into:
  - `D:\SM\docs\product\project_intent.md`
  - `D:\SM\docs\governance\contract_registry.md`
  - `D:\SM\docs\governance\decision_log.md`
  - `D:\SM\docs\governance\acceptance_criteria.md`
  - `D:\SM\docs\governance\response_contract.md`
  - `D:\SM\docs\handoff\AI_HANDOFF.md`
### Why
- The earlier design direction had promoted account-interval retrospective reporting into a formal core object, but the user explicitly corrected that report writing should be handled later by Skill and must not blur into the mathematical core.
- This correction keeps the post-open line aligned with the fixed architecture rule: pure data/mathematical support first, narrative/report generation later.
### Remaining
- [ ] If future work needs deterministic retrospective selectors or aggregates, define them explicitly as structured data support rather than as report-writing objects.
### Risks
- [ ] The filename of the existing design note still reflects the earlier reporting topic even though its content is now a boundary note; future sessions must read the current content instead of inferring intent from the filename alone.
### Closed
- Verified the governed source-of-truth files no longer treat `AccountClosedPositionReviewReport` as a current formal mathematical-core object.
- Verified handoff now states that retrospective reporting belongs to later Skill-layer consumers.
## 2026-04-19
### Modified
- Reworked `D:\SM\README.md` from a document-tree-first entry into a business-facing repository entry for the post-open mainline.
- Added a compact mainline-status summary and one explicit business usage flow from `ApprovedOpenPositionPacket` through `ClosedPositionArchive`.
- Clarified in `D:\SM\README.md` that retrospective reporting remains outside the mathematical core and belongs to later Skill-layer consumers.
- Updated `D:\SM\docs\overview\README.md` so it explicitly stays focused on documentation architecture while the root README owns the business-facing entry role.
### Why
- The user clarified that AI handoff already has its own dedicated document, so the root README should serve business understanding and usage flow instead of duplicating handoff/navigation concerns.
- This keeps the repository entry aligned with the current architecture: pure-data core first, reporting later, and clear separation between business flow and document taxonomy.
### Remaining
- [ ] If the mainline grows beyond `Task 9`, refresh the README status section so it continues to reflect the actual implemented lifecycle instead of becoming stale.
### Risks
- [ ] The README now intentionally emphasizes business flow over engineering onboarding, so future edits should avoid drifting it back into a second handoff document.
### Closed
- Verified `D:\SM\README.md` now presents repository purpose, mainline status, business usage flow, current boundaries, and deeper reading links.
- Verified `D:\SM\docs\overview\README.md` now explicitly positions itself as the documentation-architecture entry rather than the business workflow entry.
## 2026-04-19
### Modified
- Added the approved next-stage design baseline `D:\SM\docs\plans\design\2026-04-19-p10-p12-institution-grade-portfolio-core-design.md`.
- Captured the next-stage expansion decision in `D:\SM\docs\governance\decision_log.md`:
  - `P10-P12` is the next approved stage after `Task 9`
  - `P10` = account objective normalization
  - `P11` = unified portfolio replacement solver
  - `P12` = governed portfolio allocation decision
- Updated `D:\SM\docs\handoff\AI_HANDOFF.md` so future sessions know `P10-P12` is approved in design but still planned-only.
### Why
- The user asked to continue beyond the mainline closure and refine the existing `P10-P22` roadmap using the project standards and stage-complete delivery logic.
- This round freezes the next expansion stage clearly enough that later implementation planning can start from one approved portfolio-core contract instead of ad hoc roadmap memory.
### Remaining
- [ ] Write the dedicated implementation plan for `P10-P12`.
- [ ] Decide whether to split that implementation plan into three task slices immediately or keep one stage-wide execution plan first.
### Risks
- [ ] The new stage objects are design-approved only; they are not current formal contracts yet and must not be treated as implemented.
- [ ] Future sessions could still drift if they skip the explicit planned-only note and register `P10-P12` objects too early.
### Closed
- Verified the new design document exists and contains the approved `P10 / P11 / P12` decomposition.
- Verified `contract_registry.md` still does not register `AccountObjectiveContract`, `PortfolioCandidateSet`, `PortfolioReplacementPlan`, or `PortfolioAllocationDecision` as current formal contracts.
- Verified `decision_log.md` and `AI_HANDOFF.md` both now state that `P10-P12` is the next approved but still planned-only expansion stage.
## 2026-04-19
### Modified
- Added the execution plan `D:\SM\docs\plans\execution\2026-04-19-p10-p12-institution-grade-portfolio-core-implementation-plan.md`.
- Split `P10-P12` into implementation-ready tasks for:
  - `P10` account objective contract
  - `P10` candidate-set normalization
  - `P11` unified portfolio replacement plan
  - `P12` portfolio allocation decision
  - stage flow guard
  - governance write-back
  - final focused verification sweep
### Why
- The approved stage design now needed one concrete execution path so later implementation does not drift into ad hoc file choices, partial TDD, or premature contract registration.
- This plan keeps the future-stage boundary clean by specifying when governance files should change and when they should remain untouched.
### Remaining
- [ ] Choose an execution mode for the plan: subagent-driven in this session, or separate execution session.
### Risks
- [ ] The plan assumes the new portfolio-core modules will live as dedicated files instead of being merged into `security_portfolio_position_plan.rs`; if implementation discovers a hard blocker, the design docs and plan will need synchronized revision.
### Closed
- Verified the implementation plan file exists and follows the required header/structure.
- Verified the plan explicitly distinguishes planned-only stage objects from later governance registration work.
## 2026-04-19
### Modified
- Added `D:\SM\src\ops\security_account_objective_contract.rs` to land the first implemented `P10` contract pair:
  - `SecurityAccountObjectiveContractDocument`
  - `SecurityPortfolioCandidateSet`
- Wired the new public tool `security_account_objective_contract` through:
  - `D:\SM\src\ops\stock.rs`
  - `D:\SM\src\ops\stock_execution_and_position_management.rs`
  - `D:\SM\src\tools\catalog.rs`
  - `D:\SM\src\tools\dispatcher.rs`
  - `D:\SM\src\tools\dispatcher\stock_ops.rs`
- Verified the dedicated CLI regression file `D:\SM\tests\security_account_objective_contract_cli.rs` now passes end to end.
### Why
- The approved `P10-P12` implementation plan starts with one account-level objective shell and one governed candidate set before any unified replacement solver is introduced.
- This change freezes the first executable `P10` boundary so later work can build on a real account objective contract instead of a planned-only design note.
### Remaining
- [ ] Extend `D:\SM\src\ops\security_account_objective_contract.rs` for Task 2 candidate-set thickening, including richer normalization and additional drift guards if new tests require them.
- [ ] Land `P11` unified portfolio replacement math and `P12` allocation decision after the next RED tests are written and verified.
- [ ] Perform the later governance write-back task so source-of-truth registry documents reflect the implemented stage objects at the approved time.
### Risks
- [ ] `P10` currently validates the approved-candidate-only boundary and basic account/objective guards, but it does not yet solve duplicate-symbol drift or full candidate competition semantics; those remain Task 2 scope.
- [ ] Governance source-of-truth files still treat `P10-P12` as planned-only at the stage level, so future sessions must not infer that `P11/P12` are already implemented.
### Closed
- Verified `cargo test --test security_account_objective_contract_cli -- --nocapture` passed with `6 passed; 0 failed`.
- Verified the new tool appears in the public tool catalog and no longer falls through to `unsupported tool`.
- Verified the implemented boundary matches the approved `P10` design slice only: governed inputs in, account objective plus candidate set out, with hard-fail on cross-account drift, missing capital base, conflicting constraints, and non-approved entrants.
## 2026-04-19
### Modified
- Extended `D:\SM\tests\security_account_objective_contract_cli.rs` for `Task 2` candidate-set normalization coverage:
  - row-level normalization metadata on live and approved entrant rows
  - duplicate-symbol drift rejection
  - explicit mixed-account candidate rejection
- Updated `D:\SM\src\ops\security_account_objective_contract.rs` so the `PortfolioCandidateSet` rows now preserve:
  - `candidate_status`
  - `account_id`
  - `capital_base_amount`
  - `selection_boundary_ref`
- Added duplicate symbol hard-fail inside the `P10` candidate-set normalization boundary.
### Why
- `Task 2` in the approved `P10-P12` implementation plan requires a thicker candidate-set contract before any `P11` unified replacement solver can safely consume it.
- This keeps row identity and boundary metadata explicit instead of forcing later math to infer live-vs-new status from array location or external context.
### Remaining
- [ ] If `P11` needs a single merged candidate vector instead of parallel live/new arrays, add that shape only after writing the next RED tests.
- [ ] Land the `P11` unified replacement plan contract and solver entry; `Task 2` does not include allocation solving.
- [ ] Revisit whether additional duplicate drift guards are needed for future optional inputs such as account-rebase-driven entrants.
### Risks
- [ ] The current `P10` candidate set still keeps live rows and approved entrant rows in separate arrays; that is sufficient for the approved Task 2 contract, but a later solver may still want one more explicit merged projection.
- [ ] Duplicate-symbol rejection currently protects the normalized candidate set surface only; it does not yet encode any later replacement preference or incumbent-vs-entrant tie-break semantics.
### Closed
- Verified the new `Task 2` RED tests failed for the expected reasons before implementation: missing row metadata and missing duplicate-symbol rejection.
- Verified `cargo test --test security_account_objective_contract_cli -- --nocapture` passed with `9 passed; 0 failed`.
- Verified the implementation still stays inside `P10` and does not pull `P11/P12` solver or governance behavior into the mathematical core.
## 2026-04-19
### Modified
- Reused the validated April replay baseline `D:\SM\.stockmind_runtime\analysis\bank_1d_april_replay_old_vs_new_2026_04_19_1903.json` and added one new three-way comparison artifact:
  - `D:\SM\.stockmind_runtime\analysis\bank_1d_april_replay_old_vs_full_vs_techonly_2026_04_19_1856.json`
- Completed the first same-window bank-pool `1d` April replay for the pure-technical ablation model:
  - model path `D:\SM\.stockmind_runtime\ablation_runs\fundamental_off_20260419_1545\scorecard_artifacts\a_share_equity_1d_direction_head__candidate_2026_04_19T15_45_00_08_00.json`
- Re-ran fresh verification for the runtime feature-projection fix and replay artifact health:
  - `cargo test build_raw_feature_snapshot_includes_bank_volume_percentile_for_bank_runtime_scoring --lib -- --nocapture`
  - `cargo test --test security_scorecard_cli -- --nocapture`
  - analysis JSON integrity probe for the new three-way replay artifact
### Why
- The active recovery path is now explicitly `1d -> technical-side repair first`, while the user asked to keep fundamentals temporarily out of the main judgment.
- After the runtime `bank_volume_percentile_3d` null bug was fixed, the next lowest-cost high-signal step was to replay the already-trained pure-technical ablation under the repaired runtime instead of starting another retraining branch.
### Remaining
- [ ] Explain the three-way April replay outcome to the user in one concise table plus root-cause wording, especially why pure-technical improves hit-rate/correlation but still leaves long-short spread negative.
- [ ] Decide whether the next technical cleanup should target bottom-bucket discrimination, probability dispersion, or repeated top-rank concentration around a small symbol subset.
- [ ] If the user wants event attribution, add a dated external timeline for the April strait-risk window and mark which replay dates likely overlap macro shock rather than model failure.
### Risks
- [ ] The pure-technical model still does not flip `avg_long_short_excess_1d` positive, so it cannot yet be described as a recovered ranking model.
- [ ] The pure-technical replay now shows stronger `Top3` hit-rate but much lower universe probability dispersion, which suggests the model is better at avoiding some bad tops while still weak at confidently isolating the worst names.
- [ ] Top-rank concentration shifted rather than disappeared: the replay repeatedly places `601916.SH` at the top, so there is still a concentration / calibration risk to audit before any production recovery claim.
### Closed
- Verified the repaired runtime still projects `bank_volume_percentile_3d` into live scoring and the focused regression remains green.
- Verified the new three-way replay artifact contains `462` scored rows, `11` evaluable dates, and `bank_volume_percentile_3d_null_rate = 0.0` for the pure-technical branch.
- Verified the pure-technical branch improved versus the old baseline on:
  - `top3_member_hit_rate: 0.3636 -> 0.6061`
  - `pearson_prob_vs_excess: -0.0333 -> 0.0314`
  while `avg_long_short_excess_1d` remained negative at `-0.00168`.
## 2026-04-19
### Modified
- Added the new `P11` CLI contract test file `D:\SM\tests\security_portfolio_replacement_plan_cli.rs`.
- Added the first unified replacement-plan module `D:\SM\src\ops\security_portfolio_replacement_plan.rs`.
- Wired the new public tool `security_portfolio_replacement_plan` through:
  - `D:\SM\src\ops\stock.rs`
  - `D:\SM\src\ops\stock_execution_and_position_management.rs`
  - `D:\SM\src\tools\catalog.rs`
  - `D:\SM\src\tools\dispatcher.rs`
  - `D:\SM\src\tools\dispatcher\stock_ops.rs`
- Landed the first deterministic `P11` replacement-plan output sections:
  - `current_weights`
  - `target_weights`
  - `entry_actions`
  - `trim_actions`
  - `exit_actions`
  - `replacement_pairs`
  - `capital_migration_plan`
### Why
- `Task 3` in the approved `P10-P12` implementation plan requires one formal `P11` contract that consumes only the implemented `P10` outputs and freezes the first account-level unified replacement solve.
- This change creates the minimum deterministic solver pass needed to move from “account objective + candidate set” into one governed replacement plan without pulling execution friction, stress scenarios, or LLM logic into the mathematical core.
### Remaining
- [ ] Harden `D:\SM\src\ops\security_portfolio_replacement_plan.rs` in `Task 4` for richer solver boundaries such as rebase-aware migration, more explicit no-feasible-solution branches, and simultaneous add + replace + exit cases.
- [ ] Land the `P12` portfolio allocation decision contract after the next RED tests are written and verified.
- [ ] Perform the later governance write-back task so source-of-truth registry documents reflect implemented `P11` objects only at the approved stage.
### Risks
- [ ] The current `P11` solver is intentionally minimal and deterministic: it does not yet include Kelly integration, volatility targeting, stress filtering, or multi-objective optimization beyond the first constraint checks.
- [ ] `replacement_pairs` currently link trim-derived outgoing weight to approved entrant rows in a simple deterministic way; future hardening may need richer incumbent-vs-entrant matching semantics.
- [ ] The current contract keeps `P11` inside the pure mathematical core and does not yet freeze any final governed allocation decision; future sessions must not confuse this with `P12`.
### Closed
- Verified the new `Task 3` RED tests failed for the expected reason before implementation: `unsupported tool: security_portfolio_replacement_plan`.
- Verified `cargo test --test security_portfolio_replacement_plan_cli -- --nocapture` passed with `5 passed; 0 failed`.
- Verified `cargo test --test security_account_objective_contract_cli --test security_portfolio_replacement_plan_cli -- --nocapture` passed with `14 passed; 0 failed`.
- Verified the implemented boundary matches the approved `P11` design slice only: it consumes the formal `P10` outputs, emits one unified replacement plan, and hard-fails on infeasible allocation, weight non-conservation, and cross-account drift.
## 2026-04-19
### Modified
- Extended `D:\SM\tests\security_portfolio_replacement_plan_cli.rs` for `Task 4` P11 hardening coverage:
  - row-level approved-candidate boundary drift rejection
  - rebase-aware capital migration metadata
  - structured conflict-resolution summary
  - simultaneous add + replace + exit action summary
- Updated `D:\SM\src\ops\security_portfolio_replacement_plan.rs` so the `P11` contract now also preserves:
  - row-level candidate/live boundary validation
  - optional `account_rebase_snapshot` context
  - `capital_base_amount_before/after`
  - `rebase_policy`
  - `rebase_context_applied`
  - `solver_action_summary`
  - `conflict_resolution_summary`
- Updated `D:\SM\docs\handoff\AI_HANDOFF.md` to freeze the current stage boundary:
  - `P10 / Task 1-2` implemented
  - `P11 / Task 3-4` implemented
  - `P12` not implemented
### Why
- `Task 4` is the approved hardening pass that closes `P11`, so the replacement-plan solver needed to become more auditable and more explicit about candidate boundaries, rebase context, and mixed action outcomes.
- The upload handoff also needed to reflect that `P10` and `P11` are now implemented while `P12` remains future work.
### Remaining
- [ ] Start `P12 / Task 5` with a new RED test for the governed portfolio allocation decision contract.
- [ ] Later governance write-back still needs to move implemented stage objects into the formal source-of-truth registry at the approved time.
- [ ] If future solver work introduces real Kelly or volatility-target inputs, replace the current fallback-only conflict summary with metric-backed resolution traces.
### Risks
- [ ] The current conflict-resolution summary is still deterministic fallback metadata; it does not yet represent a full multi-objective optimizer with real Kelly/vol-target inputs.
- [ ] `P11` is now considered closed for the current implementation plan, but broader repository regression is still not claimed because the workspace contains unrelated active work.
- [ ] The branch worktree still contains many unrelated dirty files and runtime artifacts, so upload should stage only the current delivery slice.
### Closed
- Verified the new `Task 4` RED tests failed for the expected reasons before implementation: missing row-level boundary rejection, missing rebase metadata, missing conflict summary, and missing structured mixed-action summary.
- Verified `cargo test --test security_portfolio_replacement_plan_cli -- --nocapture` passed with `9 passed; 0 failed`.
- Verified `cargo test --test security_account_objective_contract_cli --test security_portfolio_replacement_plan_cli -- --nocapture` passed with `18 passed; 0 failed`.
- Verified `P11` now matches the approved current-scope closeout: approved-candidate boundary, no-feasible-solution path, capital migration after rebase input, conflict-resolution summary, and simultaneous add + replace + exit handling.
## 2026-04-19
### Modified
- Prepared the `P10/P11` delivery slice for safe Git upload by isolating only the account-objective and portfolio-replacement implementation files plus their routing and focused handoff artifacts.
### Why
- The active worktree contains many unrelated runtime artifacts and parallel dirty files, so upload preparation had to stay slice-scoped instead of assuming the whole tree was reviewable.
### Remaining
- [ ] Run fresh focused verification against the isolated staged slice before commit and push.
- [ ] Push only the approved `P10/P11` delivery branch contents; do not widen the staged set to unrelated active work.
### Risks
- [ ] The broader repository still contains unrelated dirty changes and untracked artifacts that are intentionally excluded from this upload.
### Closed
- The upload-prep boundary is now explicitly recorded as `P10/P11` only.
## 2026-04-20
### Modified
- Rebuilt a clean upload branch `codex/p10-p11-clean-upload-20260420` from `origin/main` in isolated worktree `C:\wt\smu`.
- Cherry-picked the `P10/P11` portfolio-core contract delivery without carrying the earlier runtime-data commit that contained local `.stockmind_runtime` databases and backfill artifacts.
- Updated `D:\SM\docs\handoff\AI_HANDOFF.md` in the clean branch so the handoff explicitly documents the upload branch, isolated worktree, and the exclusion of runtime database artifacts.
### Why
- The original delivery branch could not be pushed safely because an earlier local-data commit contained multi-GB runtime databases that are reproducible artifacts rather than source-controlled code.
- The user approved a clean-upload path that preserves code, tests, and handoff material while excluding machine-local training/runtime outputs.
### Remaining
- [ ] Run fresh focused verification in the clean upload worktree before pushing.
- [ ] Push only the clean upload branch and confirm the remote branch name for downstream review.
### Risks
- [ ] The clean branch does not include local runtime databases or replay artifacts, so anyone reproducing those results must regenerate them from the documented pipeline.
- [ ] The original working tree at `D:\SM` still contains unrelated dirty changes that remain outside this upload.
### Closed
- The Git delivery boundary is now explicitly frozen as code/tests/handoff only, with `.stockmind_runtime` database payloads excluded from versioned upload.
- Added the new `P14` CLI contract test file `D:\SM\tests\security_portfolio_execution_request_enrichment_cli.rs`.
- Added the new bridge implementation `D:\SM\src\ops\security_portfolio_execution_request_enrichment.rs`.
- Wired the new public tool `security_portfolio_execution_request_enrichment` through:
  `D:\SM\src\ops\stock.rs`,
  `D:\SM\src\ops\stock_execution_and_position_management.rs`,
  `D:\SM\src\tools\catalog.rs`,
  `D:\SM\src\tools\dispatcher.rs`,
  `D:\SM\src\tools\dispatcher\stock_ops.rs`.
- Restored the minimum compatibility surface needed to keep the current branch compiling while landing `P14`:
  `security_investment_manager_entry` and `security_committee_decision_package` re-exports in `D:\SM\src\ops\stock.rs`,
  `predict_numeric_head_value` compatibility alias in `D:\SM\src\ops\security_scorecard.rs`,
  and `empty_fundamental_metrics` / `finalize_fundamental_metrics` exposure in `D:\SM\src\ops\security_analysis_fullstack.rs`.
- Synced the approved governance and handoff records in
  `D:\SM\docs\governance\contract_registry.md`,
  `D:\SM\docs\governance\decision_log.md`,
  `D:\SM\docs\handoff\CURRENT_STATUS.md`,
  and `D:\SM\docs\handoff\HANDOFF_ISSUES.md`.
### Why
- The approved `P14` design freezes the next step as an execution-request enrichment bridge only: it must consume the formal `P13` request package, produce an execution-record-aligned enrichment bundle, and stop before any real apply or runtime write-back.
- The branch also needed a small set of compatibility fixes so the new `P14` RED/GREEN loop could land without widening the scope into unrelated execution or governance rework.
### Remaining
- [ ] The later apply bridge that turns enriched requests into real execution facts is still a separate future phase and must not be folded back into `P14`.
- [ ] The unrelated test issue in `D:\SM\tests\security_chair_resolution_builder_unit.rs` (`missing field 'sma_20'`) is still outside this delivery slice and remains to be handled separately.
- [ ] The root log file `D:\SM\CHANGELOG_TASK.MD` still has encoding problems, so this task journal update was recorded only in `.trae/CHANGELOG_TASK.md`.
### Risks
- [ ] Downstream callers could still misuse the `P14` enriched bundle as if it were a real execution fact; the semantic boundary must stay explicit until the future apply bridge exists.
- [ ] The workspace remains intentionally dirty with many unrelated runtime artifacts and parallel edits, so this entry does not imply that the repository as a whole is review-clean.
### Closed
- Verified `cargo test --test security_portfolio_execution_request_enrichment_cli -- --nocapture` passed with `4 passed; 0 failed`.
- Verified `cargo test --test security_portfolio_execution_request_package_cli -- --nocapture` passed with `4 passed; 0 failed`.
- Verified `cargo test --test security_portfolio_execution_preview_cli -- --nocapture` passed with `4 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p12_enhanced'; cargo test --test security_account_objective_contract_cli --test security_portfolio_replacement_plan_cli --test security_portfolio_allocation_decision_cli -- --nocapture` passed with `25 passed; 0 failed`.
- Verified the implemented `P14` boundary matches the approved contract: consume only `SecurityPortfolioExecutionRequestPackageDocument`, preserve hold semantics, hard-fail malformed lineage/count/date inputs, and avoid calling `security_execution_record` or writing runtime state.
## 2026-04-20
### Modified
- Added the new `P15` CLI contract test file `D:\SM\tests\security_portfolio_execution_apply_cli.rs`.
- Added the new governed apply bridge implementation `D:\SM\src\ops\security_portfolio_execution_apply.rs`.
- Wired the new public tool `security_portfolio_execution_apply` through:
  `D:\SM\src\ops\stock.rs`,
  `D:\SM\src\ops\stock_execution_and_position_management.rs`,
  `D:\SM\src\tools\catalog.rs`,
  `D:\SM\src\tools\dispatcher.rs`,
  `D:\SM\src\tools\dispatcher\stock_ops.rs`.
- Synced the approved governance and handoff records in
  `D:\SM\docs\governance\contract_registry.md`,
  `D:\SM\docs\governance\decision_log.md`,
  `D:\SM\docs\handoff\CURRENT_STATUS.md`,
  and `D:\SM\docs\handoff\HANDOFF_ISSUES.md`.
### Why
- The approved `P15` design freezes the next step as a governed apply bridge only: it must consume the formal `P14` enrichment bundle, execute only `ready_for_apply` rows through a bounded internal adapter, and return one auditable batch-level apply document.
- This route keeps the thick `security_execution_record` compatibility shell behind one internal adapter instead of leaking it as the new public downstream contract.
### Remaining
- [ ] The current `P15` direct adapter is intentionally heuristic: it derives minimum execution-record inputs from enrichment rows plus local history and symbol routing, so broker-fill replay and order-ledger exactness remain future work.
- [ ] The unrelated test issue in `D:\SM\tests\security_chair_resolution_builder_unit.rs` (`missing field 'sma_20'`) is still outside this delivery slice and remains to be handled separately.
- [ ] The root log file `D:\SM\CHANGELOG_TASK.MD` still has encoding problems, so this task journal update was recorded only in `.trae/CHANGELOG_TASK.md`.
### Risks
- [ ] Downstream callers could still over-read the first `P15` apply result as a broker-exact execution ledger; the current contract only guarantees governed ready-row application with explicit partial-success semantics.
- [ ] The workspace remains intentionally dirty with many unrelated runtime artifacts and parallel edits, so this entry does not imply that the repository as a whole is review-clean.
### Closed
- Verified the `P15` RED test failed for the expected reason before implementation: `unsupported tool: security_portfolio_execution_apply`.
- Verified `cargo test --test security_portfolio_execution_apply_cli -- --nocapture` passed with `5 passed; 0 failed`.
- Verified `cargo test --test security_portfolio_execution_request_enrichment_cli -- --nocapture` passed with `4 passed; 0 failed`.
- Verified `cargo test --test security_portfolio_execution_request_package_cli -- --nocapture` passed with `4 passed; 0 failed`.
- Verified `cargo test --test security_portfolio_execution_preview_cli -- --nocapture` passed with `4 passed; 0 failed`.
- Verified `cargo test --test security_execution_record_cli -- --nocapture` passed with `5 passed; 0 failed`.
- Verified the implemented `P15` boundary matches the approved contract: consume only `SecurityPortfolioExecutionRequestEnrichmentDocument`, execute only governed `ready_for_apply` rows, preserve explicit hold and blocked skips, hard-fail malformed enrichment input before execution, and surface `applied` / `partial_success` / `failed` outcomes without exposing `SecurityExecutionRecordRequest` as the public request shell.
## 2026-04-20
### Modified
- Updated the pure builder fixture in `D:\SM\tests\security_chair_resolution_builder_unit.rs` so its local `TechnicalConsultationBasicResult` payload matches the current formal technical-analysis contract.
- Added the missing fixture fields required by the current schema:
  `trend_direction_strength`,
  `volume_confirmation_direction`,
  `momentum_continuation_signal`,
  `volatility_regime_signal`,
  `indicator_snapshot.sma_20`,
  `indicator_snapshot.volume_ratio_3_vs_20`,
  `indicator_snapshot.volume_ratio_5_vs_20`,
  `indicator_snapshot.obv_slope_5d`.
- Refreshed branch-health notes in
  `D:\SM\docs\handoff\CURRENT_STATUS.md`
  and `D:\SM\docs\handoff\HANDOFF_ISSUES.md`
  after the original chair-fixture blocker cleared and a new first blocking regression surfaced.
### Why
- The failing `security_chair_resolution_builder_unit` suite was not exposing a chair-arbitration logic bug; it was failing earlier because its hand-written technical fixture had drifted behind the live `TechnicalConsultationBasicResult` / `TechnicalIndicatorSnapshot` contract.
- Repairing the fixture keeps the builder tests focused on chair behavior while preserving the stricter production contract instead of weakening serde requirements globally.
### Remaining
- [ ] The newly exposed first blocking regression is now `D:\SM\tests\post_open_position_data_flow_guard.rs`, which expects the formal stock boundary to include `security_adjustment_input_package`.
- [ ] The root log file `D:\SM\CHANGELOG_TASK.MD` still has encoding problems, so this task journal update was recorded only in `.trae/CHANGELOG_TASK.md`.
- [ ] The workspace remains intentionally dirty with many unrelated runtime artifacts and parallel edits, so this fix does not imply repository-wide review cleanliness.
### Risks
- [ ] This fix intentionally updates only the pure builder fixture; if other hand-written fixtures still lag behind the technical-analysis contract, similar schema drift can reappear elsewhere.
- [ ] The new first blocker is a post-open boundary/source-guard failure, so full-regression green is still not claimed.
### Closed
- Verified RED with `$env:CARGO_TARGET_DIR='D:\SM\target_chair_fixture_red'; cargo test --test security_chair_resolution_builder_unit -- --nocapture`, which failed on `missing field 'sma_20'`.
- Verified GREEN with `$env:CARGO_TARGET_DIR='D:\SM\target_chair_fixture_green'; cargo test --test security_chair_resolution_builder_unit -- --nocapture`, which passed with `4 passed; 0 failed`.
- Verified the original broader regression moved forward with `$env:CARGO_TARGET_DIR='D:\SM\target_chair_regression_verify'; cargo test -- --nocapture`: the chair fixture no longer blocks, and the next first failure is now `post_open_position_data_flow_guard` expecting `security_adjustment_input_package`.
## 2026-04-20
### Modified
- Restored the formal public route for `D:\SM\src\ops\security_adjustment_input_package.rs` across:
  `D:\SM\src\ops\stock.rs`,
  `D:\SM\src\ops\stock_execution_and_position_management.rs`,
  `D:\SM\src\tools\catalog.rs`,
  `D:\SM\src\tools\dispatcher.rs`,
  and `D:\SM\src\tools\dispatcher\stock_ops.rs`.
- Added the minimum local adapter helper inside `D:\SM\src\ops\security_adjustment_input_package.rs` so the post-open package tool can preview downstream execution-record-aligned requests without widening the public `security_execution_record` surface.
- Updated `D:\SM\docs\governance\contract_registry.md`, `D:\SM\docs\handoff\CURRENT_STATUS.md`, and `D:\SM\docs\handoff\HANDOFF_ISSUES.md` so branch-health records now reflect the restored post-open boundary and the newly exposed next blocker.
### Why
- The current branch already carried the `security_adjustment_input_package` implementation, but its public stock-bus exposure had drifted out of the catalog / dispatcher / grouping surface, so the formal post-open route was missing even though the tool logic still existed.
- Restoring the boundary with one local adapter helper was the smallest governed fix that recovered the public contract without leaking `security_execution_record` as a wider downstream request shell.
- Once that boundary blocker cleared, branch-health truth needed to move forward to the next real first failure instead of continuing to report a solved issue.
### Remaining
- [ ] The current repository-wide first blocker is now `D:\SM\tests\security_analysis_fullstack_cli.rs`, where `security_analysis_fullstack_aggregates_technical_fundamental_and_disclosures` returns `Null` for one field that the test still asserts as `0.92`.
- [ ] The root log file `D:\SM\CHANGELOG_TASK.MD` still has encoding problems, so this task journal update was recorded only in `.trae/CHANGELOG_TASK.md`.
- [ ] The workspace remains intentionally dirty with many unrelated runtime artifacts and parallel edits, so this entry does not imply repository-wide review cleanliness.
### Risks
- [ ] The restored `security_adjustment_input_package` route is intentionally package/preview-only; downstream callers could still misuse it as if it performed real execution or persistence unless the contract note remains visible.
- [ ] The newly exposed `security_analysis_fullstack_cli` failure may come from provider-contract drift, fixture drift, or serializer field loss; root-cause investigation is still required before any fix.
### Closed
- Verified `security_adjustment_input_package` RED-to-GREEN recovery through `$env:CARGO_TARGET_DIR='D:\SM\target_adjustment_input_green'; cargo test --test security_adjustment_input_package_cli -- --nocapture`, which passed with `6 passed; 0 failed`.
- Verified `post_open_position_data_flow_guard` through `$env:CARGO_TARGET_DIR='D:\SM\target_adjustment_input_verify'; cargo test --test post_open_position_data_flow_guard -- --nocapture`, which passed with `2 passed; 0 failed`.
- Verified broader regression progress with `$env:CARGO_TARGET_DIR='D:\SM\target_adjustment_input_verify2'; cargo test -- --nocapture`: the post-open boundary no longer blocks, and the next first failure is now `security_analysis_fullstack_cli` with `left: Null` versus `right: 0.92`.
## 2026-04-20
### Modified
- Restored the broader governed `FundamentalMetrics` contract in `D:\SM\src\ops\security_analysis_fullstack.rs`, including:
  `roa_pct`,
  `pe_ttm`,
  `pb`,
  `dividend_yield`,
  `total_assets`,
  `total_asset_growth_pct`,
  `equity_growth_pct`,
  `asset_liability_ratio`,
  `equity_ratio_pct`,
  `liability_to_equity_ratio_pct`,
  `log_revenue`,
  `log_net_profit`,
  and `pb_vs_roe_gap`.
- Repaired all current metric initializer sites inside `D:\SM\src\ops\security_analysis_fullstack.rs` for Eastmoney latest/history parsing, official JSON parsing, Sina latest parsing, Sina resilient parsing, Sina history parsing, the shared empty helper, and the shared finalization helper.
- Added one governed merge path in `D:\SM\src\ops\security_analysis_fullstack.rs` so Eastmoney-success financial history rows now merge Sina-only bank proxy metrics by `report_period` instead of returning the thinner Eastmoney rows directly.
- Refreshed `D:\SM\docs\handoff\CURRENT_STATUS.md` and `D:\SM\docs\handoff\HANDOFF_ISSUES.md` so branch-health records now move from the solved fullstack blocker to the newly exposed approved-open-position packet blocker.
### Why
- The current branch had drifted `FundamentalMetrics` back to a truncated shape even though the fullstack contract tests, governed-history replay tests, and training-side feature expectations still relied on the wider B1+B2 metric set.
- That drift removed both direct provider fields such as `roa_pct` and derived fields such as `log_revenue`, while also dropping the Eastmoney-plus-Sina merge needed to keep bank proxy metrics in governed storage when Eastmoney succeeds first.
- Once this contract was repaired and re-verified, branch-health truth needed to advance to the next real first failure instead of continuing to report the solved fullstack blocker.
### Remaining
- [ ] The current repository-wide first blocker is now `D:\SM\tests\security_approved_open_position_packet_cli.rs`, where the public stock bus still returns `unsupported tool: security_approved_open_position_packet`.
- [ ] The root log file `D:\SM\CHANGELOG_TASK.MD` still has encoding problems, so this task journal update was recorded only in `.trae/CHANGELOG_TASK.md`.
- [ ] The workspace remains intentionally dirty with many unrelated runtime artifacts and parallel edits, so this entry does not imply repository-wide review cleanliness.
### Risks
- [ ] The restored `FundamentalMetrics` contract now carries both provider-native and derived fields again; future edits that touch only one initializer site can silently reintroduce drift unless the current source guard remains active.
- [ ] The new first blocker is a separate packet/public-route gap, so this repair still does not justify any claim of repository-wide green.
### Closed
- Verified RED with `cargo test --test security_analysis_fullstack_fundamental_metrics_source_guard -- --nocapture`, which originally failed on missing bank proxy field markers before the contract repair.
- Verified GREEN with `cargo test --test security_analysis_fullstack_fundamental_metrics_source_guard -- --nocapture`, which passed with `2 passed; 0 failed`.
- Verified `cargo test --test security_analysis_fullstack_cli -- --nocapture`, which passed with `5 passed; 0 failed`.
- Verified `cargo test --test security_fundamental_history_live_backfill_cli -- --nocapture`, which passed with `4 passed; 0 failed`.
- Verified `cargo test --test security_stock_history_governance_cli -- --nocapture`, which passed with `2 passed; 0 failed`.
- Verified broader regression progress with `$env:CARGO_TARGET_DIR='D:\SM\target_security_analysis_fullstack_verify'; cargo test -- --nocapture`: the fullstack blocker no longer blocks, and the next first failure is now `security_approved_open_position_packet_cli` with `unsupported tool: security_approved_open_position_packet`.
## 2026-04-21
### Modified
- Added a short implementation plan at `D:\SM\docs\plans\2026-04-21-security-approved-open-position-packet-public-route.md` for the approved minimal public-route recovery.
- Restored the formal public route for `D:\SM\src\ops\security_approved_open_position_packet.rs` across:
  `D:\SM\src\tools\catalog.rs`,
  `D:\SM\src\tools\dispatcher.rs`,
  and `D:\SM\src\tools\dispatcher\stock_ops.rs`.
- Refreshed `D:\SM\docs\handoff\CURRENT_STATUS.md` and `D:\SM\docs\handoff\HANDOFF_ISSUES.md` so branch-health records now move from the solved approved-open-position packet blocker to the newly exposed closed-position archive blocker.
### Why
- The approved packet contract already existed in the stock domain and governance docs, but its public stock-bus exposure had drifted out of the catalog and dispatcher stack, so the CLI contract failed at the unsupported-tool boundary instead of reaching the real normalization and validation logic.
- The approved route was restored as a minimal public-surface recovery rather than a schema redesign so this round could stay scoped to the confirmed blocker and let full regression advance to the next unrelated failure.
### Remaining
- [ ] The current repository-wide first blocker is now `D:\SM\tests\security_closed_position_archive_cli.rs`, where the public stock bus still returns `unsupported tool: security_closed_position_archive`.
- [ ] The root log file `D:\SM\CHANGELOG_TASK.MD` still has encoding problems, so this task journal update was recorded only in `.trae/CHANGELOG_TASK.md`.
- [ ] The workspace remains intentionally dirty with many unrelated runtime artifacts and parallel edits, so this entry does not imply repository-wide review cleanliness.
### Risks
- [ ] The restored `security_approved_open_position_packet` route is intentionally intake-only normalization and validation; downstream callers could still over-read it as if it already created a live position contract or execution artifact unless the boundary remains explicit.
- [ ] The newly exposed `security_closed_position_archive` failure may be another public-route drift rather than a behavior bug, but that still requires a fresh design-and-TDD loop before any additional code change.
### Closed
- Verified RED with `$env:CARGO_TARGET_DIR='D:\SM\target_security_approved_packet_red'; cargo test --test security_approved_open_position_packet_cli -- --nocapture`, which failed because the public stock bus returned `unsupported tool: security_approved_open_position_packet`.
- Verified GREEN with `$env:CARGO_TARGET_DIR='D:\SM\target_security_approved_packet_green'; cargo test --test security_approved_open_position_packet_cli -- --nocapture`, which passed with `10 passed; 0 failed`.
- Verified broader regression progress with `$env:CARGO_TARGET_DIR='D:\SM\target_security_approved_packet_full_verify'; cargo test -- --nocapture`: the approved-open-position packet blocker no longer blocks, and the next first failure is now `security_closed_position_archive_cli` with `unsupported tool: security_closed_position_archive`.
## 2026-04-21
### Modified
- Added a short implementation plan at `D:\SM\docs\plans\2026-04-21-security-closed-position-archive-public-route.md` for the approved minimal public-route recovery.
- Restored the formal public route for `D:\SM\src\ops\security_closed_position_archive.rs` across:
  `D:\SM\src\ops\stock.rs`,
  `D:\SM\src\ops\stock_execution_and_position_management.rs`,
  `D:\SM\src\tools\catalog.rs`,
  `D:\SM\src\tools\dispatcher.rs`,
  and `D:\SM\src\tools\dispatcher\stock_ops.rs`.
- Refreshed `D:\SM\docs\handoff\CURRENT_STATUS.md` and `D:\SM\docs\handoff\HANDOFF_ISSUES.md` so branch-health records now move from the solved closed-position archive blocker to the newly exposed committee decision package blocker.
### Why
- The closed-position archive contract already existed in the stock domain, but its formal stock-boundary export, grouped gateway exposure, catalog listing, and dispatcher routing had drifted away, so the CLI contract failed at the unsupported-tool boundary instead of reaching the real archive builder and validation logic.
- This round stayed scoped to the confirmed public-surface drift and did not redesign the archive schema, which let full regression advance cleanly to the next unrelated blocker.
### Remaining
- [ ] The current repository-wide first blocker is now `D:\SM\tests\security_committee_decision_package_cli.rs`, where the public stock bus still returns `unsupported tool: security_committee_decision_package`.
- [ ] The root log file `D:\SM\CHANGELOG_TASK.MD` still has encoding problems, so this task journal update was recorded only in `.trae/CHANGELOG_TASK.md`.
- [ ] The workspace remains intentionally dirty with many unrelated runtime artifacts and parallel edits, so this entry does not imply repository-wide review cleanliness.
### Risks
- [ ] The restored `security_closed_position_archive` route is intentionally lifecycle-archive-only; downstream callers could still over-read it as if it already performed committee packaging, execution, or persistence unless the boundary remains explicit.
- [ ] The newly exposed `security_committee_decision_package` failure may be another public-route drift rather than a behavior bug, but that still requires a fresh design-and-TDD loop before any additional code change.
### Closed
- Verified RED with `$env:CARGO_TARGET_DIR='D:\SM\target_security_closed_archive_red'; cargo test --test security_closed_position_archive_cli -- --nocapture`, which failed because the public stock bus returned `unsupported tool: security_closed_position_archive`.
- Verified GREEN with `$env:CARGO_TARGET_DIR='D:\SM\target_security_closed_archive_green'; cargo test --test security_closed_position_archive_cli -- --nocapture`, which passed with `10 passed; 0 failed`.
- Verified broader regression progress with `$env:CARGO_TARGET_DIR='D:\SM\target_security_closed_archive_full_verify'; cargo test -- --nocapture`: the closed-position archive blocker no longer blocks, and the next first failure is now `security_committee_decision_package_cli` with `unsupported tool: security_committee_decision_package`.
## 2026-04-21
### Modified
- Added the required `Security Decision Committee Legacy Freeze` handoff section to `D:\SM\docs\handoff\AI_HANDOFF.md`.
- Removed the unnecessary ETF-specific veto relaxation drift from the frozen legacy file `D:\SM\src\ops\security_decision_committee.rs` so the legacy committee compatibility zone returns to its approved frozen fingerprint.
- Refreshed `D:\SM\docs\handoff\CURRENT_STATUS.md` and `D:\SM\docs\handoff\HANDOFF_ISSUES.md` so branch-health records now move from the solved legacy-freeze blocker to the newly exposed `security_feature_snapshot_cli` blocker.
### Why
- The current blocker was not a measurement or modeling issue; the source guard failed because the handoff file was missing the required freeze section and the frozen legacy committee file had drifted away from its approved snapshot.
- Focused verification proved the gold-ETF chair path still stayed green after removing the legacy drift, which means the legacy edit was unnecessary and should not remain in the frozen compatibility zone.
- Once the freeze boundary was restored, branch-health truth needed to advance to the next real first failure instead of continuing to report a solved blocker.
### Remaining
- [ ] Investigate `D:\SM\tests\security_feature_snapshot_cli.rs`, where four tests now fail around historical-information fallback, governed disclosure/corporate-action preference counts, layered market/sector anchor fields, and equity-ETF manual-proxy preservation.
- [ ] The root log file `D:\SM\CHANGELOG_TASK.MD` still has encoding problems, so this task journal update was recorded only in `.trae/CHANGELOG_TASK.md`.
- [ ] The workspace remains intentionally dirty with many unrelated runtime artifacts and parallel edits, so this entry does not imply repository-wide review cleanliness.
### Risks
- [ ] Repository-wide green is still not achieved; the next first blocker is now the feature-snapshot suite, not the legacy committee freeze guard.
- [ ] The newly exposed feature-snapshot failures may come from fixture drift, contract drift, or runtime projection loss, so root-cause investigation is still required before any further fix.
### Closed
- Verified `$env:CARGO_TARGET_DIR='C:\codex-targets\sm_committee_package_green'; cargo test --test security_committee_decision_package_cli -- --nocapture`, which passed with `6 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='C:\codex-targets\sm_legacy_freeze_etf_red'; cargo test --test security_chair_resolution_cli security_chair_resolution_does_not_require_stock_only_information_for_gold_etf_when_proxy_history_is_complete -- --nocapture`, which stayed green after the legacy ETF drift was removed.
- Verified `$env:CARGO_TARGET_DIR='C:\codex-targets\sm_legacy_freeze_guard_green2'; cargo test --test security_decision_committee_legacy_freeze_source_guard -- --nocapture`, which passed with `1 passed; 0 failed`.
- Verified broader regression progress with `$env:CARGO_TARGET_DIR='C:\codex-targets\sm_legacy_freeze_full_verify'; cargo test -- --nocapture`: the legacy-freeze blocker no longer blocks, and the next first failure is now `security_feature_snapshot_cli` with four failing cases.
## 2026-04-23
### Modified
- Reconciled `D:\SM_latest_8214bc7d\tests\stock_formal_boundary_manifest_source_guard.rs` with the current merged mainline by freezing the restored public stock modules:
  - `import_stock_price_history_legacy_db`
  - `security_adjustment_input_package`
  - `security_closed_position_archive`
  - `security_committee_decision_package`
  - `security_investment_manager_entry`
- Restored the formal-boundary design references into `D:\SM_latest_8214bc7d\docs\plans\design\`:
  - `2026-04-16-stock-formal-boundary-manifest-gate-design.md`
  - `2026-04-15-stock-foundation-split-manifest-design.md`
  - `2026-04-15-stock-foundation-boundary-gate-v2-design.md`
- Added the `Stock Formal Boundary Manifest Gate` handoff section in `D:\SM_latest_8214bc7d\docs\handoff\AI_HANDOFF.md`.
- Refreshed `D:\SM_latest_8214bc7d\docs\handoff\CURRENT_STATUS.md` with the fresh 2026-04-23 focused verification evidence and corrected the formal-boundary design-doc source path in `D:\SM_latest_8214bc7d\docs\governance\decision_log.md`.
- Rebuilt `D:\SM_latest_8214bc7d\tests\security_investment_manager_entry_cli.rs` to replace one encoding-corrupted headline assertion with an ASCII-safe packet-preservation check.

### Why
- The approved A1 route keeps the newer merged D-drive mainline and restores missing lifecycle-tail capability instead of rolling back to the older worktree.
- After the tail modules were restored, the remaining blocker was boundary-guard truth drift: the merged branch no longer had the required design-doc files, and the frozen manifest still reflected the pre-reconciliation module set.
- Fresh verification also exposed that `security_investment_manager_entry_cli.rs` was not failing on business behavior; it contained a broken encoded string literal that prevented the test from compiling at all.

### Remaining
- [ ] Repository-wide `cargo test -- --nocapture` is still not re-run or green in this worktree.
- [ ] The latest known repository-level first blocker remains `tests/security_feature_snapshot_cli.rs` until that suite is debugged and fixed separately.
- [ ] The worktree remains intentionally dirty with many unrelated local edits and runtime artifacts, so this task only closes the approved A1 tail-lifecycle and boundary-guard slice.

### Risks
- [ ] The restored boundary docs under `docs/plans/design/` were copied into the merged worktree to satisfy the current guard and handoff truth; future branch surgery could drift them again if the guard is updated without keeping docs in sync.
- [ ] `security_investment_manager_entry_cli` is now green, but its repaired assertion deliberately checks compact-packet preservation instead of one locale-sensitive substring, so future wording changes in the upstream headline can still require intentional test review.
- [ ] `cargo check` remains warning-only due to unused helpers in `security_symbol_taxonomy.rs`; this task did not change or reduce those warnings.

### Closed
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_restore_tail_green3'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture`, which passed with `4 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_guard_tail'; cargo test --test security_committee_decision_package_cli --test security_adjustment_input_package_cli --test security_closed_position_archive_cli -- --nocapture`, which passed with `22 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_apply_bridge'; cargo test --test security_portfolio_execution_apply_bridge_cli -- --nocapture`, which passed with `8 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_manager_entry_fixed'; cargo test --test security_investment_manager_entry_cli -- --nocapture`, which passed with `2 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_check'; cargo check`, which passed with exit code `0` and the pre-existing three unused-code warnings in `security_symbol_taxonomy.rs`.
## 2026-04-24
### Modified
- Added the missing handoff sections to `D:\SM_latest_8214bc7d\docs\handoff\AI_HANDOFF.md` required by the active boundary source guard:
  - `Stock/Foundation Split Manifest Frozen`
  - `Stock/Foundation Boundary Gate V2`
- Re-ran the current repository-wide regression after the minimal handoff fix to advance the first failing boundary from `stock_foundation_boundary_gate_v2_source_guard` to the next unresolved guard.

### Why
- Fresh repository-wide verification on 2026-04-24 showed that the previous first blocker was no longer a business or runtime failure; it was a documentation guard failure because `AI_HANDOFF.md` no longer carried the split-manifest and gate-v2 handoff sections required by the checked-in source guard.
- The user explicitly approved `方案A`, so this round stayed intentionally minimal and only repaired the missing handoff-memory surface required by the active guard before rerunning full regression.

### Remaining
- [ ] The new repository-wide first blocker is now `D:\SM_latest_8214bc7d\tests\stock_foundation_boundary_source_guard.rs`.
- [ ] That guard currently fails because `docs/plans/design/2026-04-15-stock-foundation-decoupling-design.md` is missing from the merged worktree.
- [ ] No additional doc or code changes were made beyond the approved `方案A` surface; the next fix needs a new approved方案。

### Risks
- [ ] Repository-wide green is still not achieved; the first failure has only advanced to the next boundary-document guard.
- [ ] Because this round intentionally avoided broader status-doc synchronization, `CURRENT_STATUS.md` still does not describe the 2026-04-24 full-regression checkpoint.
- [ ] The worktree remains dirty with unrelated runtime and local changes, so this task journal entry only covers the minimal guard-memory repair.

### Closed
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_gate_v2_fix'; cargo test --test stock_foundation_boundary_gate_v2_source_guard -- --nocapture`, which passed with `3 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_full_regression_20260424_b'; cargo test -- --nocapture`, which still failed repository-wide but advanced the first blocking failure to `stock_foundation_boundary_source_guard` on missing `docs/plans/design/2026-04-15-stock-foundation-decoupling-design.md`.
## 2026-04-24
### Modified
- Backfilled the missing stock-boundary design docs under `D:\SM_latest_8214bc7d\docs\plans\design\` required by the active source guards:
  - `2026-04-15-stock-foundation-decoupling-design.md`
  - `2026-04-15-stock-application-entry-layer-design.md`
  - `2026-04-15-stock-business-flow-baseline.md`
  - `2026-04-16-security-legacy-committee-governance-design.md`
  - `2026-04-16-stock-modeling-lifecycle-split-design.md`
- Added the missing `Stock/Foundation Decoupling Baseline` section to `D:\SM_latest_8214bc7d\docs\handoff\AI_HANDOFF.md`.
- Refreshed `D:\SM_latest_8214bc7d\docs\handoff\CURRENT_STATUS.md` and `D:\SM_latest_8214bc7d\docs\handoff\HANDOFF_ISSUES.md` so branch-health truth now records the fresh 2026-04-24 repository-wide green verification instead of the older preserved blocker history.

### Why
- The approved `方案B` for this round was to fix the whole same-class boundary/source-guard document set, not keep advancing one missing file at a time.
- Root cause was a partial migration/backfill drift from `docs/plans/` to `docs/plans/design/`, not a runtime or business-logic failure.
- Because the user explicitly required a GitHub handoff after the fix, the status and handoff docs also needed to reflect the fresh repository truth before staging and push.

### Remaining
- [ ] The worktree is still intentionally dirty with many unrelated runtime artifacts, generated fixtures, and parallel edits, so Git staging must stay limited to this task slice only.
- [ ] The root log file `D:\SM\CHANGELOG_TASK.MD` still has encoding problems, so this task journal update was recorded only in `.trae/CHANGELOG_TASK.md`.
- [ ] The `docs/plans/` to `docs/plans/design/` migration remains a future drift risk if later sessions add guard-linked docs without backfilling the new path in the same change.

### Risks
- [ ] This round proves the current worktree is green under fresh `cargo test`, but it does not sanitize or review the many unrelated local modifications already present in the workspace.
- [ ] If future boundary-guard work updates only tests or only handoff memory without updating `docs/plans/design/`, the same class of blocker can reappear.

### Closed
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_boundary_decoupling'; cargo test --test stock_foundation_boundary_source_guard -- --nocapture`, which passed with `3 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_entry_layer'; cargo test --test stock_entry_layer_source_guard -- --nocapture`, which passed with `5 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_catalog_grouping'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture`, which passed with `2 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_dispatcher_grouping'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture`, which passed with `1 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_modeling_split'; cargo test --test stock_modeling_training_split_source_guard -- --nocapture`, which passed with `2 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_package_chair'; cargo test --test security_decision_package_chair_node_source_guard -- --nocapture`, which passed with `1 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_package_verify'; cargo test --test security_decision_verify_package_source_guard -- --nocapture`, which passed with `1 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_verify_legacy_freeze'; cargo test --test security_decision_committee_legacy_freeze_source_guard -- --nocapture`, which passed with `1 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM_latest_8214bc7d\target_full_regression_20260424_c'; cargo test -- --nocapture`, which passed repository-wide in this worktree.
## 2026-04-25
### Modified
- Restored the missing P16 execution-status bridge module at `D:\SM\src\ops\security_portfolio_execution_status_bridge.rs`.
- Added CLI coverage at `D:\SM\tests\security_portfolio_execution_status_bridge_cli.rs` for catalog visibility, fully-applied status freeze, and rejected apply-document preservation.
- Restored the missing capital-flow/capital-source module files and runtime store required by the current stock boundary:
  - `D:\SM\src\runtime\security_capital_flow_store.rs`
  - `D:\SM\src\ops\security_capital_flow_backfill.rs`
  - `D:\SM\src\ops\security_capital_flow_raw_audit.rs`
  - `D:\SM\src\ops\security_capital_flow_jpx_weekly_import.rs`
  - `D:\SM\src\ops\security_capital_flow_jpx_weekly_live_backfill.rs`
  - `D:\SM\src\ops\security_capital_flow_mof_weekly_import.rs`
  - `D:\SM\src\ops\security_capital_source_factor_snapshot.rs`
  - `D:\SM\src\ops\security_capital_source_factor_audit.rs`
- Cleaned generated verification target directories and deleted the verified backup `D:\SM_backup_20260425_055754`.

### Why
- After consolidating the latest branch into `D:\SM`, fresh verification failed because the public stock boundary referenced modules that were not present in the working tree.
- P16 needed to remain a pure P15 apply-document status-freeze layer, not reconciliation, runtime replay, broker execution, or position materialization.
- The Nikkei capital-source training path also depended on governed capital-flow persistence and observation-only factor snapshots, so compile-only placeholders were not sufficient for repository-wide green.

### Remaining
- [ ] The restored JPX/MOF import and live-backfill routes preserve public contracts but do not implement real workbook, CSV, or network adapters in this recovery slice.
- [ ] The capital-source snapshot computes the observation-only metrics required by current training tests from governed raw rows; broader financial interpretation still needs a separate approved design if expanded.

### Risks
- [ ] The capital-flow restoration was reconstructed from active tests and contract references because the original module files were absent from `HEAD`, branch history, and the retained backup.
- [ ] Future capital-source work should avoid treating this recovery as approval to merge capital-source factors into model features; current behavior remains observation-only.

### Closed
- Verified RED first: `$env:CARGO_TARGET_DIR='D:\SM\target_p16_status_bridge_red'; cargo test --test security_portfolio_execution_status_bridge_cli -- --nocapture`, which failed on missing `src\ops\security_portfolio_execution_status_bridge.rs`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p16_status_bridge_green'; cargo test --test security_portfolio_execution_status_bridge_cli -- --nocapture`, which passed with `3 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p16_status_bridge_check'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture`, which passed with `4 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_scorecard_training_capital_fix'; cargo test --test security_scorecard_training_cli -- --nocapture`, which passed with `17 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_verify_after_p16_final'; cargo test -- --nocapture`, which passed repository-wide.
## 2026-04-25
### Modified
- Added `D:\SM\docs\plans\2026-04-25-p17-p18-execution-recovery-design.md` and `D:\SM\docs\plans\2026-04-25-p17-p18-execution-recovery-plan.md` to freeze the approved方案A recovery contract before implementation.
- Rebuilt P17 at `D:\SM\src\ops\security_portfolio_execution_reconciliation_bridge.rs` with CLI coverage in `D:\SM\tests\security_portfolio_execution_reconciliation_bridge_cli.rs`.
- Rebuilt P18 at `D:\SM\src\ops\security_portfolio_execution_repair_package.rs` with CLI coverage in `D:\SM\tests\security_portfolio_execution_repair_package_cli.rs`.
- Wired P17/P18 through `D:\SM\src\ops\stock.rs`, `D:\SM\src\ops\stock_execution_and_position_management.rs`, `D:\SM\src\tools\catalog.rs`, `D:\SM\src\tools\dispatcher.rs`, and `D:\SM\src\tools\dispatcher\stock_ops.rs`.
- Updated `D:\SM\tests\stock_formal_boundary_manifest_source_guard.rs`, `D:\SM\docs\governance\contract_registry.md`, `D:\SM\docs\governance\decision_log.md`, `D:\SM\docs\handoff\CURRENT_STATUS.md`, and `D:\SM\docs\handoff\HANDOFF_ISSUES.md` for the recovered P17/P18 truth.

### Why
- The user approved方案A after inspection showed `E:\SM` and the historical temporary worktrees were unavailable and Git history did not contain the P17/P18 source files.
- P17 had to be rebuilt before P18 because repair intent must consume a formal reconciliation artifact rather than raw P16 status rows.
- P18 was kept as repair-intent freeze only, not replay execution, broker execution, position materialization, or lifecycle closeout.

### Remaining
- [ ] P19 replay executor / repair executor is still not designed or implemented; it requires a separate approved contract.
- [ ] Repository-wide `cargo test -- --nocapture` was not rerun in this recovery closeout; current evidence is focused P17/P18 plus boundary/grouping guards and `cargo check`.

### Risks
- [ ] The original `E:\SM` P17/P18 files were unavailable, so this recovery was reconstructed from the current P16 implementation, handoff notes, and changelog evidence rather than copied byte-for-byte.
- [ ] The worktree remains dirty with unrelated local files and generated targets, so any Git delivery must stage only this task slice.

### Closed
- Verified RED first: `$env:CARGO_TARGET_DIR='D:\SM\target_p17_recovery_red'; cargo test --test security_portfolio_execution_reconciliation_bridge_cli -- --nocapture`, which failed with `unsupported tool: security_portfolio_execution_reconciliation_bridge`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p17_recovery_green'; cargo test --test security_portfolio_execution_reconciliation_bridge_cli -- --nocapture`, which passed with `4 passed; 0 failed`.
- Verified RED first: `$env:CARGO_TARGET_DIR='D:\SM\target_p18_recovery_red'; cargo test --test security_portfolio_execution_repair_package_cli -- --nocapture`, which failed with `unsupported tool: security_portfolio_execution_repair_package`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p18_recovery_green'; cargo test --test security_portfolio_execution_repair_package_cli -- --nocapture`, which passed with `6 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_guard'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture`, which passed with `4 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_guard'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture`, which passed with `2 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_guard'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture`, which passed with `1 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_final'; cargo test --test security_portfolio_execution_reconciliation_bridge_cli --test security_portfolio_execution_repair_package_cli -- --nocapture`, which passed with `10 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_final'; cargo check`, which completed successfully.
- Ran `cargo fmt`.
- Verified after formatting: `$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_verify_after_fmt'; cargo test --test security_portfolio_execution_reconciliation_bridge_cli --test security_portfolio_execution_repair_package_cli -- --nocapture`, which passed with `10 passed; 0 failed`.
- Verified after formatting: `$env:CARGO_TARGET_DIR='D:\SM\target_p17_p18_recovery_verify_after_fmt'; cargo check`, which completed successfully.
## 2026-04-25
### Modified
- Updated `D:\SM\docs\handoff\CURRENT_STATUS.md` and `D:\SM\docs\handoff\HANDOFF_ISSUES.md` with the fresh repository-wide verification result after the P17/P18 recovery.

### Why
- The user chose方案1 after P17/P18 focused verification: run full repository regression before deciding whether to design P19.
- Branch-health truth changed from focused-green plus `cargo check` to repository-wide `cargo test -- --nocapture` green in the current `D:\SM` worktree.

### Remaining
- [ ] P19 replay executor / repair executor is still not designed or implemented; it still requires a separate approved contract-first design.
- [ ] The worktree remains dirty with multiple tracked and untracked recovery files, so repository green does not imply clean Git state or ready-to-stage scope.

### Risks
- [ ] The full regression used an isolated target directory, but it ran in the dirty local `D:\SM` worktree; future Git delivery must still stage only the intended task slice.

### Closed
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p18'; cargo test -- --nocapture`, which completed with exit code 0 after running unit tests, integration tests, source guards, P17/P18 tests, and doc tests.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p18_confirm'; cargo test -- --nocapture`, which completed with exit code 0 in the current session before moving to any P19 design decision.
## 2026-04-25
### Modified
- Added `D:\SM\docs\plans\2026-04-25-p19a-execution-replay-request-package-design.md` and `D:\SM\docs\plans\2026-04-25-p19a-execution-replay-request-package-plan.md` for the approved A1 strict replay-request package.
- Added P19A at `D:\SM\src\ops\security_portfolio_execution_replay_request_package.rs` with CLI coverage in `D:\SM\tests\security_portfolio_execution_replay_request_package_cli.rs`.
- Wired P19A through `D:\SM\src\ops\stock.rs`, `D:\SM\src\ops\stock_execution_and_position_management.rs`, `D:\SM\src\tools\catalog.rs`, `D:\SM\src\tools\dispatcher.rs`, and `D:\SM\src\tools\dispatcher\stock_ops.rs`.
- Updated `D:\SM\tests\stock_formal_boundary_manifest_source_guard.rs`, `D:\SM\docs\governance\contract_registry.md`, `D:\SM\docs\governance\decision_log.md`, `D:\SM\docs\handoff\CURRENT_STATUS.md`, and `D:\SM\docs\handoff\HANDOFF_ISSUES.md` for P19A.

### Why
- The user selected P19A/A1 after P17/P18 recovery and repository-wide regression verification.
- P19A needed to freeze only P18 `governed_retry_candidate` rows as replay requests before any future replay executor contract.
- Manual follow-up and blocked governance rows must remain excluded from replay request rows.

### Remaining
- [ ] P19B replay executor / retry executor is still not designed or implemented; it requires a separate approved contract.
- [ ] Repository-wide `cargo test -- --nocapture` was not rerun after P19A; current evidence is P19A focused tests, formal boundary/grouping guards, and `cargo check`.

### Risks
- [ ] The worktree remains dirty with unrelated local files and parallel Nikkei/capital-source changes, so any Git delivery must stage only the intended task slice.
- [ ] Formal boundary guard verification had to align its expected manifest with the active `security_volume_source_manifest` module already present in the dirty worktree.

### Closed
- Verified RED first: `$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_red'; cargo test --test security_portfolio_execution_replay_request_package_cli -- --nocapture`, which failed with `unsupported tool: security_portfolio_execution_replay_request_package`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_green'; cargo test --test security_portfolio_execution_replay_request_package_cli -- --nocapture`, which passed with `5 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_final'; cargo test --test security_portfolio_execution_replay_request_package_cli -- --nocapture`, which passed with `5 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture`, which passed with `4 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture`, which passed with `2 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture`, which passed with `1 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p19a_replay_request_final'; cargo check`, which completed successfully.
## 2026-04-25
### Modified
- Added the approved Nikkei volume-proxy training contract in `D:\SM\docs\plans\2026-04-25-nikkei-volume-proxy-design.md` and `D:\SM\docs\plans\2026-04-25-nikkei-volume-proxy-plan.md`.
- Updated `D:\SM\src\ops\security_scorecard_training.rs` with `volume_proxy_symbol` support for weekly training.
- Updated `D:\SM\tests\security_scorecard_training_cli.rs` with a regression test proving the proxy gives weekly volume features non-constant values without enabling futures price features.
- Updated `D:\SM\task_plan.md`, `D:\SM\progress.md`, and `D:\SM\findings.md` with the data state, real rerun comparison, and verification evidence.

### Why
- The restored governed Nikkei price source is FRED close-only data, so all `NK225.IDX` rows have `volume=0`.
- Weekly volume features were therefore structurally unavailable and collapsed to zero variance.
- The user approved Scheme B: preserve FRED as the official price source and add a separate volume-only proxy instead of mixing Yahoo OHLCV into the spot history.

### Remaining
- [ ] The current proxy source only covers `2024-10-01..2026-04-01`; a full-range Nikkei volume proxy is still needed before treating the result as a complete 10-year validation.
- [ ] Accuracy improvement is limited and mixed, so further model tuning should wait until the longer volume source is available.

### Risks
- [ ] The short proxy fixes feature availability but may overweight recent volume behavior because early training windows still fall back to spot/futures/zero-volume availability.
- [ ] `D:\SM` remains a dirty worktree with unrelated recovery files and generated target directories; any Git delivery must stage only the intended task slice.

### Closed
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_volume_proxy_green'; cargo test security_scorecard_training_nikkei_weekly_uses_volume_proxy_without_futures_features --test security_scorecard_training_cli -- --nocapture`, which passed with `1 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_volume_proxy_weekly'; cargo test weekly_ --test security_scorecard_training_cli -- --nocapture`, which passed with `5 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_volume_proxy_full'; cargo test --test security_scorecard_training_cli -- --nocapture`, which passed with `18 passed; 0 failed`.
## 2026-04-25
### Modified
- Added `D:\SM\docs\plans\2026-04-25-nikkei-volume-source-manifest-design.md` and `D:\SM\docs\plans\2026-04-25-nikkei-volume-source-manifest-plan.md`.
- Added `security_volume_source_manifest` as a public stock data-pipeline tool.
- Added source/date/volume coverage aggregation in `D:\SM\src\runtime\stock_history_store.rs`.
- Added manifest operation logic in `D:\SM\src\ops\security_volume_source_manifest.rs`.
- Wired the tool through `D:\SM\src\ops\stock.rs`, `D:\SM\src\ops\stock_data_pipeline.rs`, `D:\SM\src\tools\catalog.rs`, `D:\SM\src\tools\dispatcher.rs`, and `D:\SM\src\tools\dispatcher\stock_ops.rs`.
- Added CLI coverage in `D:\SM\tests\security_volume_source_manifest_cli.rs`.
- Generated real manifest data at `D:\.stockmind_runtime\nikkei_10y_market_20260425\nikkei_volume_source_manifest_20260425.json`.
- Updated `D:\SM\task_plan.md`, `D:\SM\progress.md`, and `D:\SM\findings.md` with the manifest verdict.

### Why
- The user asked to补量能系统清单数据 before further model tuning.
- The previous volume-proxy implementation proved the feature path works, but the source inventory was still implicit.
- A top-level investment system needs a reproducible manifest that distinguishes `no_volume`, `usable_short_proxy`, and `train_ready_volume_proxy`.

### Remaining
- [ ] Current Nikkei volume has no 750-day train-ready source; `NK225_VOL.PROXY` remains a short proxy with `365` rows.
- [ ] The manifest is read-only in this phase; training does not yet enforce manifest gates automatically.

### Risks
- [ ] `NK225_VOL.PROXY` has one zero-volume row and short coverage, so it should not be treated as complete 10-year volume evidence.
- [ ] The worktree still contains unrelated recovery changes and generated target directories; Git delivery must stage only the intended task slice.

### Closed
- Verified RED first: `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_volume_manifest_red'; cargo test --test security_volume_source_manifest_cli -- --nocapture`, which failed with unsupported tool/catalog miss.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_volume_manifest_green'; cargo test --test security_volume_source_manifest_cli -- --nocapture`, which passed with `2 passed; 0 failed`.
- Ran `cargo fmt`.
- Verified after formatting: `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_volume_manifest_green'; cargo test --test security_volume_source_manifest_cli -- --nocapture`, which passed with `2 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_volume_manifest_check'; cargo check`, which completed successfully.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_volume_manifest_guards'; cargo test --test stock_catalog_grouping_source_guard --test stock_dispatcher_grouping_source_guard --test stock_formal_boundary_manifest_source_guard -- --nocapture`, which passed with `7 passed; 0 failed` across the three guard files.
## 2026-04-25
### Modified
- Added `D:\SM\docs\plans\2026-04-25-nikkei-turnover-official-import-design.md` and `D:\SM\docs\plans\2026-04-25-nikkei-turnover-official-import-plan.md`.
- Added `security_nikkei_turnover_import` as a public stock data-pipeline tool.
- Added official turnover import logic in `D:\SM\src\ops\security_nikkei_turnover_import.rs`.
- Wired the tool through `D:\SM\src\ops\stock.rs`, `D:\SM\src\ops\stock_data_pipeline.rs`, `D:\SM\src\tools\catalog.rs`, `D:\SM\src\tools\dispatcher.rs`, and `D:\SM\src\tools\dispatcher\stock_ops.rs`.
- Added CLI coverage in `D:\SM\tests\security_nikkei_turnover_import_cli.rs`.
- Updated `D:\SM\tests\stock_formal_boundary_manifest_source_guard.rs` so the frozen formal stock manifest includes the approved turnover importer.
- Updated `D:\SM\task_plan.md`, `D:\SM\progress.md`, and `D:\SM\findings.md` with the turnover receiver status.

### Why
- The user rejected paid Stooq and confirmed a free-source route.
- Nikkei official `Total Trading Value` is the preferred free proxy, but automated local access is Cloudflare-blocked.
- The system therefore needs a governed receiver for manually exported official Nikkei turnover files.

### Remaining
- [ ] Actual long-history `NK225_TURNOVER.NIKKEI` import still needs the official Nikkei turnover export/copy-text file.
- [ ] After the file is supplied, re-run `security_nikkei_turnover_import`, regenerate `security_volume_source_manifest`, then rerun weekly training with `volume_proxy_symbol=NK225_TURNOVER.NIKKEI`.

### Risks
- [ ] Turnover is not share-volume truth; it must remain labeled as `Total Trading Value` proxy.
- [ ] Missing price dates are skipped by design, so a real import may have fewer rows than the official turnover export if `NK225.IDX` price coverage has gaps.

### Closed
- Verified RED first: `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_turnover_import_red'; cargo test --test security_nikkei_turnover_import_cli -- --nocapture`, which failed with unsupported tool/catalog miss.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_turnover_import_green'; cargo test --test security_nikkei_turnover_import_cli -- --nocapture`, which passed with `2 passed; 0 failed`.
- Ran `cargo fmt`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_turnover_import_check'; cargo check`, which completed successfully.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_turnover_import_guards'; cargo test --test stock_catalog_grouping_source_guard --test stock_dispatcher_grouping_source_guard --test stock_formal_boundary_manifest_source_guard -- --nocapture`, which passed with `7 passed; 0 failed`.
## 2026-04-25
### Modified
- Updated `D:\SM\docs\handoff\CURRENT_STATUS.md` with the repository-wide verification result after P19A.

### Why
- The user approved running full repository regression after P19A before entering P19B design.
- Branch-health truth changed from P19A focused-green plus `cargo check` to repository-wide `cargo test -- --nocapture` green in the current `D:\SM` worktree.

### Remaining
- [ ] P19B replay executor / retry executor is still not designed or implemented; it now proceeds to a separate contract-first design step.
- [ ] The worktree remains dirty with multiple tracked and untracked recovery, Nikkei, capital-source, and P19A files.

### Risks
- [ ] The full regression used an isolated target directory, but it ran in the dirty local `D:\SM` worktree; future Git delivery must still stage only the intended task slice.

### Closed
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19a'; cargo test -- --nocapture`, which completed with exit code 0 after running unit tests, integration tests, P19A tests, source guards, and doc tests.
## 2026-04-25
### Modified
- Added `D:\SM\docs\plans\2026-04-25-p19b-execution-replay-dry-run-executor-design.md`.
- Added `D:\SM\docs\plans\2026-04-25-p19b-execution-replay-dry-run-executor-plan.md`.

### Why
- The user confirmed entering P19B after the P19A repository-wide regression passed.
- P19B needs a separate dry-run-first executor contract before any runtime write or commit-mode replay work.

### Remaining
- [ ] P19B implementation has not started; the next step is TDD red tests for `security_portfolio_execution_replay_executor`.
- [ ] Commit-mode replay remains explicitly out of scope until a later approved design.

### Risks
- [ ] The worktree remains dirty with unrelated recovery, Nikkei, capital-source, and P19A files, so future Git delivery must stage only the intended task slice.

### Closed
- Recorded the P19B B1 dry-run-first executor design and implementation plan.
## 2026-04-25
### Modified
- Added P19B at `D:\SM\src\ops\security_portfolio_execution_replay_executor.rs` with CLI coverage in `D:\SM\tests\security_portfolio_execution_replay_executor_cli.rs`.
- Wired P19B through `D:\SM\src\ops\stock.rs`, `D:\SM\src\ops\stock_execution_and_position_management.rs`, `D:\SM\src\tools\catalog.rs`, `D:\SM\src\tools\dispatcher.rs`, and `D:\SM\src\tools\dispatcher\stock_ops.rs`.
- Updated `D:\SM\tests\stock_formal_boundary_manifest_source_guard.rs`, `D:\SM\docs\governance\contract_registry.md`, `D:\SM\docs\governance\decision_log.md`, `D:\SM\docs\handoff\CURRENT_STATUS.md`, and `D:\SM\docs\handoff\HANDOFF_ISSUES.md` for P19B.

### Why
- The user asked to continue after approving P19B.
- P19B needed to freeze dry-run executor eligibility and deterministic idempotency keys before any commit-mode runtime replay.
- Commit mode remains explicitly rejected in this phase.

### Remaining
- [ ] P19C/P20 commit-mode replay executor is not designed or implemented; it requires a separate approved contract.
- [ ] Repository-wide `cargo test -- --nocapture` was not rerun after P19B; current evidence will be focused P19B tests, guards, and `cargo check`.

### Risks
- [ ] The worktree remains dirty with unrelated recovery, Nikkei, capital-source, P19A, and P19B files, so any Git delivery must stage only the intended task slice.
- [ ] P19B dry-run validates executor readiness but does not reduce unresolved runtime execution state.

### Closed
- Verified RED first: `$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_red'; cargo test --test security_portfolio_execution_replay_executor_cli -- --nocapture`, which failed with `unsupported tool: security_portfolio_execution_replay_executor`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_green'; cargo test --test security_portfolio_execution_replay_executor_cli -- --nocapture`, which passed with `7 passed; 0 failed`.
## 2026-04-25
### Modified
- Updated `D:\SM\docs\handoff\CURRENT_STATUS.md` with the final focused P19B verification evidence.

### Why
- The P19B implementation entry had recorded RED/GREEN evidence but not the final source guards and `cargo check` evidence from the current session.
- Completion claims for P19B must stay tied to fresh command output rather than inferred from earlier focused runs.

### Remaining
- [ ] Repository-wide `cargo test -- --nocapture` was not rerun after P19B; current evidence is P19B focused tests, formal boundary/grouping guards, and `cargo check`.
- [ ] P19C/P20 commit-mode replay executor remains out of scope until a separate approved contract.

### Risks
- [ ] The worktree remains dirty with unrelated recovery, Nikkei, capital-source, P19A, and P19B files, so Git delivery must stage only the intended task slice.

### Closed
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_final'; cargo test --test security_portfolio_execution_replay_executor_cli -- --nocapture`, which passed with `7 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture`, which passed with `4 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture`, which passed with `2 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture`, which passed with `1 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p19b_replay_executor_final'; cargo check`, which completed successfully.
## 2026-04-25
### Modified
- Updated `D:\SM\docs\handoff\CURRENT_STATUS.md` with the failed P19B follow-up repository-wide regression.

### Why
- The user approved running full repository regression after P19B.
- The command did not pass and must be recorded as the current branch-health truth before any further completion claim.

### Remaining
- [ ] Resolve `security_nikkei_turnover_import` formal boundary manifest drift or remove that unapproved boundary exposure, then rerun the focused guard and full repository regression.
- [ ] P19C/P20 commit-mode replay executor remains out of scope until P19B repository health is restored and a separate approved contract exists.

### Risks
- [ ] The failure comes from a parallel Nikkei official turnover import slice in the dirty worktree, so fixing it must avoid reverting unrelated user/parallel edits.
- [ ] Until rerun, P19B remains focused-green but the current `D:\SM` worktree is not repository-wide green after P19B.

### Closed
- Ran `$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19b'; cargo test -- --nocapture`, which failed with exit code 1.
- Reproduced the blocker with `$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19b'; cargo test --test stock_formal_boundary_manifest_source_guard stock_root_keeps_only_the_frozen_module_manifest -- --nocapture`, which failed because `security_nikkei_turnover_import` is present in `src\ops\stock.rs` but absent from the frozen public boundary manifest.
## 2026-04-25
### Modified
- Updated `C:\Users\wakes\.codex\skills\contract-first-design\SKILL.md` to require a Cross-Artifact Contract and Independent Risk Pass when public boundaries, registries, catalogs, dispatchers, manifests, snapshots, or source guards may drift.
- Updated `C:\Users\wakes\.codex\skills\writing-plans\SKILL.md` to require a Risk Synchronization Gate and a separate boundary-manifest/source-guard sync task for public boundary changes.
- Updated `C:\Users\wakes\.codex\skills\writing-skills\SKILL.md` so future skill edits that prevent repeated cross-artifact drift failures must include RED/GREEN pressure scenarios.

### Why
- The P19B follow-up repository-wide regression exposed an earlier process gap: a public stock boundary module can be wired through runtime entrypoints while its frozen manifest/source guard is missed.
- The user asked to solve this by requiring another risk-identification subprocess, not by adding another ordinary checklist.

### Remaining
- [ ] The current `D:\SM` repository-wide regression still needs to be rerun after the active boundary drift is resolved.
- [ ] The skill changes are local to `C:\Users\wakes\.codex\skills`; they are not repository-tracked unless a separate skill distribution step is requested.

### Risks
- [ ] Inline fresh-pass mode is not a true independent agent; it must be labeled honestly unless the user authorizes `spawn_agent`.
- [ ] The stronger gates may add design overhead for boundary-facing changes, but they are scoped to public surfaces, manifests, registries, catalogs, dispatchers, source guards, and derived artifacts.

### Closed
- RED pressure scenario confirmed the previous `contract-first-design` and `writing-plans` skills did not require an independent risk subprocess as a hard gate.
- GREEN pressure scenario confirmed the updated skills now trigger `Independent Risk Pass` / `Risk Synchronization Gate` and require mode, source-of-truth, frozen-artifact, guard-test, must-sync, and blocker fields.
- Verified `python C:\Users\wakes\.codex\skills\.system\skill-creator\scripts\quick_validate.py C:\Users\wakes\.codex\skills\contract-first-design`, which reported `Skill is valid!`.
- Verified `python C:\Users\wakes\.codex\skills\.system\skill-creator\scripts\quick_validate.py C:\Users\wakes\.codex\skills\writing-plans`, which reported `Skill is valid!`.
- Verified `$env:PYTHONUTF8='1'; python C:\Users\wakes\.codex\skills\.system\skill-creator\scripts\quick_validate.py C:\Users\wakes\.codex\skills\writing-skills`, which reported `Skill is valid!`.
## 2026-04-25
### Modified
- Imported existing yfinance-derived Nikkei volume history into the active Nikkei market DB as `NK225_VOL.YFINANCE`.
- Updated `D:\SM\task_plan.md`, `D:\SM\progress.md`, and `D:\SM\findings.md` with the corrected data-source route, manifest verdict, and three-run training comparison.

### Why
- The user clarified that the project already has yfinance code/data access, so the correct path is to reuse the local implementation before searching external sources.
- The earlier `NK225_VOL.PROXY` source was too short for the 750-day readiness gate, so the weekly training comparison needed the long yfinance proxy.

### Remaining
- [ ] `NK225_VOL.YFINANCE` is still a proxy, not official Nikkei turnover truth.
- [ ] Weekly readiness remains `caution`; do not promote this model without addressing weak walk-forward evidence and redundant correlated price-position features.

### Risks
- [ ] `NK225.IDX` must remain the FRED price source; yfinance OHLCV rows are only imported under the separate proxy symbol.
- [ ] The post-validation holdout did not improve versus the no-proxy run, even though valid/test accuracy improved.

### Closed
- Imported `D:\.stockmind_runtime\nikkei_10y_market_20260425\nikkei_yfinance_N225_20160425_20260425_training_format.csv` as `NK225_VOL.YFINANCE`, with `2443` rows from `2016-04-25` through `2026-04-24`.
- Verified manifest output reports `NK225_VOL.YFINANCE` as `train_ready_volume_proxy`.
- Reran current Nikkei weekly `direction_head` with `volume_proxy_symbol=NK225_VOL.YFINANCE`; result was `valid_acc=0.520548`, `test_acc=0.502283`, `holdout_acc=0.370370`, `walk_forward_mean=0.532787`, readiness `caution`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_yfinance_volume_verify_manifest'; cargo test --test security_volume_source_manifest_cli -- --nocapture`, which passed with `2 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_yfinance_volume_verify_scorecard'; cargo test security_scorecard_training_nikkei_weekly_uses_volume_proxy_without_futures_features --test security_scorecard_training_cli -- --nocapture`, which passed with `1 passed; 0 failed`.
## 2026-04-26
### 修改内容
- Added `D:\SM\docs\plans\2026-04-26-nikkei-long-horizon-volume-behavior-design.md`.
- Added `D:\SM\docs\plans\2026-04-26-nikkei-long-horizon-volume-behavior-plan.md`.
- Updated `D:\SM\src\ops\security_scorecard_training.rs` with long-horizon Nikkei weekly volume behavior features:
  - `weekly_volume_ratio_13w`
  - `weekly_volume_ratio_26w`
  - `weekly_volume_ratio_52w`
  - `weekly_price_position_52w`
  - `weekly_volume_accumulation_26w`
  - `weekly_volume_accumulation_52w`
  - `weekly_high_volume_low_price_signal`
  - `weekly_high_volume_breakout_signal`
- Updated `D:\SM\tests\security_scorecard_training_cli.rs` with RED/GREEN coverage for the new weekly feature contract.
- Updated `D:\SM\task_plan.md`, `D:\SM\progress.md`, and `D:\SM\findings.md` with formulas, verification, and real Nikkei rerun metrics.

### 修改原因
- The user confirmed Scheme B and clarified that index-level accumulation can be yearly, so the previous 4-week volume ratio was too short for large capital behavior.
- The model needed explicit half-year/yearly volume context before judging volume as bullish, bearish, accumulation, or breakout.

### 方案还差什么
- [ ] The model remains `caution`, not production-ready.
- [ ] A follow-up feature-governance step should reduce correlated volume features before any promotion.
- [ ] yfinance volume remains a proxy source, not official Nikkei turnover truth.

### 潜在问题
- [ ] Feature count increased from `14` to `22` while weekly sample count stayed `244`, reducing sample-per-feature.
- [ ] High-correlation pair count increased from `1` to `2`; `weekly_volume_accumulation_26w` and `weekly_volume_accumulation_52w` correlate at `0.8621`.
- [ ] Walk-forward mean fell to `0.491803`, even though test and holdout improved.

### 关闭项
- RED verified: `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_red'; cargo test weekly_price_aggregation_emits_distribution_quantiles_for_nikkei_training --test security_scorecard_training_cli -- --nocapture` failed because `weekly_volume_ratio_13w` was missing.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_green'; cargo test weekly_price_aggregation_emits_distribution_quantiles_for_nikkei_training --test security_scorecard_training_cli -- --nocapture`, which passed with `1 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_proxy_green'; cargo test security_scorecard_training_nikkei_weekly_uses_volume_proxy_without_futures_features --test security_scorecard_training_cli -- --nocapture`, which passed with `1 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_weekly'; cargo test weekly_ --test security_scorecard_training_cli -- --nocapture`, which passed with `5 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_check'; cargo check`, which completed successfully.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_nikkei_long_volume_scorecard_full'; cargo test --test security_scorecard_training_cli -- --nocapture`, which passed with `18 passed; 0 failed`.
- Real Nikkei rerun completed at `D:\.stockmind_runtime\nikkei_current_rerun_20260426_direction_head_yfinance_10y_long_volume_behavior` with `valid_acc=0.484018`, `test_acc=0.525114`, `holdout_acc=0.407407`, `walk_forward_mean=0.491803`, readiness `caution`.
## 2026-04-26
### 修改内容
- Added the low-price extreme-volume event finding to `D:\SM\findings.md`.
- Updated `D:\SM\progress.md` with the anomaly export path and interpretation.

### 修改原因
- The user identified that low-price extreme volume may be caused by major events rather than ordinary accumulation.
- The anomaly table supports that interpretation: the strongest low-price extreme-volume Nikkei cases cluster around tariff shock, yen carry unwind, war shock, Fed/yen pressure, and banking-stress weeks.

### 方案还差什么
- [ ] Mild-volume behavior still needs separate analysis after this event finding.
- [ ] Event tags are initial annotations; a later feature contract would need structured event-source governance before entering training.

### 潜在问题
- [ ] Without event tagging, the model may confuse panic liquidation with accumulation.
- [ ] Low-price extreme-volume sample count is small, so conclusions should remain diagnostic rather than production rules.

### 关闭项
- Recorded the finding: low-price extreme volume is usually panic/capitulation context when it coincides with major macro or policy events; no-event low-price volume is the better accumulation candidate.
## 2026-04-26
### Modified
- Updated `D:\SM\docs\handoff\CURRENT_STATUS.md` with the P19B follow-up boundary recheck and repository-wide regression pass.

### Why
- The previous P19B follow-up full regression had failed on `security_nikkei_turnover_import` formal boundary manifest drift.
- The user approved continuing with focused guard verification, turnover import verification, and a full repository rerun before moving past P19B.

### Remaining
- [ ] P19C/P20 commit-mode replay executor remains undesigned and unimplemented; it requires a separate contract-first design before any code change.
- [ ] The worktree remains dirty with unrelated recovery, Nikkei, capital-source, generated target, and runtime fixture artifacts; Git delivery still needs narrow staging.

### Risks
- [ ] Repository-wide green was verified in the current dirty `D:\SM` worktree, so it proves current local branch health but not clean Git scope.
- [ ] `security_nikkei_turnover_import` is now treated as a formal public stock-boundary module; future changes must keep its manifest/catalog/dispatcher/source guards synchronized.

### Closed
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p19b_boundary_recheck'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture`, which passed with `4 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_p19b_boundary_recheck'; cargo test --test security_nikkei_turnover_import_cli -- --nocapture`, which passed with `2 passed; 0 failed`.
- Verified `$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19b_recheck'; cargo test -- --nocapture`, which completed with exit code 0.
## 2026-04-26
### Modified
- Added `D:\SM\docs\plans\2026-04-26-p19c-execution-replay-commit-preflight-design.md`.
- Added `D:\SM\docs\plans\2026-04-26-p19c-execution-replay-commit-preflight-plan.md`.

### Why
- The user approved P19C方案A and explicitly asked to use the newly strengthened risk-subprocess skill flow.
- P19C should freeze commit preflight inputs, canonical payload hashes, and future durable idempotency candidates before any P19D runtime writer exists.
- P19C must not open P19B commit mode or call `security_execution_record`.

### Remaining
- [ ] P19C production implementation has not started; the next step is TDD RED tests for `security_portfolio_execution_replay_commit_preflight`.
- [ ] P19D controlled runtime write remains undesigned and unimplemented.
- [ ] P19C still needs focused verification, source guards, `cargo check`, and repository-wide regression after implementation.

### Risks
- [ ] P19C naming and docs must keep `preflight` explicit so future work does not mistake it for runtime commit authority.
- [ ] The worktree remains dirty with unrelated recovery, Nikkei, capital-source, generated target, and runtime fixture artifacts; Git delivery still needs narrow staging.

### Closed
- Ran an independent risk subprocess with mode `user-approved-subagent`; it confirmed P19C must remain commit-preflight-only and defer runtime writes to P19D.
- Static-checked the P19C design for `Cross-Artifact Contract`, `Independent Risk Pass`, `Hard Rejection Red Lines`, `security_execution_record` non-goal, and P19D deferral.
- Static-checked the P19C plan for `Risk Synchronization Gate`, `user-approved-subagent` mode, must-sync files, must-run checks, runtime ref rejection, and focused Cargo commands.
- Verified current P19C changes are documentation-only: only the two new P19C docs appear under the `p19c` git-status filter.
## 2026-04-26
### Modified
- Added `D:\SM\src\ops\security_portfolio_execution_replay_commit_preflight.rs`.
- Added `D:\SM\tests\security_portfolio_execution_replay_commit_preflight_cli.rs`.
- Updated the stock public boundary, catalog, dispatcher, contract registry, decision log, current status, and handoff issues for P19C.
- Added P19C independent-risk follow-up hard gates for P19B/P14 `document_type`, P19B/P14 `contract_version`, blocked P14 readiness, and source-level preflight-only enforcement.

### Why
- The user approved P19C scheme A and explicitly required using the newly added independent risk subprocess flow.
- P19C must freeze replay commit preflight payload hashes and idempotency candidates before any P19D runtime writer exists.
- The independent risk subprocess found that formal input identity and blocked-P14 gating were not hard enough, so the contract was tightened before final verification.

### Remaining
- [ ] P19D controlled runtime write remains undesigned and unimplemented.
- [ ] P19C remains preflight-only and must not be extended to call `security_execution_record` without a new approved P19D contract.
- [ ] The worktree remains dirty with unrelated recovery, Nikkei, capital-source, generated target, and runtime fixture artifacts; Git delivery still needs narrow staging.

### Risks
- [ ] P19C source now has a guard against direct runtime-write adapters, but future P19D work still needs a separate durable idempotency and already-committed detection design.
- [ ] Current repository-wide green was verified in the dirty `D:\SM` worktree, so it proves current local branch health but not clean Git scope.

### Closed
- RED verified: `$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_red'; cargo test --test security_portfolio_execution_replay_commit_preflight_cli -- --nocapture` failed because the P19C tool was unsupported.
- GREEN verified: `$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_green'; cargo test --test security_portfolio_execution_replay_commit_preflight_cli -- --nocapture` passed with `10 passed; 0 failed`.
- Independent risk RED verified: `$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_risk_red'; cargo test --test security_portfolio_execution_replay_commit_preflight_cli -- --nocapture` failed with 3 expected failures for missing P19B/P14 identity and blocked-P14 gating.
- Independent risk GREEN verified: `$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_risk_green'; cargo test --test security_portfolio_execution_replay_commit_preflight_cli -- --nocapture` passed with `14 passed; 0 failed`.
- Final P19C focused verification passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_final'; cargo test --test security_portfolio_execution_replay_commit_preflight_cli -- --nocapture` with `14 passed; 0 failed`.
- Final boundary guard passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture` with `4 passed; 0 failed`.
- Final catalog guard passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture` with `2 passed; 0 failed`.
- Final dispatcher guard passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19c_commit_preflight_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture` with `1 passed; 0 failed`.
- Final `cargo check` completed successfully with `D:\SM\target_p19c_commit_preflight_final`.
- Repository-wide regression passed: `$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19c'; cargo test -- --nocapture` completed with exit code 0.
## 2026-04-26
### Modified
- Added and hardened `D:\SM\docs\plans\2026-04-26-p19d-controlled-replay-commit-writer-design.md`.
- Added and hardened `D:\SM\docs\plans\2026-04-26-p19d-controlled-replay-commit-writer-plan.md`.

### Why
- The user approved P19D A1, but the independent risk subprocess found that deterministic idempotency, already-committed evidence, and source-write guards were not hard enough.
- P19D must write runtime facts only through `security_execution_record`, with replay-control conflict checks inside that path rather than a notes-only outer precheck.

### Remaining
- [ ] P19D production implementation has not started; the next step is TDD RED for `SecurityExecutionReplayCommitControl` and the P19D writer.
- [ ] P19D still needs focused tests, source guards, boundary/catalog/dispatcher sync, `cargo check`, and repository-wide regression after implementation.

### Risks
- [ ] `SecurityExecutionRecordDocument` must gain machine-readable replay metadata fields; notes-only evidence is explicitly rejected.
- [ ] `security_execution_record` must prevent conflict overwrite inside its own runtime session because the current repository upsert uses conflict update behavior.
- [ ] The worktree remains dirty with unrelated changes; future Git delivery must stage only P19D-owned files.

### Closed
- Incorporated the successful user-approved independent risk subprocess findings after one failed 429 attempt.
- Static-checked P19D design and plan for no whitespace errors with `git diff --check -- docs\plans\2026-04-26-p19d-controlled-replay-commit-writer-design.md docs\plans\2026-04-26-p19d-controlled-replay-commit-writer-plan.md`.
- Confirmed this step is documentation/planning only; P19D production code has not been modified.
## 2026-04-26
### Modified
- Added `D:\SM\src\ops\security_portfolio_execution_replay_commit_writer.rs`.
- Added `D:\SM\tests\security_portfolio_execution_replay_commit_writer_cli.rs`.
- Updated `D:\SM\src\ops\security_execution_record.rs` with `SecurityExecutionReplayCommitControl` and machine-readable replay metadata fields.
- Updated `D:\SM\src\runtime\security_execution_store_session.rs` with session-local execution-record lookup for replay conflict checks.
- Updated public stock boundary, grouped gateway, catalog, dispatcher, formal boundary guard, governance docs, handoff docs, and adjacent execution-record tests for P19D.

### Why
- The user confirmed P19D after approving A1.
- P19D must consume P19C preflight evidence and write runtime records only through `security_execution_record`.
- The independent risk subprocess required deterministic target refs, machine-readable already-committed evidence, and no direct runtime write APIs from P19D.

### Remaining
- [ ] Git delivery still needs narrow staging because the worktree contains many unrelated existing and generated changes.
- [ ] Future phases after P19D, such as lifecycle closeout or broker-fill replay, remain undesigned and out of scope.

### Risks
- [ ] P19D is intentionally per-row and non-atomic across rows; callers must read `non_atomicity_notice` and row statuses instead of assuming rollback.
- [ ] `SecurityExecutionReplayCommitControl` now extends `security_execution_record`; future changes must preserve the no-overwrite check inside the runtime session.

### Closed
- RED verified: `$env:CARGO_TARGET_DIR='D:\SM\target_p19d_execution_record_red'; cargo test --test security_execution_record_cli security_execution_record_replay_control -- --nocapture` failed with 2 expected failures for missing deterministic replay id and conflict rejection.
- GREEN verified: `$env:CARGO_TARGET_DIR='D:\SM\target_p19d_execution_record_green'; cargo test --test security_execution_record_cli security_execution_record_replay_control -- --nocapture` passed with `2 passed; 0 failed`.
- RED verified: `$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_red'; cargo test --test security_portfolio_execution_replay_commit_writer_cli tool_catalog_includes_security_portfolio_execution_replay_commit_writer -- --nocapture` failed because the P19D tool was absent from the catalog.
- Focused P19D verification passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_green'; cargo test --test security_portfolio_execution_replay_commit_writer_cli -- --nocapture` with `6 passed; 0 failed`.
- Final P19D focused verification passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo test --test security_portfolio_execution_replay_commit_writer_cli -- --nocapture` with `6 passed; 0 failed`.
- Final adjacent execution-record verification passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo test --test security_execution_record_cli -- --nocapture` with `7 passed; 0 failed`.
- Final boundary guard passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture` with `4 passed; 0 failed`.
- Final catalog guard passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture` with `2 passed; 0 failed`.
- Final dispatcher guard passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19d_commit_writer_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture` with `1 passed; 0 failed`.
- Final `cargo check` completed successfully with `D:\SM\target_p19d_commit_writer_final`.
- Repository-wide regression passed: `$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19d'; cargo test -- --nocapture` completed with exit code 0.
- Post-format repository-wide regression passed: `$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19d_fmt'; cargo test -- --nocapture` completed with exit code 0.
## 2026-04-26
### Modified
- Added `D:\SM\src\ops\security_portfolio_execution_replay_commit_audit.rs`.
- Added `D:\SM\tests\security_portfolio_execution_replay_commit_audit_cli.rs`.
- Added `D:\SM\docs\plans\2026-04-26-p19e-replay-commit-audit-design.md`.
- Added `D:\SM\docs\plans\2026-04-26-p19e-replay-commit-audit-plan.md`.
- Updated public stock boundary, grouped gateway, catalog, dispatcher, formal boundary guard, governance docs, and handoff docs for P19E.

### Why
- The user approved Scheme A after P19D: add P19E commit audit/runtime replay verification before any P20 lifecycle closeout.
- P19E must verify P19D runtime replay metadata through read-only execution-record lookup.
- The independent risk subprocess required synchronizing public boundary artifacts and avoiding another frozen-manifest drift.

### Remaining
- [ ] Git delivery still needs narrow staging because the worktree contains many unrelated existing and generated changes.
- [ ] P20 lifecycle closeout remains undesigned and must start from a separate approved contract.

### Risks
- [ ] P19E intentionally verifies replay commit metadata only; it does not prove broker fills, position materialization, or lifecycle closure.
- [ ] P19E depends on P19D machine-readable replay metadata fields remaining stable.

### Closed
- RED verified: `$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_red'; cargo test --test security_portfolio_execution_replay_commit_audit_cli -- --nocapture` failed because `security_portfolio_execution_replay_commit_audit` was unsupported and the source file was absent after correcting the test fixture recursion limit.
- Focused P19E verification passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_green'; cargo test --test security_portfolio_execution_replay_commit_audit_cli -- --nocapture` with `9 passed; 0 failed`.
- Boundary guard first exposed the expected frozen manifest drift for `security_portfolio_execution_replay_commit_audit`, then passed after manifest sync.
- Final P19E focused verification passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo test --test security_portfolio_execution_replay_commit_audit_cli -- --nocapture` with `9 passed; 0 failed`.
- Final adjacent P19D verification passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo test --test security_portfolio_execution_replay_commit_writer_cli -- --nocapture` with `6 passed; 0 failed`.
- Final boundary guard passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture` with `4 passed; 0 failed`.
- Final catalog guard passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture` with `2 passed; 0 failed`.
- Final dispatcher guard passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p19e_commit_audit_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture` with `1 passed; 0 failed`.
- Final `cargo check` completed successfully with `D:\SM\target_p19e_commit_audit_final`.
- Repository-wide regression passed: `$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p19e'; cargo test -- --nocapture` completed with exit code 0.
## 2026-04-26
### Modified
- Added `D:\SM\src\ops\security_portfolio_execution_lifecycle_closeout_readiness.rs`.
- Added `D:\SM\tests\security_portfolio_execution_lifecycle_closeout_readiness_cli.rs`.
- Updated public stock boundary, grouped gateway, catalog, dispatcher, formal boundary guard, governance docs, and handoff docs for P20A.

### Why
- The user approved P20A after P19E and required full-repository regression before continuing.
- P20A must consume P19E replay commit audit truth and emit side-effect-free closeout preflight readiness.
- P20A must not call `security_execution_record`, `security_post_trade_review`, or `security_closed_position_archive`, and must not claim lifecycle closure.

### Remaining
- [ ] Git delivery still needs narrow staging because the worktree contains many unrelated existing and generated changes.
- [ ] P20B remains undesigned and must start from a separate approved contract before adding any lifecycle writer/archive-producing behavior.

### Risks
- [ ] P20A eligibility is only preflight readiness; downstream code must not treat it as broker-fill truth, position materialization, closed archive truth, or lifecycle closure.
- [ ] Future formatting must avoid running rustfmt on root modules that recursively format frozen legacy modules; use leaf files or skip-child formatting instead.

### Closed
- RED verified: `$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_red'; cargo test --test security_portfolio_execution_lifecycle_closeout_readiness_cli -- --nocapture` failed because `security_portfolio_execution_lifecycle_closeout_readiness` was unsupported and the source file was absent.
- Focused P20A verification passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_green'; cargo test --test security_portfolio_execution_lifecycle_closeout_readiness_cli -- --nocapture` with `9 passed; 0 failed`.
- Boundary guard first exposed the expected frozen manifest drift for `security_portfolio_execution_lifecycle_closeout_readiness`, then passed after manifest sync.
- Final P20A focused verification passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_final'; cargo test --test security_portfolio_execution_lifecycle_closeout_readiness_cli -- --nocapture` with `9 passed; 0 failed`.
- Final adjacent P19E verification passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_final'; cargo test --test security_portfolio_execution_replay_commit_audit_cli -- --nocapture` with `9 passed; 0 failed`.
- Final boundary guard passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture` with `4 passed; 0 failed`.
- Final catalog guard passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture` with `2 passed; 0 failed`.
- Final dispatcher guard passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p20a_closeout_readiness_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture` with `1 passed; 0 failed`.
- Final `cargo check` completed successfully with `D:\SM\target_p20a_closeout_readiness_final`.
- First repository-wide regression attempt exposed a rustfmt recursion mistake that formatted frozen `security_decision_committee.rs`; the formatting-only drift was reverted and `security_decision_committee_legacy_freeze_source_guard` passed again.
- Repository-wide regression passed: `$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p20a'; cargo test -- --nocapture` completed with exit code 0.
## 2026-04-26
### Modified
- Added `D:\SM\docs\plans\2026-04-26-p20b-lifecycle-closeout-evidence-package-design.md`.
- Added `D:\SM\docs\plans\2026-04-26-p20b-lifecycle-closeout-evidence-package-plan.md`.
- Added `D:\SM\src\ops\security_portfolio_execution_lifecycle_closeout_evidence_package.rs`.
- Added `D:\SM\tests\security_portfolio_execution_lifecycle_closeout_evidence_package_cli.rs`.
- Updated public stock boundary, grouped gateway, catalog, dispatcher, formal boundary guard, governance docs, and handoff docs for P20B.

### Why
- The user confirmed P20B after P20A full-repository regression.
- P20B must consume P20A closeout-readiness truth and produce a read-only closeout evidence package for archive preflight.
- P20B must not write lifecycle/archive/runtime records, must not call lifecycle writer tools, and must not claim lifecycle closure.

### Remaining
- [ ] Git delivery still needs narrow staging because the worktree contains many unrelated existing and generated changes.
- [ ] Any lifecycle closeout writer or archive-producing phase remains undesigned and must start from a separate approved contract.

### Risks
- [ ] P20B evidence readiness is archive preflight evidence only; downstream code must not treat it as lifecycle closure or closed archive production.
- [ ] P20B reads runtime execution records for eligible P20A rows; missing or drifted runtime records remain explicit blockers rather than auto-repair behavior.

### Closed
- Design and implementation plan were reviewed with an inline fallback risk pass after the independent risk subprocess was blocked by external quota errors.
- RED verified: `$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_red'; cargo test --test security_portfolio_execution_lifecycle_closeout_evidence_package_cli -- --nocapture` failed with 11 expected failures for unsupported tool and missing source file.
- Focused P20B verification passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_green'; cargo test --test security_portfolio_execution_lifecycle_closeout_evidence_package_cli -- --nocapture` with `11 passed; 0 failed`.
- Boundary guard first exposed the expected frozen manifest drift for `security_portfolio_execution_lifecycle_closeout_evidence_package`, then passed after manifest sync.
- Final P20B focused verification passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_final'; cargo test --test security_portfolio_execution_lifecycle_closeout_evidence_package_cli -- --nocapture` with `11 passed; 0 failed`.
- Final adjacent P20A verification passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_final'; cargo test --test security_portfolio_execution_lifecycle_closeout_readiness_cli -- --nocapture` with `9 passed; 0 failed`.
- Final boundary guard passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_final'; cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture` with `4 passed; 0 failed`.
- Final catalog guard passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_final'; cargo test --test stock_catalog_grouping_source_guard -- --nocapture` with `2 passed; 0 failed`.
- Final dispatcher guard passed: `$env:CARGO_TARGET_DIR='D:\SM\target_p20b_closeout_evidence_final'; cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture` with `1 passed; 0 failed`.
- Final `cargo check` completed successfully with `D:\SM\target_p20b_closeout_evidence_final`.
- Repository-wide regression passed: `$env:CARGO_TARGET_DIR='D:\SM\target_repo_full_after_p20b'; cargo test -- --nocapture` completed with exit code 0.
## 2026-04-26
### Modified
- Updated `D:\SM\docs\handoff\CURRENT_STATUS.md` to mark P20B lifecycle closeout evidence package as complete instead of in progress.
- Updated the P20B handoff verification bullets to include final focused, adjacent, guard, `cargo check`, and repository-wide regression evidence.

### Why
- The user asked to continue into P20C, and the active handoff file still contained stale P20B pending language.
- P20C design must start from a truthful status baseline: P20B is complete and P20C is not yet designed.

### Remaining
- [ ] P20C lifecycle closeout/archive writer design has not been approved or written yet.
- [ ] Git delivery still needs narrow staging because the worktree contains many unrelated existing and generated changes.

### Risks
- [ ] P20C must not infer archive/lifecycle write semantics from P20B evidence readiness without a new approved contract.
- [ ] Handoff files contain older historical sections; future status claims should cite the latest P20B verification and task journal entry.

### Closed
- `git diff --check -- docs\handoff\CURRENT_STATUS.md` completed with exit code 0; Git reported only LF/CRLF normalization warnings.
## 2026-04-26
### Modified
- Added `D:\SM\docs\plans\2026-04-26-p20c-lifecycle-closeout-archive-writer-design.md`.

### Why
- The user approved P20C Scheme A: a controlled lifecycle closeout/archive writer after P20B evidence readiness.
- Local repository evidence shows no callable `security_closed_position_archive` implementation or route, so P20C must define its own explicit writer contract instead of depending on stale handoff language.

### Remaining
- [ ] P20C design still needs user approval before writing an implementation plan.
- [ ] P20C implementation plan, RED tests, runtime archive repository/schema changes, public boundary wiring, and verification are not started.
- [ ] Git delivery still needs narrow staging because the worktree contains many unrelated existing and generated changes.

### Risks
- [ ] P20C introduces the first closeout/archive write path after P20A/P20B; idempotency, conflict rejection, and partial-row semantics must be frozen before code.
- [ ] If the user wants all-row atomic closure instead of per-row non-atomic archive writes, the design must be revised before implementation.

### Closed
- P20C contract-first design now defines intent, contract, hard rejection lines, decision, acceptance gates, cross-artifact sync, and an inline fresh risk pass.
## 2026-04-26
### Modified
- Prepared the current source, test, plan, governance, and handoff state for a narrow GitHub upload on branch `codex/p10-p11-clean-upload-20260420`.

### Why
- The user asked to push the current work to GitHub before continuing P20C implementation planning.
- The worktree contains many generated targets and runtime fixtures, so upload preparation must stage source and documentation artifacts without staging generated outputs.

### Remaining
- [ ] The Git commit and push still need to be performed after narrow staging and staged diff health checks.
- [ ] P20C implementation plan and code remain unstarted.

### Risks
- [ ] The worktree remains dirty with generated artifacts and unrelated local runtime output after the upload.
- [ ] Public boundary files reference multiple active source slices, so staging must include the corresponding source and test files rather than only the latest P20C design doc.

### Closed
- Confirmed the active branch is `codex/p10-p11-clean-upload-20260420`.
- Confirmed `origin` points to `https://github.com/wakeskuld1-ctrl/StockMind.git`.
- Confirmed the branch is currently even with `origin/codex/p10-p11-clean-upload-20260420` before the new upload commit.
## 2026-04-26
### 修改内容
- 新增 `D:\SM\docs\plans\2026-04-26-nikkei-etf-position-signal-tool-plan.md`，固化日经ETF每日仓位 Tool 的实现计划和边界同步要求。
- 新增 `D:\SM\src\ops\security_nikkei_etf_position_signal.rs`，提供 `security_nikkei_etf_position_signal` 的日经指数锚定仓位信号。
- 新增 `D:\SM\tests\security_nikkei_etf_position_signal_cli.rs`，覆盖 catalog、rule_only 输出、HGB artifact 拒绝、历史不足拒绝、未来数据隔离、权重股广度确认。
- 更新 `D:\SM\src\ops\stock.rs`、`D:\SM\src\ops\stock_governance_and_positioning.rs`、`D:\SM\src\tools\catalog.rs`、`D:\SM\src\tools\dispatcher.rs`、`D:\SM\src\tools\dispatcher\stock_ops.rs`，把新 Tool 接入正式 stock Tool bus。
- 更新 `D:\SM\docs\governance\contract_registry.md`，登记日经ETF每日仓位信号契约。

### 修改原因
- 用户要求把当前日经ETF策略写成以后每天可运行一次的正式 Tool。
- 当前策略目标是交易日经ETF，日经指数作为锚定，权重股只作为买入/加仓时机和广度确认依据。
- Tool 必须避免未来函数，不能把回测未来标签或 HGB 临时状态伪装成每日可用信号。

### 方案还差什么
- [ ] `v3_hgb` 当前只做 artifact 必填拒绝，还没有实现受治理模型 artifact 的真实推理。
- [ ] 当前组件广度支持本地 CSV 权重和组件日线目录，尚未绑定官方前30权重文件的字段兼容矩阵。
- [ ] 日经成交额/量能 proxy 已作为输入字段保留，但本轮尚未把 3D/20D 放量突破确认正式纳入输出。

### 潜在问题
- [ ] 当前 rule_only 是可运行的 V3/广度第一版，不等同于之前 walk-forward 里 HGB增强V3 的完整收益结果。
- [ ] 如果实际组件 CSV 文件名或列名与测试夹具不同，后续每日运行前需要补兼容解析。
- [ ] 当前工作区已有大量其他未提交变更，本轮只应窄范围提交新增 Tool 相关文件。

### 关闭项
- 已验证 RED：`cargo test --test security_nikkei_etf_position_signal_cli -- --nocapture` 初始失败，原因是 catalog 缺少新 Tool 且 dispatcher 返回 unsupported tool。
- 已验证组件广度 RED：`cargo test --test security_nikkei_etf_position_signal_cli security_nikkei_etf_position_signal_uses_component_breadth_when_supplied -- --nocapture` 失败，原因是实现尚未读取组件广度。
- Focused Tool 测试通过：`cargo test --test security_nikkei_etf_position_signal_cli -- --nocapture`，`6 passed; 0 failed`。
- Catalog guard 通过：`cargo test --test stock_catalog_grouping_source_guard -- --nocapture`，`2 passed; 0 failed`。
- Dispatcher guard 通过：`cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture`，`1 passed; 0 failed`。
## 2026-04-26
### 修改内容
- 扩展 `D:\SM\src\ops\security_nikkei_etf_position_signal.rs`，新增 `volume_signal` 与 `volume_metrics` 输出。
- 扩展 `D:\SM\src\ops\security_nikkei_etf_position_signal.rs`，实现 `v3_hgb` 模式读取 `nikkei_v3_hgb_adjustment.v1` 每日调仓 artifact。
- 扩展 `D:\SM\tests\security_nikkei_etf_position_signal_cli.rs`，新增量能突破确认测试和 HGB artifact 调仓测试。
- 更新 `D:\SM\docs\governance\contract_registry.md` 与 `D:\SM\docs\plans\2026-04-26-nikkei-etf-position-signal-tool-plan.md`，同步量能与 HGB artifact 契约。

### 修改原因
- 用户要求继续做完日经ETF每日策略 Tool。
- 上一轮只完成 rule_only、组件广度、HGB 缺 artifact 拒绝；仍缺量能确认和可审计 HGB 接入边界。
- 日常运行必须避免把历史回测 log 当作未来推理结果，因此 HGB 只接受明确日期匹配的每日 artifact。

### 方案还差什么
- [ ] 尚未实现 Rust 内部训练/执行 HGB 模型；当前接入的是已生成的每日 HGB 调仓 artifact。
- [ ] 量能确认当前使用 volume proxy 的 3D 平均 / 前20D 平均，并叠加20D突破；如果后续要复刻 60D 或组件放量突破阈值，需要新增独立契约。

### 潜在问题
- [ ] 如果每日 HGB artifact 没有由训练系统稳定产出，`v3_hgb` 模式会拒绝运行，不能自动降级伪装成模型输出。
- [ ] 当前 volume proxy 读取 `volume` 字段，若 volume 为 0 则回退使用 `close`，适配成交额 proxy 但要求输入源语义清晰。

### 关闭项
- 已验证量能 RED：`cargo test --test security_nikkei_etf_position_signal_cli security_nikkei_etf_position_signal_confirms_volume_backed_breakout -- --nocapture` 初始失败，原因是输出缺少 `volume_signal`。
- 已验证 HGB RED：`cargo test --test security_nikkei_etf_position_signal_cli security_nikkei_etf_position_signal_applies_hgb_adjustment_artifact -- --nocapture` 初始失败，原因是 `hgb_adjustment` 仍为 `0.0`。
- Focused Tool 测试通过：`cargo test --test security_nikkei_etf_position_signal_cli -- --nocapture`，`8 passed; 0 failed`。
## 2026-04-26
### 修改内容
- 新增研究脚本 `D:\.stockmind_runtime\a_share_dynamic_hs300_backtest_20260426\practical_strategy_A_backtest.py`，实现沪深300动态成分“方案A：实盘约束版”20D突破、3D试仓、5D加仓、10D加满、跌回阻力止损、市场状态仓位上限、单日最多10只和交易成本回测。
- 新增测试 `D:\.stockmind_runtime\a_share_dynamic_hs300_backtest_20260426\test_practical_strategy_A.py`，覆盖印花税切换、市场状态、分层加仓止损、单日买入上限。
- 导出实盘约束研究产物：组合曲线、交易账本、批次汇总、年度汇总、与裸3D策略对比表和运行 meta。
### 修改原因
- 用户批准方案A，要求把已识别的实盘缺口补入沪深300回测，而不是继续使用无试仓/无止损/无市场仓位上限/无交易成本的裸策略。
- 当前任务明确为研究回测产物，不新增正式 Rust Tool。
### 方案还差什么
- [ ] 尚未建模涨跌停、停牌、真实撮合、ETF申赎/溢价和盘中成交可得性。
- [ ] 市场状态暂用 510300 ETF 的 MA50/MA200/MA200斜率近似，不等同于正式沪深300指数全量状态引擎。
- [ ] 该脚本位于 runtime 研究目录，尚未纳入正式 Tool 或稳定 CLI 契约。
### 潜在问题
- [ ] 同收盘价止损是假设条件，可能高估极端行情下可执行性。
- [ ] 方案A显著增加交易次数，成本和假突破损耗对结果影响较大，需要继续拆解交易频率和重复信号过滤。
### 关闭项
- RED 已验证：`python -m pytest D:\.stockmind_runtime\a_share_dynamic_hs300_backtest_20260426\test_practical_strategy_A.py -q` 初始失败，原因为 `ModuleNotFoundError: No module named 'practical_strategy_A_backtest'`。
- 规则测试已通过：同一测试命令最终 `4 passed`。
- 完整回测已执行：`python D:\.stockmind_runtime\a_share_dynamic_hs300_backtest_20260426\practical_strategy_A_backtest.py`，期末权益 `1276157.9825888462`。
## 2026-04-26
### 修改内容
- 新增 `D:\SM\docs\plans\2026-04-26-nikkei-etf-live-like-backtest-plan.md`，记录日经ETF实盘化研究回测的临时契约、口径和验收条件。
- 导出研究回测产物到 `D:\.stockmind_runtime\nikkei_etf_live_like_backtest_20260426`，包含T-1信号、T日开盘执行、交易成本、溢价过滤和调仓死区的对比结果。

### 修改原因
- 用户要求先按方案B做研究回测，观察实盘化口径对159866和513520收益、Sharpe、回撤、交易频率的影响，然后再决定是否升级正式Tool。
- 当前正式Tool仍未改动，避免把临时研究参数直接固化为生产契约。

### 方案还差什么
- [ ] 尚未把T-1信号/T日开盘执行、溢价过滤、调仓死区纳入正式 `security_nikkei_etf_position_signal` Tool。
- [ ] 尚未用真实盘中IOPV验证开盘溢价；当前研究使用 `开盘价 / 当日收盘后NAV - 1` 作为代理口径。
- [ ] 尚未针对过滤阈值做参数扫描，例如0.5%、1%、1.5%、2%溢价阈值和5%、10%、15%调仓死区。

### 潜在问题
- [ ] 同一ETF执行日的多个HGB信号在研究中合并为最新信号，更符合实盘，但与旧回测逐条执行口径不同。
- [ ] 过滤版收益下降可能来自错过高溢价后的继续上涨，不能直接等同于模型失效。
- [ ] ETF开盘成交可得性、滑点、实时溢价估算仍需实盘接口或券商成交数据验证。

### 关闭项
- 已生成 `01_live_like_operation_ledger.csv`、`02_live_like_equity_curve.csv`、`03_live_like_summary.csv`、`04_signal_execution_schedule_after_consolidation.csv`、`05_live_like_rule_audit.csv`。
- 审计通过：所有执行日均晚于信号日；过滤版未在开盘溢价代理值超过2%时买入；过滤版未执行低于10%仓位差的交易。
## 2026-04-26
### 修改内容
- 在 `D:\.stockmind_runtime\nikkei_etf_live_like_backtest_20260426` 新增无调仓死区研究结果：`06_dual_low_premium_no_deadband_ledger.csv`、`07_dual_low_premium_no_deadband_curve.csv`、`08_no_deadband_decision_summary.csv`、`09_no_deadband_decision_audit.csv`。
- 新增“双ETF买入时选择开盘溢价代理值更低的一只、卖出按持仓比例减仓、无调仓死区、无高溢价硬阻断、3bp成本”的研究口径。

### 修改原因
- 用户认可高溢价不应一刀切过滤，并要求去掉调仓死区后重新计算收益、年化、Sharpe、回撤等决策指标。

### 方案还差什么
- [ ] 尚未扫描极端溢价阈值；本轮组合版完全不阻断买入，只在159866和513520之间选择相对低溢价。
- [ ] 尚未纳入真实盘中IOPV，仍使用 `开盘价 / 当日NAV - 1` 的代理溢价。
- [ ] 尚未把组合择优逻辑纳入正式Tool。

### 潜在问题
- [ ] 双ETF组合策略可能造成持仓分散，后续正式化时需要定义是否允许长期同时持有两只ETF。
- [ ] 如果两只ETF未来跟踪误差、流动性、费用结构变化，简单低溢价择优可能需要增加流动性约束。

### 关闭项
- 审计通过：组合版所有执行日均晚于信号日；买入时均选择开盘溢价代理值更低的一只；流水非空。
## 2026-04-26
### 修改内容
- 扩展 `D:\SM\src\ops\security_nikkei_etf_position_signal.rs`，在原有日经ETF目标仓位信号之外新增可选 `NikkeiEtfLiveExecutionPlan` 输出。
- 新增 live execution 输入字段：`planned_execution_date`、`current_cash_cny`、`current_positions`、`execution_quotes`、`commission_rate`、`extreme_premium_block_pct`。
- 固化方案A实盘规则：T-1信号/T日next-open计划、买入选择开盘溢价代理值更低的ETF、卖出优先卖高溢价ETF、无调仓死区、默认3bp成本、仅当全部候选ETF超过极端溢价阈值时阻断买入。
- 扩展 `D:\SM\tests\security_nikkei_etf_position_signal_cli.rs`，新增低溢价买入、极端溢价阻断、高溢价优先卖出三个CLI集成测试。
- 更新 `D:\SM\docs\governance\contract_registry.md`，同步 `security_nikkei_etf_position_signal` 的可选实盘执行计划合同。

### 修改原因
- 用户确认从研究方案B升级到正式方案A，希望以后直接通过Tool输出日经ETF实盘买卖计划，而不是继续手工临时计算。

### 方案还差什么
- [ ] 尚未接入真实盘中IOPV；当前执行计划中的溢价仍明确为 `open_price / nav - 1` 代理口径。
- [ ] 尚未实现自动生成每日 `nikkei_v3_hgb_adjustment.v1` artifact；`v3_hgb` 仍要求外部显式传入已治理artifact。
- [ ] 尚未增加每日自动化运行入口或定时任务。

### 潜在问题
- [ ] 如果券商实际成交价偏离开盘价，执行计划仍需用成交后复盘修正。
- [ ] 双ETF择优依赖调用方同时提供两个ETF的有效 `execution_quotes`，否则只能在已提供候选里择优。
- [ ] 极端溢价阈值默认5%，后续可能需要基于真实盘中IOPV重新校准。

### 关闭项
- RED已验证：新增live execution字段在旧合同下被拒绝，`cargo test --test security_nikkei_etf_position_signal_cli -- --nocapture` 初始出现3个失败用例。
- Focused Tool测试通过：`cargo test --test security_nikkei_etf_position_signal_cli -- --nocapture`，11 passed。
- Catalog guard通过：`cargo test --test stock_catalog_grouping_source_guard -- --nocapture`，2 passed。
- Dispatcher guard通过：`cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture`，1 passed。
- 编译检查通过：`cargo check`。
## 2026-04-27
### 修改内容
- 修正研究脚本 `D:\.stockmind_runtime\nikkei_etf_daily_model_scoring_20260427\daily_hgb_rf_v3_scoring.py` 的每日 adjustment JSON 命名，将 `train_policy` 写入文件名。
- 重新运行 `live_pre_year` 和 `known_labels_asof` 两套研究口径，生成 HGB 增强V3 与 RF 增强V3 的每日评分、验证指标、全局重要度、本地解释和最新 adjustment artifact。
### 修改原因
- 用户要求先按方案A查看 HGB 可解释性，并补充 RF 增强V3 训练结果。
- 原 JSON 文件名未包含训练口径，连续运行实盘口径和诊断口径时会互相覆盖，导致文件名显示 live 但内容实际为 `known_labels_asof`。
### 方案还差什么
- [ ] 该研究脚本仍未接入正式 Rust Tool，每日自动化入口和正式 artifact 治理还未固化。
- [ ] 尚未清理旧的无口径 JSON 文件，后续读取时应优先使用带 `live_pre_year` 或 `known_labels_asof` 的新文件名。
### 潜在问题
- [ ] `known_labels_asof` 包含已完成未来标签的诊断信息，不应作为实盘信号依据。
- [ ] `live_pre_year` 的验证平衡准确率不高，模型建议应继续结合 HGB/RF 分歧和突破站稳规则做风控解释。
### 关闭项
- 已稳定复现旧 artifact 覆盖问题：`hgb_l2_leaf20_live_2026-04-24_adjustment.json` 文件名显示 live，但内部 `train_policy=known_labels_asof`。
- 已修复 JSON 命名冲突，并重新运行两套口径。
- 已验证新产物包含独立文件：`hgb_l2_leaf20_live_live_pre_year_2026-04-24_adjustment.json`、`rf_depth4_leaf20_live_live_pre_year_2026-04-24_adjustment.json`、`hgb_l2_leaf20_live_known_labels_asof_2026-04-24_adjustment.json`、`rf_depth4_leaf20_live_known_labels_asof_2026-04-24_adjustment.json`。
## 2026-04-27
### 修改内容
- 新增研究包 `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427`，纳入日经ETF HGB/RF V3 研究链路的训练/中间过程、实盘化回测、每日评分产物和哈希清单。
- 新增 `README.md`、`ALGORITHM_HANDOFF_MANUAL.md`、`UPLOAD_NOTES.md`，说明模型研究思路、算法交接、验证证据、可用口径和禁止误用项。
- 更新 `D:\SM\docs\handoff\CURRENT_STATUS.md` 与 `D:\SM\docs\handoff\AI_HANDOFF.md`，把研究包作为后续日经ETF模型工作的恢复入口。
### 修改原因
- 用户要求按方案B把研究模型、中间过程、测试、相关模型打包上传到 GitHub，并把 AI 的研究过程和研究思路写成算法交接手册。
- 当前工作区存在大量无关生成物和历史 fixture，必须用窄范围研究包保留日经ETF主线全量证据，同时避免把无关大目录污染仓库。
### 方案还差什么
- [ ] 尚未把每日 HGB/RF 评分脚本改成默认使用仓库内相对路径；当前脚本仍保留原 runtime 绝对路径默认值。
- [ ] 尚未把每日 HGB/RF artifact 生成纳入正式 Rust Tool 或自动化任务。
### 潜在问题
- [ ] `known_labels_asof` 只能用于诊断，不能作为实盘信号。
- [ ] 旧的无 `train_policy` JSON 被保留用于追溯，但后续读取必须优先使用带口径的新文件名。
- [ ] A股/沪深300旁路实验目录约 577.64MB，未纳入本次日经ETF研究包，若后续重启A股模型需单独打包。
### 关闭项
- 已生成 `artifact_manifest.csv`，覆盖 210 个研究产物文件，总计约 15.84MB。
- 已确认日经ETF相关三段 runtime 快照分别为：每日HGB/RF评分 17 文件、实盘化回测 12 文件、训练/中间过程 181 文件。
- 已把算法交接手册和研究包入口写入 handoff 文档。
## 2026-04-27
### 修改内容
- 补充研究包上传前验证记录，覆盖仓库内快照复算、研究产物哈希校验、日经ETF Tool focused 测试、catalog/dispatcher guard 和 `cargo check`。
### 修改原因
- 上传到 GitHub 前必须留下可追溯验证证据，避免只上传数据而无法判断包是否可恢复、Tool 是否仍可编译。
### 方案还差什么
- [ ] 尚未执行全仓库 `cargo test -- --nocapture`，本轮仅做日经ETF相关 focused 验证和边界 guard。
### 潜在问题
- [ ] 当前工作区仍存在大量无关脏文件和生成目录，提交时必须窄范围 stage。
### 关闭项
- 仓库内快照复算通过：`daily_hgb_rf_v3_scoring.py --analysis-root docs\research\...\01_training_and_intermediate_full_snapshot\analysis_exports --output-root D:\.stockmind_runtime\nikkei_package_verify_20260427 --train-policy live_pre_year`。
- 研究包哈希校验通过：`artifact manifest hash check passed: 210/210`。
- Focused Tool 测试通过：`cargo test --test security_nikkei_etf_position_signal_cli -- --nocapture`，`11 passed; 0 failed`。
- Catalog guard 通过：`cargo test --test stock_catalog_grouping_source_guard -- --nocapture`，`2 passed; 0 failed`。
- Dispatcher guard 通过：`cargo test --test stock_dispatcher_grouping_source_guard -- --nocapture`，`1 passed; 0 failed`。
- 编译检查通过：`cargo check`。
