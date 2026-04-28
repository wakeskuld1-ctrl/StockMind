# Nikkei Real Failure Event Mining Design

## Intent
- Goal: improve sparse Nikkei continuation negative classes by mining real historical failure events only, without broad simulated-action augmentation.
- Scope: derive a separately tagged real-failure event pool from governed historical event-study fields, run a train-only augmentation experiment, and compare baseline vs mined-failure performance on the same untouched real validation slice.
- Non-goals: do not merge mined failure rows into the official replay export by default, do not reintroduce broad simulated add/reduce rows, do not modify Rust Tool boundaries, and do not wire failure-event mining into the daily operator.
- Success definition: the project can mine a governed real-failure event pool, export before/after distribution tables, and show whether balance-aware continuation metrics improve on real validation.
- Delivery form: one design doc, one implementation plan, Python scripts/tests, experiment artifacts, and one short summary update.

## Contract

### Core Objects
- `real_failure_event_row`: one separately tagged row mined from a governed historical event-study context that already carries failure semantics in the real research artifacts.
- `failure_event_pool`: the collection of mined real-failure rows used only as train-time augmentation.
- `failure_event_experiment`: one baseline vs augmented comparison that preserves the same real validation slice.

### Single Source Of Truth
- Historical row source: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\02_replay_labeled_samples.csv`
- Failure-event mining rules source of truth: `D:\SM\scripts\build_nikkei_replay_samples.py`
- Experiment artifact root: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\07_real_failure_event_experiment\`

### Inputs
- Required fields:
  - `sample_id`
  - `sample_source`
  - `signal_date`
  - `signal_family`
  - `candidate_event_type`
  - `candidate_action_label`
  - `candidate_stood_1d`
  - `candidate_stood_3d`
  - `candidate_stood_5d`
  - `downside_suggested_action`
  - `horizon_1d_close_return`
  - `horizon_3d_close_return`
  - `horizon_5d_close_return`
  - `horizon_1d_max_drawdown`
  - `horizon_3d_max_drawdown`
  - `horizon_5d_max_drawdown`
  - `next_signal_primary_adjustment`

### Mining Rules
- Add-failure candidate contexts may come only from real breakout / resistance failure semantics such as:
  - `candidate_action_label in {false_breakout_avoid_chase, resistance_reject_watch, support_fail_watch, uncertain_breakout_wait}`
  - or `candidate_event_type in {breakout_20d, breakout_60d, near_resistance_20d}` with `candidate_stood_<horizon> = False`
- Reduce-failure candidate contexts may come only from real downside / support / over-defensive semantics such as:
  - `downside_suggested_action in {tighten_risk, reduce_partial_or_wait_reclaim, reduce_or_avoid}`
  - `candidate_action_label in {false_breakdown_avoid_panic_sell, support_hold_watch}`
- The builder must be horizon-specific:
  - input includes `label_horizon in {1d, 3d, 5d}`
  - output rows must be retained only if `continuation_label_<label_horizon> = 0`
  - rows that are negative only on other horizons but positive on the requested horizon must be rejected
- Mined rows must derive replay labels under the mined direction (`add` or `reduce`) and must be retained only if they remain negative continuation outcomes for the requested horizon, not positive continuation outcomes.

### Output Row Contract
- Each mined row must expose:
  - `sample_source=real_failure_event_mining`
  - `source_sample_id`
  - `is_real_failure_event=True`
  - `failure_label_horizon`
  - `mined_action_direction`
  - `mined_failure_reason`
  - replay labels
  - continuation labels

### Validation Boundary
- Mined failure rows must never enter validation.
- Baseline and augmented runs must use the same real validation rows.
- Mined rows may augment the train split only.
- The runner must call the builder with the same `label_horizon` used for training and validation.

### Rejection Conditions
- Reject any implementation that includes broad positive augmentation rows.
- Reject any mined row that cannot be traced back to a historical `source_sample_id`.
- Reject any experiment where mined rows leak into validation.
- Reject any experiment summary that headlines raw accuracy without balance-aware metrics.

### Traceability Requirements
- The mined failure pool must be exportable as a standalone CSV.
- The comparison summary must include:
  - baseline train count
  - augmented train count
  - mined failure train count
  - validation accuracy
  - validation balanced accuracy
  - negative recall
- The summary must explicitly state that mined rows remain experimental.

## Decision
- Chosen approach: Scheme A, mine only real failure events from governed event-study contexts and use a horizon-specific builder to produce train-only negative pools.
- Rejected alternative 1: broad simulated-action augmentation.
  - Reason: the first experiment proved it increases majority-class bias and harms balance-aware metrics.
- Rejected alternative 2: simple duplication of existing negative continuation rows.
  - Reason: that would increase quantity without improving event-shape diversity.
- Known tradeoffs:
  - this lane is safer and more interpretable, but sample growth may stay modest;
  - the quality of the pool depends on the failure-event rules being narrow enough;
  - if the event rules are too permissive, the same majority-bias problem will reappear.
- Open questions deferred:
  - whether add-failure and reduce-failure pools should later be trained separately;
  - whether some mined rows can later be promoted into governed training after additional review.

## Acceptance

### Pre-Implementation Gate
- Failure-event rules are explicit and narrow.
- Output row schema is explicit.
- Validation isolation rule is explicit.
- Balance-aware comparison metrics are explicit.

### Pre-Completion Gate
- There is a tested failure-event builder.
- There is a tested experiment runner that keeps validation real-only.
- There are exported failure-pool and distribution files.
- There is a summary comparing baseline vs mined-failure augmentation.

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
- Trigger: the experiment can still drift if failure-event rules accidentally reintroduce broad positive augmentation or validation leakage.
- Fresh-context question: what must be true for this experiment to measure real failure mining rather than another synthetic oversampling pass?
- Findings:
  - failure rows must stay negative-only after derived labeling;
  - the augmentation lane must remain separately tagged;
  - the real validation slice must remain frozen.
- Blocking gaps: none.

## Next Skill
- `writing-plans` should produce the implementation breakdown for this slice.
