# Nikkei Simulated Action Balance Design

## Intent
- Goal: improve sparse Nikkei replay and continuation negative classes by mining real historical event rows and generating separately tagged simulated add/reduce action samples.
- Scope: build one isolated augmentation lane for `1D / 3D / 5D`, export sample-distribution tables, and compare continuation-head results before and after augmentation on the same untouched real validation slice.
- Non-goals: do not rewrite the main replay label contract, do not merge simulated samples into the official replay export by default, do not wire augmentation into the daily operator, and do not change Rust Tool boundaries.
- Success definition: the project can generate governed simulated action samples from historical event rows, keep them isolated from validation, and export a before/after comparison focused on balance-aware metrics.
- Delivery form: one design doc, one implementation plan, Python scripts/tests, experiment artifacts, and one short research summary update.

## Contract

### Core Objects
- `real_event_row`: one existing historical replay row from the governed Nikkei research chain.
- `simulated_action_sample_row`: one separately tagged sample derived from a real historical event row, with simulated `add` or `reduce` action semantics and labels computed from the same forward outcomes.
- `augmentation_experiment`: one experiment that compares baseline continuation training against augmentation-enhanced training while preserving the same real validation slice.

### Single Source Of Truth
- Real historical row source: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\02_replay_labeled_samples.csv`
- Simulated action rules source of truth: `D:\SM\scripts\build_nikkei_replay_samples.py`
- Experiment artifact root: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\06_simulated_action_balance_experiment\`

### Inputs
- Required real fields:
  - `sample_id`
  - `signal_date`
  - `sample_source`
  - `signal_family`
  - `candidate_event_type`
  - `candidate_action_label`
  - `downside_suggested_action`
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
  - `next_signal_primary_adjustment`
- Required simulated outputs:
  - `sample_id`
  - `sample_source`
  - `simulated_action_direction`
  - `simulated_action_reason`
  - `source_sample_id`
  - `is_simulated_action`
  - replay labels derived under the simulated action direction
  - continuation labels derived from those replay labels

### Simulated Action Rules
- Simulated `add` candidates must come only from real historical rows with breakout / resistance / support-rebound style context, for example:
  - `signal_family in {breakout_followthrough, support_test}`
  - or `candidate_event_type in {breakout_20d, breakout_60d, near_resistance_20d}`
- Simulated `reduce` candidates must come only from real historical rows with breakdown / downside / support-failure style context, for example:
  - `signal_family in {breakdown_followthrough, support_test}`
  - or `candidate_event_type in {breakdown_20d, breakdown_60d, near_support_20d}`
  - or `downside_suggested_action in {tighten_risk, reduce_partial_or_wait_reclaim, reduce_or_avoid}`
- Each simulated row must preserve the real market context and forward outcomes, but overwrite only the action semantics needed for counterfactual label derivation.

### Validation Boundary
- Simulated rows must never enter the official baseline validation slice.
- Baseline and augmented runs must use the same real validation rows.
- Simulated rows may augment the training split only.

### Outputs
- Simulated sample export
- Label distribution tables before and after augmentation
- One comparison summary with:
  - baseline sample counts
  - augmented training sample counts
  - per-horizon validation accuracy
  - per-horizon balanced accuracy
  - per-horizon negative-class recall when available

### Rejection Conditions
- Reject any implementation that silently mixes simulated rows into the official replay export used as source of truth.
- Reject any comparison where simulated rows appear in validation.
- Reject any implementation that labels simulated rows with generic weekly direction instead of the governed replay/continuation rules.
- Reject any implementation that cannot trace every simulated row back to a real `source_sample_id`.

### Traceability Requirements
- Simulated rows must expose:
  - `source_sample_id`
  - `sample_source=simulated_action_replay`
  - `is_simulated_action=True`
  - `simulated_action_direction`
  - `simulated_action_reason`
- The experiment summary must explicitly state whether improvements are measured on real validation only.

## Decision
- Chosen approach: Scheme A, use real historical event rows as anchors, create separately tagged simulated action samples, and run augmentation as a train-only experiment.
- Rejected alternative 1: directly merge simulated rows into the official replay/continuation dataset.
  - Reason: too much truth pollution risk before we know whether the augmentation helps.
- Rejected alternative 2: do only class weighting / oversampling with no new event-derived samples.
  - Reason: too weak for the current imbalance problem and does not add missing action-quality structure.
- Known tradeoffs:
  - this approach is safer, but slower than directly merging synthetic rows;
  - the augmentation quality is bounded by the real event filters we choose;
  - if event filters are too broad, we will add noise instead of useful negatives.
- Open questions deferred:
  - whether successful simulated rows should later join the governed training base;
  - whether add and reduce need separate augmentation lanes;
  - whether source-specific feature leakage requires dropping `sample_source` in experiment-only training.

## Acceptance

### Pre-Implementation Gate
- Simulated row schema is explicit.
- Candidate-event filters for simulated add/reduce are explicit.
- Validation isolation rule is explicit.
- Comparison metric set includes `balanced_accuracy`.

### Pre-Completion Gate
- There is a tested simulated-sample builder.
- There is a tested comparison runner that keeps validation real-only.
- There are exported sample-distribution tables.
- There is an exported baseline vs augmented comparison summary.
- Fresh verification covers the new tests plus existing continuation/replay regressions.

### Verification Standard
- Required commands before claiming completion:
  - `python D:\SM\scripts\test_nikkei_simulated_action_balance.py`
  - `python D:\SM\scripts\test_nikkei_continuation_head.py`
  - `python D:\SM\scripts\test_nikkei_replay_classifier.py`
  - `python D:\SM\scripts\test_run_nikkei_hgb_rf_daily_workflow.py`

## Cross-Artifact Contract
- Triggered: no
- Reason: the slice stays inside Python research scripts, experiment artifacts, and docs. No Rust public boundary, registry, or source guard changes are introduced.

## Independent Risk Pass
- Mode: inline-fresh-pass
- Trigger: augmentation can create misleading gains if simulated rows leak into validation or overwrite source-of-truth datasets.
- Fresh-context question: what must stay frozen so the experiment remains informative instead of self-confirming?
- Findings:
  - validation isolation is the highest-risk boundary;
  - simulated rows must remain separately tagged and separately exportable;
  - comparison must prioritize balance-aware metrics over raw accuracy.
- Blocking gaps: none.

## Next Skill
- `writing-plans` should produce the task breakdown for implementation.
