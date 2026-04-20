# P10 P11 Tail Closeout Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close the remaining P10/P11 tail items by removing the last non-blocking warning, pruning stale absolute-path handoff drift, and generating the first repository-level graph audit.

**Architecture:** Keep the scope inside the already approved P10/P11 closeout boundary. First remove the known test-only warning and normalize handoff docs so current truth no longer depends on workspace-specific historical paths. Then run a repository-level graph audit and connect the resulting artifacts back into current-status and handoff issue tracking.

**Tech Stack:** Rust tests, Markdown handoff docs, Graphify CLI, Cargo test.

---

### Task 1: Remove The Last Non-Blocking Warning

**Files:**
- Modify: `E:/SM/tests/security_account_open_position_snapshot_cli.rs`

**Step 1: Confirm the helper is unused**

Run:

```powershell
rg -n "execution_request\(" E:/SM/tests/security_account_open_position_snapshot_cli.rs
```

Expected:
- only the helper definition is returned

**Step 2: Delete the unused helper with the smallest diff**

- remove the dead helper
- do not change test behavior

**Step 3: Run the focused test file**

Run:

```powershell
cargo test --test security_account_open_position_snapshot_cli -- --nocapture
```

Expected:
- PASS
- no `execution_request` unused warning

### Task 2: Prune Workspace-Specific Handoff Drift

**Files:**
- Modify: `E:/SM/docs/handoff/AI_HANDOFF.md`
- Modify: `E:/SM/docs/handoff/HANDOFF_ISSUES.md`

**Step 1: Find the stale absolute-path references**

Run:

```powershell
rg -n "D:\\SM|E:\\SM" E:/SM/docs/handoff/AI_HANDOFF.md E:/SM/docs/handoff/HANDOFF_ISSUES.md
```

Expected:
- `AI_HANDOFF.md` contains historical absolute-path references

**Step 2: Replace current guidance paths with portable repo-relative references**

- keep historical context only where necessary
- remove or rewrite stale `D:\SM` implementation guidance
- do not change the meaning of active governance rules

**Step 3: Re-read the edited handoff files**

Run:

```powershell
Get-Content -Encoding utf8 E:/SM/docs/handoff/AI_HANDOFF.md
Get-Content -Encoding utf8 E:/SM/docs/handoff/HANDOFF_ISSUES.md
```

Expected:
- current guidance is portable
- unresolved issues still truthfully describe remaining gaps

### Task 3: Generate The First Repository Graph Audit

**Files:**
- Create: `E:/SM/graphify-out/*`
- Modify: `E:/SM/docs/handoff/CURRENT_STATUS.md`
- Modify: `E:/SM/docs/handoff/HANDOFF_ISSUES.md`

**Step 1: Run Graphify on the repository root**

Run:

```powershell
graphify E:/SM --no-viz
```

If HTML is lightweight enough and the tool supports it cleanly, prefer the default HTML output instead of suppressing visualization.

Expected:
- `graphify-out/GRAPH_REPORT.md`
- `graphify-out/graph.json`
- optional `graphify-out/graph.html`

**Step 2: Review the audit outputs**

Read:

```powershell
Get-Content -Encoding utf8 E:/SM/graphify-out/GRAPH_REPORT.md
Get-Content -Encoding utf8 E:/SM/graphify-out/graph.json | Select-Object -First 40
```

Expected:
- report exists
- graph export exists

**Step 3: Update handoff truth to reflect the new audit reality**

- remove the “graph audit missing” issue if the output is present and usable
- record any residual audit limitation that remains

### Task 4: Verify And Journal The Closeout

**Files:**
- Modify: `E:/SM/CHANGELOG_TASK.MD`

**Step 1: Run acceptance verification**

Run:

```powershell
cargo test --test security_account_open_position_snapshot_cli -- --nocapture
cargo test -- --nocapture
```

Expected:
- PASS
- no repository-level blocker introduced by the tail closeout

**Step 2: Append the task journal entry**

- record the warning cleanup
- record the handoff path cleanup
- record the graph audit generation
- record the exact verification commands

**Step 3: Re-check current truth**

Run:

```powershell
git status --short
```

Expected:
- only the intended tail-closeout files are touched for this slice
