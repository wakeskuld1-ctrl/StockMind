# Nikkei ETF HGB/RF V3 Algorithm Handoff Manual

## Start Here

Read these files first:

1. `README.md`
2. `PREDICTION_METHODS_HANDOFF_20260429.md`
3. `MODEL_SUMMARY_20260428.md`
4. `REPLAY_CLASSIFIER_SUMMARY_20260428.md`
5. `CONTINUATION_HEAD_SUMMARY_20260428.md`
6. `artifact_manifest.csv`
7. `artifacts/03_daily_hgb_rf_scoring_full_snapshot/01_daily_model_scores_live_pre_year.csv`
8. `artifacts/03_daily_hgb_rf_scoring_full_snapshot/04_local_driver_explanations_live_pre_year.csv`
9. `artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/adjustment_point_analysis/64_walk_forward_hgb_backtest_summary.csv`
10. `artifacts/02_live_like_backtest_full_snapshot/08_no_deadband_decision_summary.csv`

If the immediate question is only about the two prediction-enhancement methods,
read `PREDICTION_METHODS_HANDOFF_20260429.md` first. That document is the
shortest path into the Replay Classifier and Continuation Head line.

The current practical question is not "predict Nikkei up/down every day". The operating question is:

When the Nikkei anchor reaches a position-adjustment point, what risk position should the ETF strategy carry, and which ETF should be used for execution?

## Research Evolution

### Phase 1: Direction Prediction Was Too Broad

The initial direction-prediction framing was too broad. In a strong bull market, many 10-day forward labels become positive, so a generic "will rise in 10 days" label can generalize away the real decision.

The research shifted toward adjustment-point prediction:

- Buy/add only after breakout and standing confirmation.
- Reduce risk when price becomes extended from support or downside behavior appears.
- Use the model to move target position, not to forecast every small daily move.

### Phase 2: Breakout Stability Became the Entry Logic

The useful operational signal became:

- Breakout above resistance.
- Check whether price falls back below resistance after 3D, 5D, and 10D.
- 3D standing can be treated as trial position.
- 5D standing is more reliable for add/open confirmation.
- 10D helps evaluate whether the breakout has matured or become late.

Relevant artifacts:

- `07_breakout_3d_5d_10d_decision_table.csv`
- `08_breakout_fallback_volume_overlay.csv`
- `12_entry_timing_training_samples_scheme_b.csv`
- `13_entry_timing_training_summary_scheme_b.csv`

### Phase 3: Volume Was Added as Confirmation, Not a Standalone Rule

Volume alone was not enough. The model needed to distinguish:

- Index-level breakout.
- Component-weighted volume breadth.
- Volume with price behavior.
- Downside volume during weak days.

Relevant features include:

- `weighted_b20_vol`
- `weighted_b60_vol`
- `weighted_vol_down`
- `weighted_bd20_vol`
- `component_above200_breadth`
- `avg_component_vr`

The conclusion was that volume matters most when attached to price location and support/resistance context.

### Phase 4: HGB Enhanced V3 Became the Main Risk-Position Model

The HGB enhanced V3 model was retained because its walk-forward backtest gave better risk-adjusted behavior than the simple base rule:

- It reduced max drawdown materially versus buy-and-hold.
- It produced higher Sharpe than buy-and-hold in the 2022-2026 window.
- It remained explainable through feature importance and local driver tables.

Important artifact:

`64_walk_forward_hgb_backtest_summary.csv`

### Phase 5: RF Enhanced V3 Was Added as a Secondary Opinion

RF enhanced V3 was added for comparison, not as a replacement.

Live-like validation on previous Q4:

| Model | Accuracy | Balanced Accuracy | Behavior |
|---|---:|---:|---|
| HGB | 48.39% | 40.74% | More willing to flag risk reduction. |
| RF | 54.84% | 31.48% | More neutral-biased, weaker minority-class balance. |

Practical interpretation:

- HGB is the primary risk-position model.
- RF is a stabilizer or disagreement detector.
- If HGB says reduce and RF says hold, do not immediately declare the model broken; inspect support distance, volume breadth, and trend breadth.

### Phase 6: Replay Classification Became the First Signal-Quality Gate

Replay classification was added because generic weekly direction was too broad for the real trading question.

The replay layer grades whether a governed action was:

- correct
- acceptable
- premature
- late

across `1D / 3D / 5D`.

Use:

- `REPLAY_CLASSIFIER_SUMMARY_20260428.md`
- `artifacts/04_replay_classifier_full_snapshot/`

### Phase 7: Continuation Head Was Added as a Second-Stage Refinement

A continuation head now sits after replay classification and reuses the same event-anchored sample base.

Its role is narrower:

- separate usable continuation from stop-quality continuation;
- operate as a research-only refinement layer;
- not replace HGB risk-position logic;
- not act as a standalone live execution signal.

Use:

- `CONTINUATION_HEAD_SUMMARY_20260428.md`
- `artifacts/05_continuation_head_full_snapshot/`

## Model Objects

### Label

The V3 adjustment model uses adjustment classes:

- `-1`: reduce risk / hold low
- `0`: keep base position
- `1`: buy or add

The daily scoring script maps these to a target-position proxy:

- `target = base_position_v3 - 0.25` for `-1`
- `target = base_position_v3` for `0`
- `target = base_position_v3 + 0.25` for `1`

This is a proxy for research interpretation. The formal execution tool may add portfolio constraints, ETF choice, cost, and premium logic.

### Main Feature Families

Price location:

- `dist_res20`: distance from 20D resistance.
- `dist_sup20`: distance from 20D support.
- `dist_sup60`: distance from 60D support.
- `dist_ma20`, `dist_ma50`, `dist_ma200`.

Breakout/breakdown:

- `breakout20`
- `breakout60`
- `breakdown20`
- `breakdown60`

Trend/regime:

- `ma50_over_ma200`
- `ma200_slope20`
- `regime_bull`
- `regime_range`
- `regime_bear`

Volume and component breadth:

- `volume_ratio60`
- `weighted_b20_vol`
- `weighted_b60_vol`
- `weighted_vol_down`
- `weighted_bd20_vol`
- `component_above200_breadth`
- `avg_component_vr`

Base position:

- `base_position_v3`

### HGB Global Importance, Live Policy

The top positive permutation-importance features were:

- `dist_sup60`
- `dist_res20`
- `dist_sup20`
- `weighted_b60_vol`
- `ma50_over_ma200`
- `dist_ma200`

Interpretation:

HGB cares heavily about where the index is relative to support/resistance. It tends to reduce risk when the market has moved far from support, especially when volume breadth becomes elevated.

### RF Global Importance, Live Policy

The top positive permutation-importance features were:

- `ma50_over_ma200`
- `dist_sup20`
- `component_above200_breadth`
- `dist_ma200`
- `dist_sup60`
- `dist_res20`

Interpretation:

RF gives more weight to trend structure and breadth. This explains why RF can hold neutral while HGB reduces risk during extended bull-market moves.

## Training Policies

### `live_pre_year`

This is the live-like policy used for current interpretation:

- Train through 2025-09-30.
- Validate on 2025Q4.
- Score 2026 rows without using completed 2026 forward labels.

Use this policy for live discussion.

### `known_labels_asof`

This is diagnostic only:

- Includes rows whose forward-label horizon has completed by `as_of_date`.
- Useful for understanding model behavior after labels are known.
- Not acceptable as a live trading signal.

## Approved Live Cadence (2026-04-28)

The approved operating cadence is now:

1. Expand-window retrain.
2. Daily walk-forward scoring.
3. Consume only governed `live_pre_year` artifacts.

Interpretation:

- The governed retrain / yearly walk-forward side should follow the approved expanding-window cadence, not a frozen old one-off run.
- The daily live workflow side should behave like a daily walk-forward process, not a retrospective diagnostic read.
- `known_labels_asof` remains a diagnostic comparison surface only.
- The daily operator path must expose both `as_of_date` and the actually usable `effective_signal_date`.

Important precision:

- In the current 2026 `live_pre_year` daily workflow, the train/validate split is fixed at `train through 2025-09-30` and `validate on 2025Q4`, then the workflow scores daily rows for the requested range.
- The expanding-window cadence is expressed on the governed retrain / yearly HGB walk-forward side, not as a day-by-day 2026 train-window rewrite inside the operator workflow.

If the requested `as_of_date` is later than the last completed live-policy market row, the workflow is allowed to fall back to the latest available live artifact on or before that date. That fallback is valid only when:

- `train_policy = live_pre_year`
- the artifact filename is policy-qualified
- the workflow summary and manifest both expose the fallback explicitly

Implementation field names:

- artifact table: `requested_as_of_date`, `effective_as_of_date`
- stdout summary: `effective_signal_date=...`
- workflow manifest: `latest_artifact_as_of_date`

This prevents a false assumption that "requested date" always equals "signal date".

## Daily Scoring Process

The script is:

`artifacts/03_daily_hgb_rf_scoring_full_snapshot/daily_hgb_rf_v3_scoring.py`

It does four things:

1. Loads the V3 adjustment training frame.
2. Builds a live daily feature frame from Nikkei index, volume, and component data.
3. Trains HGB and RF according to the selected policy.
4. Writes daily scores, validation metrics, global importance, local driver explanations, and latest JSON artifacts.

Output files:

- `01_daily_model_scores_<policy>.csv`
- `02_model_validation_metrics_<policy>.csv`
- `03_global_feature_importance_<policy>.csv`
- `04_local_driver_explanations_<policy>.csv`
- `05_latest_adjustment_artifacts_<policy>.csv`
- `06_daily_workflow_manifest_<policy>.json`
- `<model_id>_<policy>_<as_of_date>_adjustment.json`

Do not use non-policy JSON filenames. They are deprecated.

The operator entrypoint is now:

`scripts/run_nikkei_hgb_rf_daily_workflow.py`

This workflow:

1. Runs one governed `live_pre_year` scoring batch.
2. Reloads the written live artifact table from disk.
3. Reloads the matching workflow manifest from disk.
4. Prints a stable HGB/RF summary for operators.
5. Optionally persists signal facts into `docs/trading-journal/nikkei/` through the governed journal skill script.

Recommended operator command:

`python D:\SM\scripts\run_nikkei_hgb_rf_daily_workflow.py --as-of-date 2026-04-27 --score-start-date 2026-04-01 --journal-dir D:\SM\docs\trading-journal\nikkei`

Important reading rule:

- `as_of_date` is the requested workflow date.
- `effective_signal_date` is the actual latest usable live signal date.
- `requested_as_of_date` / `effective_as_of_date` live in the artifact table.
- `latest_artifact_as_of_date` lives in `06_daily_workflow_manifest_live_pre_year.json`.
- Live interpretation must follow `effective_signal_date`, not a guessed market date.

Journal outputs owned by this workflow live at:

- `D:\SM\docs\trading-journal\nikkei\journal.csv`
- `D:\SM\docs\trading-journal\nikkei\journal.md`
- `D:\SM\docs\trading-journal\nikkei\snapshots\<signal_date>_<etf_symbol>.json`

Rating-change replay command:

`python C:\Users\wakes\.codex\skills\nikkei-live-journal\scripts\compare_rating_change.py --journal-dir D:\SM\docs\trading-journal\nikkei --signal-date <date> --etf-symbol <symbol>`

## Local Explanation Method

The local driver table is an explanation proxy, not true SHAP.

It ranks features by combining:

- Global permutation importance.
- Feature z-score against the training distribution.
- Current feature value on the target day.

This is good enough to answer "which features are unusual and important today", but it does not prove exact tree-path causality.

If production-grade explainability is required, add explicit HGB tree-path extraction or SHAP-like analysis as a separate governed enhancement.

## 2026-04 Key Interpretation

The important April 2026 pattern:

- 2026-04-08: 20D breakout with high `weighted_b20_vol`; both HGB and RF reduced risk.
- 2026-04-16: 20D/60D breakout, far above support; both reduced risk.
- 2026-04-22: breakout still present, but both models held neutral.
- 2026-04-24: HGB reduced risk, RF held neutral.

The disagreement on 2026-04-24 is explainable:

- HGB saw high extension from support plus high `weighted_b60_vol`.
- RF saw trend and breadth still acceptable.

This should be treated as "do not add; inspect risk", not as a buy signal.

## ETF Execution Layer

The model is anchored on Nikkei behavior. The ETF execution layer applies China-market ETF choices:

- 159866
- 513520

The strongest practical variant currently captured is:

`dual_low_premium_buy_no_deadband_3bp`

Rule summary:

- Use T-1 close information for signal.
- Execute at next available open in research backtest.
- Buy the ETF with lower open premium proxy.
- Sell according to target-position reduction.
- Use 3bp cost.
- No rebalance deadband.

This layer is not identical to the HGB model. HGB produces risk-position intent; ETF execution chooses the instrument and trade mechanics.

## Formal Tool Boundary

The formal Rust tool exists at:

`src/ops/security_nikkei_etf_position_signal.rs`

Current governed behavior:

- It consumes governed HGB adjustment artifacts only.
- It accepts only `live_pre_year` artifacts for model mode.
- It rejects `known_labels_asof` artifacts for live use.
- It rejects deprecated non-policy JSON filenames even if the JSON body was hand-edited.
- It rejects stale filename/date mismatches against the governed live naming contract.
- It still does not train HGB/RF internally and depends on external daily artifact generation.

If formalizing this research into production, the next design should decide whether:

1. Rust Tool continues to consume daily JSON artifacts.
2. Python research script becomes a governed preprocessing job.
3. Rust Tool grows a first-class model scoring subsystem.

Option 2 is the safer next step.

## Standard Daily Operating Procedure

1. Refresh Nikkei index, volume, and component data.
2. Run `python D:\SM\scripts\run_nikkei_hgb_rf_daily_workflow.py --as-of-date <date> --score-start-date <date> --journal-dir D:\SM\docs\trading-journal\nikkei`.
3. Read the printed summary and confirm both `as_of_date` and `effective_signal_date`.
4. Verify `06_daily_workflow_manifest_live_pre_year.json` and `05_latest_adjustment_artifacts_live_pre_year.csv` agree on the same live-policy run.
5. If HGB and RF disagree, inspect `04_local_driver_explanations_live_pre_year.csv`.
6. Feed the selected HGB artifact into `security_nikkei_etf_position_signal` only after confirming the artifact filename is policy-qualified and the live interpretation follows `effective_signal_date`.
7. Use ETF premium/quote data for execution selection.

## Common Mistakes To Avoid

- Do not treat `known_labels_asof` as live.
- Do not read old JSON filenames without `train_policy`.
- Do not assume `as_of_date` equals `effective_signal_date`.
- Do not interpret "breakout" alone as a buy signal.
- Do not replace HGB only because RF has higher raw accuracy; RF's balanced accuracy is worse in the current validation.
- Do not confuse the model signal date with the ETF execution date.
- Do not treat ETF premium backtest proxies as real-time IOPV validation.
- Do not treat any new `10d` Nikkei weekly registry/refit suffix as acceptable. The code/test path is fixed and the packaged research snapshot was rerun on 2026-04-28, so any new `10d` suffix now indicates a regression rather than historical residue.
- Do not upload the 577MB A-share/HS300 runtime into this Nikkei package.

## Next Recommended Work

1. Make `daily_hgb_rf_v3_scoring.py` path-configurable by defaulting to packaged relative paths when available.
2. Add a governed daily artifact writer that emits only policy-qualified JSON.
3. Add a formal model-run manifest: input hash, training window, validation window, output date, and model version.
4. Add a small CLI or script that prints the current HGB/RF decision table in Chinese.
5. Later, decide whether to integrate the daily artifact generator with the Rust Tool or keep it as a separate preprocessing job.
