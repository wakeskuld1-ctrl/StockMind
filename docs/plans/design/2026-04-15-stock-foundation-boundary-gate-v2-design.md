# Stock/Foundation Boundary Gate V2 Design

采用 `方案 B`。

## Shared/Runtime Hold-Zone Guard

The guard checks that stock shell layers stay thin, shared/runtime files remain present, and no future change silently reintroduces foundation ownership into the standalone stock repo.
