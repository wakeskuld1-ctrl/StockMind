## Objective
- 目标：让另一台主机上的 AI 或工程师，在不依赖当前聊天上下文的前提下，能够继续日经指数 `1w` 周频训练工作。
- 范围：只覆盖日经 `nikkei_index + direction_head` 这条训练线，不覆盖黄金、台股、ETF 溢价。

## Current Baseline
- Git 分支：`codex/p10-p11-clean-upload-20260420`
- 当前远端同步提交：`b13c2a5` `Sync Nikkei 1w training implementation`
- 相关前置提交：
  - `7180fde` `Add Nikkei 1w training handoff note`
  - `eb8ec79` `Add isolated cargo verification runner and record local audit blocker`
- 首先阅读：
  - [2026-04-24-nikkei-1w-training-handoff.md](/E:/SM/docs/handoff/2026-04-24-nikkei-1w-training-handoff.md)
  - [2026-04-24-nikkei-1w-cross-host-resume.md](/E:/SM/docs/handoff/2026-04-24-nikkei-1w-cross-host-resume.md)
  - [security_scorecard_training.rs](/E:/SM/src/ops/security_scorecard_training.rs)
  - [security_scorecard_training_cli.rs](/E:/SM/tests/security_scorecard_training_cli.rs)

## Data Boundary
- 这条训练在别的主机上要继续，必须先确认两类数据真源。

### Stock Source Of Truth
- 必需数据库：`stock_history.db`
- 当前本机真实路径：`E:\SM\.stockmind_runtime\stock_history.db`
- 角色：日经现货 `NK225.IDX` 与期货 `NK225_F1.FUT` 的正式价格真源
- 禁止误用：
  - 不能把某个训练输出目录里的 `runtime.db` 当成价格真源
  - 不能把 artifact 目录当成历史数据输入根

### Capital Flow Source Of Truth
- 必需数据库：`security_capital_flow.db`
- 当前本机真实路径：`E:\SM\.stockmind_runtime\capital_flow_real_2016_2025_20260422_b\security_capital_flow.db`
- 角色：JPX / MOF 周频资金流真源
- 禁止误用：
  - 不能把 artifact_runtime_root 当成 capital flow 输入根
  - 不能假设当前训练已经接入 `1年持续增强` 因子；当前正式训练里实际还是 `4w persistence`

### Artifact Root
- 当前训练输出目录示例：
  - `E:\SM\.stockmind_runtime\nikkei_weekly_direction_head_20260423_1w`
- 角色：只负责落训练产物、registry、diagnostics、run_output
- 禁止误用：
  - 不能反向把这里面的 `runtime.db` 当成价格或资金流输入真源

## Verified State
- 已验证：
  - 日经周频 `direction_head` 标签已切为 `positive_return_1w`
  - 周滚动评估合同已改成 `24周训练 + 1周验证 + 1周测试 + 每周滚动`
  - `valid/test` 已按 window 级训练后再汇总，不再是旧的全局压平污染口径
- 最近一版真实训练结果：
  - 运行文件：[run_output.json](/E:/SM/.stockmind_runtime/nikkei_weekly_direction_head_20260423_1w/run_output.json)
  - `target_label_definition = positive_return_1w`
  - `rolling_window_count = 462`
  - `valid_accuracy ≈ 0.4697`
  - `test_accuracy ≈ 0.4870`
  - `holdout_sample_count = 12`
  - `holdout_accuracy = 0.75`
- 当前不应误读：
  - holdout 只有 `12` 周，不能把 `0.75` 当作稳定结论
  - 当前资金持续性因子不是 `1y`，而是：
    - `overseas_flow_persistence_4w`
    - `domestic_flow_persistence_4w`
    - `mof_foreign_japan_equity_net_4w`
    - `overseas_vs_domestic_spread`

## Cross-Host Restore Steps
### 1. Clone And Checkout
```powershell
git clone <repo-url>
cd E:\SM
git checkout codex/p10-p11-clean-upload-20260420
git pull
```

### 2. Confirm Branch Truth
```powershell
git log --oneline -3
```
- 预期能看到：
  - `b13c2a5`
  - `7180fde`
  - `eb8ec79`

### 3. Prepare Data Roots
- 在新主机上，先把正式价格库和正式资金流库准备好。
- 最低要求：
  - `stock_history.db` 内有：
    - `NK225.IDX`
    - `NK225_F1.FUT`
  - `security_capital_flow.db` 内有：
    - `jpx_weekly_investor_type`
    - `mof_weekly_cross_border`

### 4. Set Runtime Env
```powershell
$env:EXCEL_SKILL_STOCK_DB = 'D:\runtime\stock_history.db'
$env:EXCEL_SKILL_CAPITAL_FLOW_DB = 'D:\runtime\security_capital_flow.db'
```
- 注意：
  - `EXCEL_SKILL_RUNTIME_DB` 可以指向训练输出 runtime
  - 但 `EXCEL_SKILL_STOCK_DB` 和 `EXCEL_SKILL_CAPITAL_FLOW_DB` 必须显式指向正式数据真源

### 5. Optional Stub For Financials / Announcements
- 若只续训练，不想被财报/公告链拖住，可继续用本地 stub。
- 现有做法是：
  - `/financials -> 406`
  - `/announcements -> 空列表`

### 6. Re-run Minimal Training
```powershell
$env:EXCEL_SKILL_RUNTIME_DB = 'D:\runtime\nikkei_1w_resume\runtime.db'
$env:EXCEL_SKILL_STOCK_DB = 'D:\runtime\stock_history.db'
$env:EXCEL_SKILL_CAPITAL_FLOW_DB = 'D:\runtime\security_capital_flow.db'

Get-Content .\training_request_nikkei_1w.json -Raw | .\target\debug\excel_skill.exe
```
- 说明：
  - 如果没有现成请求文件，直接参考 [run_output.json](/E:/SM/.stockmind_runtime/nikkei_weekly_direction_head_20260423_1w/run_output.json) 对应的请求参数重建
  - 当前训练合同仍是：
    - `instrument_subscope = nikkei_index`
    - `target_head = direction_head`
    - `horizon_days = 10` 仅保留旧字段兼容；真实标签定义已是 `1w`

## Do Not Guess
- 不要猜：
  - 当前训练已经用了 `1年持续增强` 资金因子
  - 当前模型已经可用
  - artifact 目录可以充当输入数据真源
- 必须明确：
  - 这版训练解决的是“周标签 + 周滚动合同”
  - 还没有解决的是“1年持续增强资金因子”

## Next Recommended Action
1. 在新主机上先恢复并确认数据真源路径。
2. 先重跑最小日经 `1w` 训练，确认可续跑。
3. 然后直接进入下一阶段：
   - 设计并接入 `1年持续增强` 资金因子
   - 重训日经 `1w`
   - 再做因子复盘
