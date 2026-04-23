# Stock/Foundation Split Manifest Design

## Shared / Runtime Hold Zone

In the standalone repo, the hold zone contains only stock-facing shared files under `src/tools/*` and governed runtime files under `src/runtime/*`.

## Adapter 规则

Compatibility adapters may remain only when they are explicitly labeled, scoped to stock-facing ownership, and do not reopen the removed foundation boundary.
