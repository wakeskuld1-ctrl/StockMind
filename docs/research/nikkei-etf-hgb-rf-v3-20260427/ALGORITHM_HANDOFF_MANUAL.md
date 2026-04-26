# Nikkei ETF HGB/RF V3 Algorithm Handoff Manual

## Start Here

Read these files first:

1. `README.md`
2. `artifact_manifest.csv`
3. `artifacts/03_daily_hgb_rf_scoring_full_snapshot/01_daily_model_scores_live_pre_year.csv`
4. `artifacts/03_daily_hgb_rf_scoring_full_snapshot/04_local_driver_explanations_live_pre_year.csv`
5. `artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/adjustment_point_analysis/64_walk_forward_hgb_backtest_summary.csv`
6. `artifacts/02_live_like_backtest_full_snapshot/08_no_deadband_decision_summary.csv`

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
- `<model_id>_<policy>_<as_of_date>_adjustment.json`

Do not use non-policy JSON filenames. They are deprecated.

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

Current limitation:

- It can consume governed HGB adjustment artifacts.
- It does not yet train HGB/RF internally.
- It still depends on external daily artifact generation for model mode.

If formalizing this research into production, the next design should decide whether:

1. Rust Tool continues to consume daily JSON artifacts.
2. Python research script becomes a governed preprocessing job.
3. Rust Tool grows a first-class model scoring subsystem.

Option 2 is the safer next step.

## Standard Daily Operating Procedure

1. Refresh Nikkei index, volume, and component data.
2. Run `daily_hgb_rf_v3_scoring.py --train-policy live_pre_year`.
3. Verify the latest policy-qualified JSON exists for the latest market date.
4. Read `05_latest_adjustment_artifacts_live_pre_year.csv`.
5. If HGB and RF disagree, inspect `04_local_driver_explanations_live_pre_year.csv`.
6. Feed the selected HGB artifact into `security_nikkei_etf_position_signal` only after confirming the artifact date matches the intended signal date.
7. Use ETF premium/quote data for execution selection.

## Common Mistakes To Avoid

- Do not treat `known_labels_asof` as live.
- Do not read old JSON filenames without `train_policy`.
- Do not interpret "breakout" alone as a buy signal.
- Do not replace HGB only because RF has higher raw accuracy; RF's balanced accuracy is worse in the current validation.
- Do not confuse the model signal date with the ETF execution date.
- Do not treat ETF premium backtest proxies as real-time IOPV validation.
- Do not upload the 577MB A-share/HS300 runtime into this Nikkei package.

## Next Recommended Work

1. Make `daily_hgb_rf_v3_scoring.py` path-configurable by defaulting to packaged relative paths when available.
2. Add a governed daily artifact writer that emits only policy-qualified JSON.
3. Add a formal model-run manifest: input hash, training window, validation window, output date, and model version.
4. Add a small CLI or script that prints the current HGB/RF decision table in Chinese.
5. Later, decide whether to integrate the daily artifact generator with the Rust Tool or keep it as a separate preprocessing job.
