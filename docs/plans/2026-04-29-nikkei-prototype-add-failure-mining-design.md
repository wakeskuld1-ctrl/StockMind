# Nikkei Prototype Add Failure Mining Design

## Intent
- Goal: align continuation negative augmentation with the dominant untouched-validation negative shape instead of broad event-style failure semantics.
- Scope: mine only prototype-driven `add` failure rows that resemble real `premature_add` negatives in untouched validation, run a train-only augmentation experiment on the same frozen validation slice, and compare whether balance-aware continuation metrics improve.
- Non-goals: do not reintroduce `reduce` failure mining in this round, do not merge mined rows into governed training by default, do not change Rust Tool boundaries, and do not wire this lane into the daily operator.
- Success definition: the project can export a separately tagged prototype-add failure pool and show whether this narrower pool improves `balanced_accuracy` or negative recall on real validation.
- Delivery form: one design doc, one implementation plan, updated Python tests/scripts, refreshed experiment artifacts, and one short summary update.

## Contract

### Core Objects
- `prototype_add_failure_row`: one separately tagged row mined from governed historical rows that matches the dominant untouched-validation negative shape.
- `prototype_add_failure_pool`: the collection of mined prototype-add failure rows used only as train-time augmentation.
- `prototype_add_failure_experiment`: one baseline vs augmented comparison that preserves the same untouched real validation slice.

### Single Source Of Truth
- Historical row source: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\02_replay_labeled_samples.csv`
- Prototype-add failure mining rules source of truth: `D:\SM\scripts\build_nikkei_replay_samples.py`
- Experiment artifact root: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\07_real_failure_event_experiment\`

### Evidence That Drives This Round
- Untouched validation negatives are dominated by `premature_add`, not by `reduce` failures.
- Most untouched validation negatives are `signal_family=daily_position`.
- Most untouched validation negatives have no explicit `candidate_event_type` or `candidate_action_label`.
- The dominant failure shape is "already elevated position, still adding near resistance, then no continuation."

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
- This round mines `add` only.
- Candidate rows may come only from governed `historical_research` rows.
- Candidate rows must satisfy the prototype-driven add context:
  - `signal_direction = add`
  - `signal_family in {daily_position, breakout_followthrough}`
  - `base_position_v3 >= 0.18`
  - `dist_res20 <= 0.02`
  - `dist_sup20 >= 0.02`
  - `avg_component_vr >= 0.74`
- Explicit event fields are optional in this round:
  - blank `candidate_action_label` and blank `candidate_event_type` are allowed
  - if explicit labels exist, only mild add-failure style labels may pass:
    - `candidate_action_label in {resistance_reject_watch, false_breakout_avoid_chase, uncertain_breakout_wait, resistance_break_watch}`
    - `candidate_event_type in {near_resistance_20d, breakout_20d, breakout_60d}`
- Rows dominated by support-failure / downside-defense semantics must be rejected in this round.
- The builder must be horizon-specific:
  - input includes `label_horizon in {1d, 3d, 5d}`
  - output rows must be retained only if `continuation_label_<label_horizon> = 0`
- Mined rows must derive replay labels under the mined `add` direction and must remain negative continuation outcomes for the requested horizon.

### Output Row Contract
- Each mined row must expose:
  - `sample_source=real_failure_event_mining`
  - `source_sample_id`
  - `is_real_failure_event=True`
  - `failure_label_horizon`
  - `mined_action_direction=add`
  - `mined_failure_reason=prototype_add_failure`
  - replay labels
  - continuation labels

### Validation Boundary
- Mined rows must never enter validation.
- Baseline and augmented runs must use the same real validation rows.
- Mined rows may augment the train split only.

### Rejection Conditions
- Reject any implementation that still mines `reduce` rows in this round.
- Reject any implementation that prefers broad support / breakdown event semantics over prototype add-failure shape.
- Reject any experiment where mined rows leak into validation.
- Reject any summary that headlines raw accuracy without balance-aware metrics.

### Traceability Requirements
- The mined prototype-add failure pool must be exportable as a standalone CSV.
- The comparison summary must include:
  - baseline train count
  - augmented train count
  - mined failure train count
  - validation accuracy
  - validation balanced accuracy
  - negative recall
- The summary must explicitly state that this round is still experimental.

## Decision
- Chosen approach: Scheme A, mine only prototype-driven `add` failures that resemble untouched-validation `premature_add` negatives.
- Rejected alternative 1: keep mixed `add + reduce` failure mining.
  - Reason: untouched validation negatives are dominated by `add` failures, so mixed mining dilutes the target shape.
- Rejected alternative 2: keep explicit event-driven failure rules as the main filter.
  - Reason: the dominant untouched negatives are mostly `daily_position` rows without explicit event labels.
- Known tradeoffs:
  - the pool will become smaller;
  - the rules are intentionally narrow and may miss some genuine failures;
  - the result may still fail if the dominant negative shape is even more local than this prototype.

## Acceptance

### Pre-Implementation Gate
- Prototype-add context rules are explicit.
- `add`-only boundary is explicit.
- Horizon-specific negative-only retention is explicit.
- Validation isolation rule is explicit.

### Pre-Completion Gate
- There is a tested prototype-add builder contract.
- There is a tested experiment runner that keeps validation real-only.
- There are refreshed experiment artifacts for `1d / 3d / 5d`.
- There is an updated summary comparing baseline vs prototype-add augmentation.

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
- Trigger: this round can still drift back into broad event mining unless the prototype shape is frozen explicitly.
- Fresh-context question: what must be true for this round to test untouched-validation negative shape alignment rather than another broad augmentation pass?
- Findings:
  - mined rows must stay `add` only;
  - prototype-driven `daily_position` style rows must be allowed even when explicit event labels are blank;
  - `reduce` and support-failure semantics must be excluded from this round.
- Blocking gaps: none.

## Next Skill
- `writing-plans` should produce the implementation breakdown for this slice.
