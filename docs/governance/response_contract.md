# StockMind Response Contract

## Purpose

This file defines what a high-quality project answer must expose when an engineer or AI reports status, completion, risk, or handoff information.

## Required Response Behavior

Every substantial project answer should:

- distinguish between current verified state, historical context, and planned follow-up
- name the exact command or file that supports an important claim
- use exact dates when a status statement could drift over time
- call out blockers plainly instead of hiding them behind optimistic summaries

## Required Status Framing

When reporting branch health:

- state the current branch when relevant
- say whether the working tree is clean or not
- separate `cargo check` from `cargo test`
- identify the first blocking failure if full regression is red

When reporting architecture state:

- say which file is the current rule source
- say whether the statement is a stable boundary rule or a branch-local observation

## What Must Not Be Claimed Without Fresh Evidence

Do not claim any of the following unless it was re-verified in the current branch:

- the repository is fully green
- a contract still matches all downstream consumers
- a graph audit already exists
- a handoff file is current

## Preferred Answer Structure

For most engineering status answers, use this order:

1. current truth
2. why it matters
3. what remains
4. where the authoritative files live

## Refusal Rules

The answer must refuse or soften certainty when:

- the branch was not verified
- the file set is internally inconsistent
- a historical handoff file disagrees with current command output
- a referenced file does not exist in the current workspace

Recommended language:

- "historical note, not current proof"
- "not verified on this branch"
- "present in handoff notes, absent from current workspace"
- "current command output takes precedence"
