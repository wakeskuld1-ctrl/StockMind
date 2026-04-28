# Nikkei 5D Slow-Fail Prototype Design

## Intent
- Goal: improve the weakest continuation horizon by separating `5D premature_add` negatives from the current shared `1D/3D` prototype-add rule.
- Scope: keep the current prototype-add mining rules for `1D/3D`, add a dedicated `5D` slow-fail prototype rule, rerun the same real-validation experiment, and compare whether `5D balanced_accuracy` and `5D` negative recall improve.
- Non-goals: do not redesign the whole continuation stack, do not introduce `reduce` mining in this round, do not merge the result into governed training by default, and do not change Rust Tool boundaries.
- Success definition: the project can mine a separately tagged `5D`-specific prototype-add pool and show whether the weakest horizon improves without breaking the current `1D/3D` lane.
- Delivery form: one design doc, one implementation plan, updated Python tests/scripts, refreshed experiment artifacts, and one short summary update.

## Contract

### Core Objects
- `prototype_add_failure_row`: one separately tagged mined row used by the current `1D/3D` prototype-add lane.
- `slow_fail_5d_row`: one separately tagged mined row used only when `label_horizon = 5d`.
- `5d_slow_fail_experiment`: one baseline vs augmented comparison that preserves the same untouched real validation slice.

### Single Source Of Truth
- Historical row source: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\02_replay_labeled_samples.csv`
- Mining rules source of truth: `D:\SM\scripts\build_nikkei_replay_samples.py`
- Experiment artifact root: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\07_real_failure_event_experiment\`

### Evidence That Drives This Round
- `1D/3D` improved after switching to prototype-add mining, so the direction is correct.
- `5D` improved only partially and still underperformed baseline.
- `5D` untouched negatives show a different shape from `1D/3D`:
  - higher existing position,
  - closer to resistance or breakout exhaustion,
  - slower multi-day failure,
  - not just immediate next-day reversal.

### Inputs
- Required fields:
  - `sample_id`
  - `sample_source`
  - `signal_date`
  - `signal_family`
  - `signal_direction`
  - `candidate_event_type`
  - `candidate_action_label`
  - `base_position_v3`
  - `dist_res20`
  - `dist_sup20`
  - `dist_sup60`
  - `weighted_vol_down`
  - `component_above200_breadth`
  - `avg_component_vr`
  - `horizon_1d_close_return`
  - `horizon_3d_close_return`
  - `horizon_5d_close_return`
  - `horizon_1d_max_drawdown`
  - `horizon_3d_max_drawdown`
  - `horizon_5d_max_drawdown`

### Mining Rules
- This round still mines `add` only.
- When `label_horizon in {1d, 3d}`:
  - keep the current prototype-add rule unchanged.
- When `label_horizon = 5d`:
  - apply a dedicated slow-fail add prototype rule.

### 5D Slow-Fail Prototype Rule
- Candidate rows may come only from governed `historical_research` rows.
- Candidate rows must satisfy:
  - `signal_direction = add`
  - `signal_family in {daily_position, breakout_followthrough}`
  - `base_position_v3 >= 0.30`
  - `dist_sup20 >= 0.035`
  - `avg_component_vr >= 0.80`
- Then at least one of the following subcontexts must hold:
  - `resistance_exhaustion_5d`
    - `dist_res20 >= -0.025`
    - `component_above200_breadth >= 0.90`
    - optional explicit event labels may be blank or in:
      - `candidate_action_label in {"", "resistance_reject_watch", "false_breakout_avoid_chase", "uncertain_breakout_wait", "resistance_break_watch"}`
      - `candidate_event_type in {"", "near_resistance_20d", "breakout_20d", "breakout_60d"}`
  - `extended_add_drift_5d`
    - `dist_res20 <= -0.045`
    - `component_above200_breadth >= 0.55`
    - `avg_component_vr >= 0.80`
    - blank explicit event fields are allowed
- The builder remains horizon-specific:
  - output rows must be retained only if `continuation_label_5d = 0`
- Mined rows must derive replay labels under the mined `add` direction and remain negative continuation outcomes for the requested horizon.

### Output Row Contract
- Each mined row must expose:
  - `sample_source=real_failure_event_mining`
  - `source_sample_id`
  - `is_real_failure_event=True`
  - `failure_label_horizon`
  - `mined_action_direction=add`
  - `mined_failure_reason`
- For `5D` rows, `mined_failure_reason` must be one of:
  - `prototype_add_failure_5d_resistance_exhaustion`
  - `prototype_add_failure_5d_extended_drift`

### Validation Boundary
- Mined rows must never enter validation.
- Baseline and augmented runs must use the same real validation rows.
- Mined rows may augment the train split only.

### Rejection Conditions
- Reject any implementation that changes `1D/3D` behavior in this round.
- Reject any implementation that still uses one shared add-prototype rule for `5D`.
- Reject any implementation that reintroduces `reduce` mining.
- Reject any experiment where mined rows leak into validation.

### Traceability Requirements
- The mined failure pool must be exportable as a standalone CSV.
- The comparison summary must include:
  - baseline train count
  - augmented train count
  - mined failure train count
  - validation accuracy
  - validation balanced accuracy
  - negative recall
- The summary must explicitly state that the `5D` slow-fail lane remains experimental.

## Decision
- Chosen approach: Scheme A, specialize the mining rule only for `5D` and leave `1D/3D` unchanged.
- Rejected alternative 1: split all horizons into `fast-fail` and `slow-fail` in one round.
  - Reason: the weakest gap is `5D`, and a single-horizon intervention gives the cleanest attribution.
- Rejected alternative 2: expand the current shared prototype thresholds.
  - Reason: that would blur the distinction that the evidence already exposed.
- Known tradeoffs:
  - the `5D` pool may become smaller;
  - fixed thresholds may still underfit some `5D` subtypes;
  - if the actual `5D` failures contain more than two subcontexts, this round may still be incomplete.

## Acceptance

### Pre-Implementation Gate
- `1D/3D` frozen behavior is explicit.
- `5D` dedicated rule is explicit.
- `5D` mined failure reasons are explicit.
- Validation isolation rule is explicit.

### Pre-Completion Gate
- There is a tested `5D`-specific builder contract.
- There is a tested experiment runner that keeps validation real-only.
- There are refreshed experiment artifacts for `1d / 3d / 5d`.
- There is an updated summary comparing baseline vs the `5D` slow-fail augmentation.

### Verification Standard
- Required commands before claiming completion:
  - `python D:\SM\scripts\test_nikkei_real_failure_event_balance.py`
  - `python D:\SM\scripts\test_nikkei_continuation_head.py`
  - `python D:\SM\scripts\test_nikkei_replay_classifier.py`
  - `python D:\SM\scripts\test_run_nikkei_hgb_rf_daily_workflow.py`

## Cross-Artifact Contract
- Triggered: no
- Reason: this slice stays inside Python research scripts, experiment artifacts, and docs. No Rust public boundary changes are involved.

## Independent Risk Pass
- Mode: inline-fresh-pass
- Trigger: this round can drift into silent multi-horizon behavior changes unless `1D/3D` freeze and `5D` specialization are both explicit.
- Fresh-context question: what must be true for this round to prove anything about `5D` rather than just another global threshold rewrite?
- Findings:
  - `1D/3D` must remain unchanged;
  - `5D` must use a dedicated rule;
  - the two `5D` subcontexts must be separately traceable in `mined_failure_reason`.
- Blocking gaps: none.

## Next Skill
- `writing-plans` should produce the implementation breakdown for this slice.
