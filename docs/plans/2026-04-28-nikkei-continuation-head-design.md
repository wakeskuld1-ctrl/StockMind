# Nikkei Continuation Head Design

## Intent
- Goal: build the first offline `continuation head` on top of the governed Nikkei replay sample base, so we can judge whether a signal is likely to continue cleanly over `1D / 3D / 5D`.
- Scope: reuse the existing replay sample builder and labeled sample export, derive continuation labels from replay outcomes, train an offline research-only model, and export auditable metrics and predictions.
- Non-goals: do not replace HGB/RF position sizing, do not change Rust Tool boundaries, do not wire continuation output into the daily operator, and do not redefine the live journal schema in v1.
- Success definition: the system can train one governed continuation classifier from the replay sample base and publish stable artifacts for `cont_1d`, `cont_3d`, and `cont_5d`.
- Delivery form: one design doc, one implementation plan, Python scripts, tests, artifact exports, and handoff updates.

## Contract

### Core Objects
- `continuation_sample_row`: one replay sample row with frozen signal-time features and replay-derived continuation supervision.
- `continuation_label_horizon`: one of `1d`, `3d`, `5d`.
- `continuation_target`: binary label where `1` means usable continuation and `0` means weak or invalid continuation.

### Single Source Of Truth
- Sample base source of truth: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\04_replay_classifier_full_snapshot\02_replay_labeled_samples.csv`
- Replay label vocabulary source of truth: `D:\SM\scripts\build_nikkei_replay_samples.py`
- Continuation training artifact root: `D:\SM\docs\research\nikkei-etf-hgb-rf-v3-20260427\artifacts\05_continuation_head_full_snapshot\`

### Inputs
- Required sample fields:
  - `sample_id`
  - `signal_date`
  - `sample_source`
  - `signal_family`
  - `signal_direction`
  - `action_type`
  - `base_position_v3`
  - `rating_state`
  - `dist_res20`
  - `dist_sup20`
  - `dist_sup60`
  - `weighted_vol_down`
  - `component_above200_breadth`
  - `avg_component_vr`
  - `horizon_1d_close_return`
  - `horizon_3d_close_return`
  - `horizon_5d_close_return`
  - `replay_label_1d`
  - `replay_label_3d`
  - `replay_label_5d`
- Optional enrichments already present in the replay sample may be reused, but v1 must not require new upstream schema.

### Outputs
- Labeled continuation sample export with:
  - `continuation_label_1d`
  - `continuation_label_3d`
  - `continuation_label_5d`
- Per-horizon training outputs:
  - metrics CSV
  - predictions CSV
  - confusion matrix CSV
  - label counts CSV
  - machine-readable training summary JSON

### Label Mapping
- Positive continuation (`1`):
  - `correct_add`
  - `acceptable_add`
  - `correct_reduce`
  - `acceptable_reduce`
- Negative continuation (`0`):
  - `premature_add`
  - `late_add`
  - `premature_reduce`
  - `late_reduce`
- Excluded from supervision:
  - `inconclusive`

### State Boundaries
- v1 is offline research only.
- v1 must reuse replay rows instead of building a second independent dataset.
- v1 may publish probabilities but must not claim live execution authority.
- v1 must keep `cont_1d`, `cont_3d`, `cont_5d` as derived research outputs, not operator decisions.

### Rejection Conditions
- Reject implementation that reads generic `1w` direction labels instead of replay labels.
- Reject implementation that silently includes `inconclusive` rows in supervised training.
- Reject implementation that changes replay label semantics inside continuation code.
- Reject implementation that adds Rust entrypoints, daily operator writes, or journal schema changes in this slice.

### Traceability Requirements
- Every exported summary must declare:
  - target definition version
  - continuation label horizon
  - positive and negative replay label mapping
  - sample counts after `inconclusive` exclusion
- Tests must prove the mapping and artifact contract explicitly.

## Decision
- Chosen approach: Scheme A, one unified binary continuation head, trained separately for `1d`, `3d`, and `5d` using the same replay sample base and one shared feature contract.
- Rejected alternative 1: split add/reduce into separate continuation models.
  - Reason: semantics are cleaner, but current sample count is too thin for a stable first pass.
- Rejected alternative 2: skip continuation and keep replay only.
  - Reason: replay tells us whether the action quality was right, but it does not separate clean continuation from noisy continuation within the same family strongly enough for the next research stage.
- Known tradeoffs:
  - one unified head is simpler and denser, but polarity-specific nuance may be diluted;
  - binary mapping is easier to stabilize, but it compresses some intermediate structure;
  - offline-only keeps risk low, but it postpones operator integration questions.
- Open questions deferred after v1:
  - whether add/reduce should be split in v2;
  - whether continuation should later consume replay probabilities as extra features;
  - whether continuation should become a second-stage gate in the daily workflow.

## Acceptance

### Pre-Implementation Gate
- A frozen continuation label mapping exists for `1d / 3d / 5d`.
- The replay sample file named above is confirmed as the only v1 training base.
- The output artifact root and filenames are explicit.
- TDD scope is explicit: mapping tests first, trainer smoke tests second.

### Pre-Completion Gate
- The project contains:
  - a continuation-label derivation path,
  - a continuation training entrypoint,
  - tests that fail before implementation and pass after implementation,
  - exported artifacts for `1d / 3d / 5d`,
  - updated handoff/research documentation.
- Fresh verification must include continuation tests plus existing Nikkei replay/daily workflow regressions.

### Verification Standard
- Required commands before claiming completion:
  - `python D:\SM\scripts\test_nikkei_continuation_head.py`
  - `python D:\SM\scripts\test_nikkei_replay_classifier.py`
  - `python D:\SM\scripts\test_run_nikkei_hgb_rf_daily_workflow.py`
  - `python C:\Users\wakes\.codex\skills\nikkei-live-journal\scripts\test_upsert_journal.py`

## Cross-Artifact Contract
- Triggered: no
- Reason: this slice does not add or remove Rust public boundaries, registries, manifests, or frozen source guards. The sync surface is limited to Python research scripts, research artifacts, tests, and handoff docs.

## Independent Risk Pass
- Mode: user-approved-subagent
- Trigger: sample/training/doc sync can drift even without public boundary changes.
- Fresh-context question: if continuation reuses replay rows, what is the smallest safe mapping and what docs will become stale immediately after implementation?
- Findings:
  - replay-labeled sample export already contains the minimum fields needed for v1;
  - the main drift risk is doc status still saying continuation is future work;
  - `inconclusive` rows are numerous and must be excluded from supervised training.
- Blocking gaps: none for v1.

## Next Skill
- `writing-plans` is complete for this slice through the paired implementation plan.
- The next execution step is `test-driven-development`.
