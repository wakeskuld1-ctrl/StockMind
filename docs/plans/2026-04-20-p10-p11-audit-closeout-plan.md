# P10 P11 Audit Closeout Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Produce the minimum governed audit-closeout package for the completed P10/P11 phase without expanding into a richer semantic audit or P12 work.

**Architecture:** Keep the route documentation-first and truth-driven. Reuse the already generated `graphify-out/` structural audit, the repository acceptance checklist, and the current handoff truth files to build one explicit closeout document that maps approved phase scope to evidence, residual limits, and non-goals. Update governance or handoff files only where they still describe a pre-graph-audit world or would otherwise mislead the next worker.

**Tech Stack:** Markdown governance docs, handoff docs, existing Graphify audit artifacts, Cargo verification record.

---

### Task 1: Freeze The Audit-Closeout Contract

**Files:**
- Create: `E:/SM/docs/plans/2026-04-20-p10-p11-audit-closeout-plan.md`
- Read: `E:/SM/docs/handoff/CURRENT_STATUS.md`
- Read: `E:/SM/docs/handoff/HANDOFF_ISSUES.md`
- Read: `E:/SM/docs/architecture/stockmind-acceptance-checklist.md`
- Read: `E:/SM/graphify-out/GRAPH_REPORT.md`

**Step 1: Reconfirm the current truth inputs**

Run:

```powershell
Get-Content E:/SM/docs/handoff/CURRENT_STATUS.md
Get-Content E:/SM/docs/handoff/HANDOFF_ISSUES.md
Get-Content E:/SM/docs/architecture/stockmind-acceptance-checklist.md
Get-Content E:/SM/graphify-out/GRAPH_REPORT.md
```

Expected:
- current branch is already full-green
- `graphify-out/` exists
- graph audit limit is explicitly AST-only

**Step 2: Lock the approved scope**

- include only `P10/P11` phase closeout evidence
- exclude richer semantic graph extraction
- exclude all `P12` design or implementation work
- exclude runtime behavior changes

### Task 2: Write The Closeout Document

**Files:**
- Create: `E:/SM/docs/handoff/P10_P11_AUDIT_CLOSEOUT.md`

**Step 1: Write the document skeleton**

Include these sections:

- purpose and phase boundary
- approved scope / explicit non-goals
- evidence inventory
- acceptance mapping
- graph audit interpretation
- residual limits
- recommended next actions

**Step 2: Map each accepted area to concrete proof**

- structure acceptance -> source-guard tests
- formal mainline acceptance -> formal chain tests
- repository acceptance -> `cargo check`, `cargo test -- --nocapture`
- graph audit support -> `graphify-out/GRAPH_REPORT.md`, `graphify-out/graph.json`, `graphify-out/graph.html`

**Step 3: Record residual limits honestly**

- graph audit is AST-only
- historical handoff docs remain background only
- this document is a phase closeout artifact, not a standing replacement for current status truth

### Task 3: Align Governance And Handoff Truth

**Files:**
- Modify: `E:/SM/docs/handoff/README.md`
- Modify: `E:/SM/docs/handoff/CURRENT_STATUS.md`
- Modify: `E:/SM/docs/governance/decision_log.md`
- Modify: `E:/SM/docs/governance/contract_registry.md`

**Step 1: Add the new closeout document to handoff discoverability**

- list the document in `docs/handoff/README.md`
- mention it in `CURRENT_STATUS.md` only as a supporting closeout artifact, not as current-status truth replacement

**Step 2: Remove stale “graph audit missing” governance wording**

- update `decision_log.md` assumptions / open questions if they still imply `graphify-out/` is absent
- update `contract_registry.md` known limits so it points to the current AST-only audit reality

### Task 4: Journal And Verify The Closeout Slice

**Files:**
- Modify: `E:/SM/CHANGELOG_TASK.MD`

**Step 1: Run the smallest honest verification set**

Run:

```powershell
cargo test --test security_analysis_fullstack_cli -- --nocapture
cargo test -- --nocapture
```

Expected:
- PASS
- no new blocker introduced by the audit-closeout slice

**Step 2: Append the task journal entry**

- record the new audit closeout document
- record governance and handoff truth alignment
- record the exact verification commands

**Step 3: Re-check final touched files**

Run:

```powershell
git status --short -- docs/handoff docs/governance docs/plans/2026-04-20-p10-p11-audit-closeout-plan.md CHANGELOG_TASK.MD tests/security_analysis_fullstack_cli.rs
```

Expected:
- only the intended closeout-supporting docs plus the already-approved fullstack test file remain touched for this slice
