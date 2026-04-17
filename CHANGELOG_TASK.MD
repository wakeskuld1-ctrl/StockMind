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
## 2026-04-17
### 修改内容
- `src/ops/stock_online_scoring_and_aggregation.rs` / `src/ops/stock_model_lifecycle.rs`：恢复 `super::super::...` 真实 re-export 层级，目的=修复为迎合 guard 而引入的编译错误。
- `tests/stock_modeling_training_split_source_guard.rs`：改为校验真实的 subgroup thin-shell 事实，目的=让 source-guard 跟代码现实一致。
- `src/ops/security_scorecard.rs`：收紧 ETF runtime guard，目的=拒绝缺少 ETF 通用或子池必要特征族的错误模型绑定。
- `src/test_support.rs` / `src/lib.rs` / `src/runtime/formal_security_runtime_registry.rs` / `src/ops/security_execution_record.rs` / `src/ops/security_real_data_validation_backfill.rs`：新增 test-only env mutex 并接入 env 修改路径，目的=消除并发测试的环境变量污染。
- `tests/security_analysis_contextual_cli.rs`：固定 fixture `as_of_date` 并收紧 breakout/breakdown 样本影线，目的=防止 integration test 被 2026-04-17 实时同步污染。
- `tests/security_chair_resolution_cli.rs`：开始收口 chair fixture 漂移，已补部分 `as_of_date`、ETF helper 通用 family 与 breakout fixture，目的=为 chair contract 回归继续排雷。

### 修改原因
- 需要先完成“必须先做”的边界真相对齐，让 stock-only 仓库的 catalog / dispatcher / docs / source-guard 回到同一事实面。
- 需要完成“可以并行”阶段里最关键的两件事：修 ETF 真 bug、让测试结果重新可信。
- 全量回归继续扫出 `security_chair_resolution_cli` 的集中失败，说明后续仍需单独收口 chair contract / fixture 偏差。

### 方案还差什么?
- [ ] 收敛 `tests/security_chair_resolution_cli.rs` 剩余 9 个失败，区分 fixture 漂移、ETF latest-proxy 语义、chair contract 预期偏差。
- [ ] 跑通完整 `cargo test -- --nocapture` 全绿后，再进入最后清理阶段。
- [ ] 按用户要求补一次 Graphify 关系图与偏差对照。

### 潜在问题
- [ ] `security_chair_resolution_cli` 当前失败已经不是单点，而是 chair 输出契约与测试预期存在系统性偏差。
- [ ] ETF chair fixtures 虽已补通用 family，但 treasury/gold/latest-proxy 语义仍需继续核实。
- [ ] 仓内仍有 warning（如未使用 import / helper），最后清理阶段还没开始。

### 关闭项
- 已通过 `cargo test --test stock_catalog_grouping_source_guard -- --nocapture`
- 已通过 `cargo test --test stock_formal_boundary_manifest_source_guard -- --nocapture`
- 已通过 `cargo test --test stock_foundation_boundary_source_guard -- --nocapture`
- 已通过 `cargo test --test stock_foundation_boundary_gate_v2_source_guard -- --nocapture`
- 已通过 `cargo test --test stock_modeling_training_split_source_guard -- --nocapture`
- 已通过 `cargo test --test security_legacy_committee_public_catalog_guard -- --nocapture`
- 已通过 `cargo test --test security_legacy_committee_application_surface_guard -- --nocapture`
- 已通过 `cargo test --test security_disclosure_history_backfill_cli -- --nocapture`
- 已通过 `cargo test --test security_external_proxy_history_import_cli -- --nocapture`
- 已通过 `cargo test --test security_fundamental_history_backfill_cli -- --nocapture`
- 已通过 `cargo test --test security_history_expansion_cli -- --nocapture`
- 已通过 `cargo test --test security_model_promotion_cli -- --nocapture`
- 已通过 `cargo test --test security_shadow_evaluation_cli -- --nocapture`
- 已通过 `cargo test ops::stock::security_scorecard::tests::etf_runtime_guard --lib -- --nocapture`
- 已通过 `cargo test --lib -- --nocapture`
- 已通过 `cargo test --test security_analysis_contextual_cli -- --nocapture`
- [ ] 未通过验证：`cargo test --test security_chair_resolution_cli -- --nocapture`，当前剩余 9 个失败

### 记忆点（可选）
- 用户明确要求“不要任何幻想”，所以 guard/test 必须尊重代码真实结构，不能为了变绿去强推一个不存在的层级或契约。
- 用户提醒过 decision chain 已换新链，因此 integration test 先判定是 fixture/as_of_date 漂移，还是业务契约真的变了，再决定修测试还是修代码。
## 2026-04-17
### 修改内容
- `src/ops/security_decision_submit_approval.rs`：统一改为按最终落盘 JSON 重新计算 package artifact manifest 的 sha256，目的：修复 verify-package 对 scorecard/signature 等工件的 hash 校验失配。
- `src/ops/security_execution_record.rs` / `src/ops/security_post_trade_review.rs` / `src/runtime/security_execution_store_execution_record_repository.rs`：补齐 lifecycle request 兼容层、execution record ref 回读与 post-trade overlay，目的：打通 condition review -> execution record -> post trade review -> package revision 的正式生命周期切片。
- `src/tools/dispatcher/stock_ops.rs`：把 `security_record_post_meeting_conclusion` 收成“兼容旧 chair-request 合同 + 新 package_path orchestration 合同”的双入口，并落盘 post-meeting artifact 后串上 package revision，目的：修复 post-meeting 真实调用面分叉。
- `src/ops/security_post_trade_review_policy.rs`：恢复 `next_account_adjustment_hint` 的中文正式文案，目的：消除 review policy 输出的英文漂移。
- `tests/security_scorecard_cli.rs`：把 scorecard CLI 从共享 live sqlite fixture 改成每次自建并导入历史数据的隔离 runtime db，目的：消除共享测试夹具被污染后的不稳定失败。
- `src/ops/security_analysis_fullstack.rs`：把 fullstack 的 fundamental/disclosure 查询顺序改成“先 governed history，后 live fallback”，目的：恢复 validation/replay 优先消费治理历史的正式语义。

### 修改原因
- 用户要求先把真实问题修完，再回头说明问题在哪，所以本轮按真实失败顺序逐条收口，而不是停在分析层。
- 全量回归暴露出的失败不是单点，而是 verify/package revision/lifecycle/post-meeting/governed-history 多条合同边界没有完全收平。
- 用户反复强调“不要任何幻想”，因此每一刀都先抓真实 error response 或 assertion，再只修对应根因。

### 方案还差什么?
- [ ] 按用户要求补一次 Graphify 关系图与偏差对照
- [ ] 按用户要求继续做多智能体/多视角的风险扫描

### 潜在问题
- [ ] `tests/security_account_open_position_snapshot_cli.rs` 当前仍有 `execution_request` 未使用 warning，属于非功能性噪音，但不影响本轮全量结果。
- [ ] 本轮是基于现有 contract 与回归测试把边界收平；如果后续再次替换 lifecycle / decision chain 合同，仍需同步更新 dispatcher、revision、verify 与 fixture。

### 关闭项
- 已通过 `cargo test --test security_decision_verify_package_cli security_decision_verify_package_accepts_signed_package_and_writes_report -- --nocapture`
- 已通过 `cargo test --test security_lifecycle_validation_cli security_lifecycle_validation_slice_round_trips_formal_tools -- --nocapture`
- 已通过 `cargo test --test security_post_meeting_conclusion_cli security_record_post_meeting_conclusion_creates_conclusion_and_revises_package -- --nocapture`
- 已通过 `cargo test --test security_scorecard_cli submit_approval_persists_formal_scorecard_even_without_model_artifact -- --nocapture`
- 已通过 `cargo test --test security_stock_history_governance_cli security_analysis_fullstack_prefers_governed_stock_history_before_live_fetch -- --nocapture`
- 已通过 `cargo test -- --nocapture`

### 记忆点（可选）
- 用户关键线 1：先修真实问题，不要先写长篇分析。
- 用户关键线 2：不要动平衡记分卡那块。
- 用户关键线 3：全仓盘点的前提是先扫完再定优先级，不能扫一半就主观判断“快完了”。
## 2026-04-17
### 修改内容
- `README.md`：重写为“仓库目标 + 分层结构 + CLI 用法 + runtime/db 路径 + 验收入口”的可上手文档，目的：让独立出的 stock 工程不再只靠口头上下文才能使用和验收。
- `docs/architecture/stockmind-acceptance-checklist.md`：新增“设计基线 -> 交付面 -> source-guard/mainline/full regression”对照清单，并补方法/依赖/数据库调用图，目的：把“设计图和交付图是否吻合”从主观判断收成可执行验收流程。

### 修改原因
- 用户要求回答 CI 是否已能防止层级/模块范围模糊化，以及设计图和交付图如何验收，因此需要把现有 boundary guard、formal mainline 和全量回归整理成显式文档。
- 现有 README 只够说明“这是什么仓库”，不够支持“如何运行、如何验证、如何判断这轮 refactor 已完成”。
- 用户要求整理 README 并加入使用方法，所以这轮文档必须和真实 CLI 输入格式、runtime 解析规则保持一致，不能写概念性说明。

### 方案还差什么?
- [ ] 若用户后续要做正式交接，可继续补一份“最常用工具的最小可运行请求样例”附录。
- [ ] 若用户后续要做架构审计，可继续补 Graphify 关系图并把结果挂到 acceptance checklist。

### 潜在问题
- [ ] README 当前给的是最稳妥的 `tool_catalog` 示例；如果后续要补更多业务工具示例，必须继续用真实可运行请求验证，避免文档样例漂移。
- [ ] 验收清单当前覆盖的是本轮批准的 split/boundary/mainline 基线；后续若新增正式模块或改变设计文档，需同步更新 checklist 与 source-guard 测试。

### 关闭项
- 已验证 `README.md` 中的 `tool_catalog` CLI 示例可运行，并返回真实 tool catalog。
- 已补齐 `README.md` 的 runtime/db 路径说明，并对齐 `src/runtime_paths.rs` 与 `src/runtime/formal_security_runtime_registry.rs` 的当前解析规则。
- 已新增 `docs/architecture/stockmind-acceptance-checklist.md`，把设计文档、结构 guard、主链回归和全量回归串成一套显式验收入口。

### 记忆点（可选）
- 用户关心的不只是“代码能跑”，还关心“怎么证明层级没糊、怎么证明设计和交付对齐、怎么让独立工程拿回去就能用”。
## 2026-04-17
### 修改内容
- `docs/AI_HANDOFF.md`：补充 stock 边界扩张规则、dispatcher 增长规则、当前高压风险区和验收提醒，目的：让后续 AI/开发者交接时明确“不要继续扩大 `stock.rs`，未来优先新拉 bus/分组壳”。

### 修改原因
- 用户明确要求把当前风险写进 AI 交接手册，并且把未来演进约束说清楚，而不是继续靠口头提醒。
- 当前仓库已经重构收口，后续更重要的是防止新的会话又把 `stock.rs` 和 `stock_ops.rs` 堆回模糊边界。

### 方案还差什么?
- [ ] 如后续要进一步硬化约束，可再补一条 source-guard，把“未来新增能力优先新拉 bus，不继续扩大 `stock.rs`”做成测试门禁。

### 潜在问题
- [ ] 当前 handoff 规则主要依赖文档 + 现有 CI/source-guard；如果后续团队直接跳过 handoff 审阅，仍可能出现边界再次膨胀。

### 关闭项
- 已补齐 `docs/AI_HANDOFF.md` 的风险交接说明，覆盖 `stock.rs` 冻结边界、`stock_ops.rs` 高压区、legacy committee 兼容定位和验收入口。

### 记忆点（可选）
- 用户这一步要的是“交接时把未来不要怎么做写清楚”，尤其是不要再默认往 `stock.rs` 上堆新能力。
