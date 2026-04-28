# Nikkei Prediction Enhancement Replay Design

## Intent

### Goal

Build the first prediction-enhancement line as a `post-signal replay classifier` for the Nikkei ETF workflow, so the system can judge whether a governed add/reduce/hold-style signal was timely, premature, late, or acceptable after the signal has played out.

### Scope

- Use existing Nikkei governed signal artifacts and adjustment-point research exports as the primary historical sample source.
- Use the live Nikkei journal as the truth source for real executed trades and later incremental replay evidence.
- Focus on replay labels anchored to signal events rather than generic future weekly direction.
- Define a follow-on interface so a future `continuation head` can reuse the same event sample base.

### Non-goals

- This design does not replace the current HGB / RF position-adjustment line.
- This design does not train a new model yet.
- This design does not expand to A-shares or non-Nikkei universes yet.
- This design does not try to solve pure “future 1w up/down” forecasting.

### Success Definition

The design is successful if the next implementation can answer these questions without ambiguity:

1. what counts as one replay sample;
2. which fields are frozen at signal time;
3. how `1D / 3D / 5D` replay labels are assigned;
4. how real journal rows and historical synthetic signal events are merged;
5. how to judge whether this line is useful enough to later support a continuation model.

### Delivery Form

- This design document
- Follow-on implementation plan after approval

## Contract

### Why This Line Exists

The current weak point is not simply “volume features are missing.” The larger issue is that the main supervised target has mostly been framed as generic future direction, for example `positive_return_1w`. That target does not align tightly enough with the real trading question, which is:

- after a signal is issued, was the action quality good or poor?
- did the reduction avoid risk, or did it cut a clean continuation too early?
- did the add signal catch a stable breakout, or did it buy a false move?

This means the system needs an event-anchored replay layer before trying to add more direction features.

### Core Object

The core object is one `signal_event_replay_sample`.

Each sample represents one governed signal event anchored at `T0`, with features frozen at or before `T0`, and replay labels evaluated over later horizons.

### Sample Source Of Truth

Historical event samples should come from existing Nikkei research artifacts, primarily:

- `artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/adjustment_point_analysis/04_adjustment_event_candidates.csv`
- `artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/adjustment_point_analysis/24_downside_reduction_logic_samples.csv`
- `artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/adjustment_point_analysis/55_v3_adjustment_model_dataset.csv`
- `artifacts/02_live_like_backtest_full_snapshot/05_live_like_rule_audit.csv`
- `artifacts/02_live_like_backtest_full_snapshot/09_no_deadband_decision_audit.csv`

Real executed replay truth should come from:

- `D:\SM\docs\trading-journal\nikkei\journal.csv`
- `D:\SM\docs\trading-journal\nikkei\journal.md`
- dated replay notes under `D:\SM\docs\trading-journal\nikkei\`

If historical synthetic events and real journal rows disagree, they must not be silently merged as equivalent truth. Historical rows are backfilled research samples; journal rows are real execution truth.

### Required Input Fields

Each replay sample must carry these fields at minimum:

- `signal_date`
- `effective_signal_date` when different from request date
- `signal_family`
- `signal_direction`
- `signal_strength`
- `action_type`
- `base_position_v3`
- `target_position_proxy`
- `rating_state`
- `primary_model_id`
- `secondary_model_id`
- `primary_adjustment`
- `secondary_adjustment`
- price-structure fields at signal time
- breadth fields at signal time
- volume and volume-down fields at signal time
- ETF execution context when available

### Recommended Event Families

The first implementation should standardize events into a small governed vocabulary:

- `reduce_risk`
- `add_risk`
- `hold_watch`
- `support_test`
- `breakout_followthrough`
- `breakdown_followthrough`

### Feature Groups

The replay classifier should reuse existing fields before inventing new ones.

#### Price structure

- distance to 20D resistance
- distance to 20D support
- distance to 60D support
- distance to 200D average
- short-horizon returns
- close position / drawdown / rebound type fields

#### Breadth

- weighted breakout breadth
- component above-200 breadth
- component breakdown counts

#### Volume and volume-price

- average component volume ratio
- weighted volume-down breadth
- volume confirmation fields
- medium / long window accumulation fields when available

#### Execution context

- ETF symbol
- sell / buy execution price
- ETF premium or discount snapshot if available
- whether execution matched planned target

### Replay Horizons

The first version must only use:

- `1D`
- `3D`
- `5D`

`10D` may be added later, but it must not be in v1 because:

- the live journal contract already centers on `1D / 3D / 5D`;
- a longer horizon blurs whether the signal quality problem is timing or regime drift.

### Label Contract

The first version should classify replay outcomes with a compact label set:

- `correct`
- `acceptable`
- `premature`
- `late`
- `inconclusive`

The final stored label should be paired with polarity:

- `correct_reduce`
- `acceptable_reduce`
- `premature_reduce`
- `late_reduce`
- `correct_add`
- `acceptable_add`
- `premature_add`
- `late_add`
- `inconclusive`

### Label Semantics

#### Reduce-family signals

- `correct_reduce`: post-signal risk releases or upside follow-through is weak enough that keeping near-full size was a poor trade
- `acceptable_reduce`: some upside continues, but not enough to invalidate the risk-compression choice
- `premature_reduce`: strong continuation follows with repaired structure and little near-term drawdown
- `late_reduce`: material weakness had already arrived before the governed reduction

#### Add-family signals

- `correct_add`: breakout or support-rebound follow-through remains intact through the replay horizon
- `acceptable_add`: some noise occurs, but the action still improves exposure quality
- `premature_add`: the move quickly fails or falls back below the event anchor
- `late_add`: much of the favorable move was already gone before the signal

### Required Derived Fields

For each replay horizon, the implementation must be able to derive:

- close-to-close return
- maximum favorable excursion
- maximum adverse excursion
- whether key structure was held or lost
- whether the next governed signal confirmed or reversed the prior action

### Rejection Conditions

Reject the sample as invalid if:

- the features use data after `T0`;
- the replay horizon is missing required market data;
- the sample mixes synthetic research events and real journal execution without a source flag;
- the label cannot distinguish action polarity;
- the design falls back to generic future `1w` up/down as the label.

## Decision

### Chosen Approach

Adopt `Scheme B`:

1. first build the `post-signal replay classifier`;
2. stabilize the event sample base and replay labels;
3. only then attach a future `continuation head` on the same event base.

### Why This Choice Was Made

This sequence fits the current problem better than directly adding another direction model:

- the current direction target is weak and near-random in walk-forward use;
- the replay line is closer to the actual trading question;
- the real journal and historical backtest assets already support event-based replay much better than another generic weekly direction task;
- replay labels can later supervise the continuation head instead of guessing what “good signal quality” means.

### Rejected Alternative

Rejected for now: build `continuation head` and `replay classifier` together from day one.

Reason:

- both lines can share the same sample base later, but the replay contract must be stabilized first;
- if both are introduced immediately, the team may blur “predicting future path” with “grading signal quality” and create label drift.

### Practical Meaning In Plain Language

The system should first learn to answer:

- “Was this reduction too early or basically right?”
- “Was this add signal buying a stable move or a fake breakout?”

Only after that should it try to answer:

- “Will this signal probably continue for the next `1D / 3D / 5D`?”

### Known Tradeoffs

- Replay labels are more aligned with trading quality, but they require more explicit rule design than raw return labels.
- Historical synthetic samples provide scale, but they are not the same as real broker execution.
- Real journal rows provide the highest truth quality, but sample count is initially small.

### Open Questions

- whether the first implementation should train one unified replay classifier with polarity fields, or separate add/reduce classifiers;
- whether `hold_watch` should be trained in v1 or only recorded as a neutral event family;
- how much ETF premium data is available historically for event alignment.

## Acceptance

### Before Implementation Starts

The following must be explicit:

- one frozen event sample schema;
- one replay label schema for `1D / 3D / 5D`;
- one source flag that separates research synthetic events from real journal execution;
- one rule for handling `effective_signal_date` versus requested date.

### Before Claiming Completion

The first implementation must produce:

- an event sample dataset;
- derived replay labels for `1D / 3D / 5D`;
- a sample-count summary by signal family and label;
- a basic model evaluation table;
- an error-analysis table showing where signals are most often `premature` or `late`.

### Verification Standard

The implementation will be considered useful only if it demonstrates at least one of the following:

- replay labels show better stability than the current generic `1w` direction target;
- the model can clearly separate common `premature_reduce` versus `correct_reduce` situations;
- the replay output explains recent live journal signals more coherently than the generic direction model.

### What Must Be Rejected Instead Of Guessed

- Do not guess replay labels from narrative alone.
- Do not backfill post-signal features into pre-signal inputs.
- Do not claim the problem is solved by “adding more volume indicators” unless the event-anchored labels improve.

## Implementation Direction

The next implementation plan should likely use three stages:

1. build the event sample base from research artifacts plus journal rows;
2. derive replay labels for `1D / 3D / 5D`;
3. train and evaluate the first replay classifier, then compare it against the current governed signal behavior.

Only after that should the team decide whether to:

- keep improving replay classification, or
- attach a `continuation head` using the same event sample base.
