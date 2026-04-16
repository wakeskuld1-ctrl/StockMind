## 2026-04-16
### 修改内容
- `Cargo.toml`：把快照仓改为独立包 `stockmind`，同时保留 `excel_skill` 库名和兼容二进制名，目的是让迁移后的测试与调用面继续可用。
- `src/lib.rs` / `src/main.rs` / `src/tools/dispatcher.rs` / `src/tools/contracts.rs` / `src/tools/catalog.rs`：收缩为 stock-only 入口、路由和契约，目的是去掉 foundation / GUI / license 噪音，保留证劵主链独立可运行能力。
- `src/ops/*` / `src/runtime/*`：按 stock 边界复制独立运行所需模块，目的是形成可单独 clone 的证券业务仓。
- `tests/security_*` / `tests/stock_*` / `tests/runtime_fixtures/security*|stock*`：复制关键证券回归与夹具，目的是验证迁移后的关键主链没有断。
- `docs/architecture/stockmind-snapshot-manifest.md`：新增迁移清单，目的是把来源、边界、运行时和已知缺口写清楚。

### 修改原因
- 需要把股票分析 / 治理 / 执行 / 复盘 / 训练链从原仓独立出来，形成一个可以直接拉取继续开发的新仓。
- 方案采用快照迁移，不保留完整历史，优先追求最快、最稳、可直接工作的独立仓。

### 方案还差什么?
- [ ] 推送本地 `StockMind` 仓到远端 GitHub：`https://github.com/wakeskuld1-ctrl/StockMind`
- [ ] 生成一次 split repo 的 graphify 关系图与审计报告

### 潜在问题
- [ ] 兼容性名称仍保留 `excel_skill` 作为库名和测试二进制名；如果后续要彻底品牌化，需要统一改名并批量更新测试。
- [ ] 夹具目录较深，Windows 长路径在某些工具上仍可能产生读取噪音。
- [ ] 当前验证覆盖了关键 package/verify/revision 主链，但没有一次性跑完整 stock 全量测试矩阵。

### 关闭项
- 已通过 `cargo check`
- 已通过 `cargo test --test security_decision_verify_package_cli -- --nocapture`
- 已通过 `cargo test --test security_decision_package_cli -- --nocapture`
- 已通过 `cargo test --test security_decision_package_revision_cli -- --nocapture`

### 记忆点（可选）
- 用户要的是“能独立拉回来就能干”的真实 stock 仓，不是保历史的漂亮迁移。
- 遇到 worktree 分叉时，要先做真实盘点；这次主仓比几个 worktree 都更新，应以主仓为准。

## 2026-04-16
### 修改内容
- `CHANGELOG_TASK.MD`：追加独立仓推送记录，目的是把 `StockMind` 已完成远端初始化、跟踪分支建立和可拉取状态补齐到任务日志里。

### 修改原因
- 本轮任务的交付目标不是继续改代码，而是把已经验证通过的拆分结果正式落到 `https://github.com/wakeskuld1-ctrl/StockMind`，形成真正可独立协作的仓库入口。

### 方案还差什么?
- [ ] 生成一次 split repo 的 graphify 关系图与审计报告
- [ ] 视后续品牌化计划，评估是否移除 `excel_skill` 兼容命名并统一测试与调用面

### 潜在问题
- [ ] 当前远端已可用，但如果后续继续从原 monorepo 双向搬运功能，容易出现接口漂移，需要尽快把后续 stock 开发主线切到 `StockMind`
- [ ] 尚未补跑完整 stock 全量测试矩阵；当前仍以关键主链用例为主

### 关闭项
- 已通过 `git push -u origin main`
- 已通过 `git ls-remote --heads origin`，确认远端存在 `refs/heads/main`
- 已通过 `git status --short --branch`，确认本地 `main` 已跟踪 `origin/main`

### 记忆点（可选）
- 用户要的是“新开一个工程后，独立拉回来就能直接干”的结果，所以远端仓可拉取、可继续开发，比保留历史更重要。
