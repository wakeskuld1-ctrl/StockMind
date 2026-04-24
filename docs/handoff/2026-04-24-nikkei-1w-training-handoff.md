## Current Objective
- 任务目标：把日经指数周频训练从旧的 `10d direction` 口径切到“未来一周”口径，并在可信的周滚动评估合同下完成一版真实训练与复盘。
- 当前阶段：训练链路已跑通，`1w` 标签已生效，正在做交接封包；模型效果仍不达标，且资金持续增强因子时间尺度与原要求不一致。

## Contract And Decision State
- 已冻结的合同：
  - 日经当前只走周频训练路线。
  - 评估合同是 `24周训练 + 1周验证 + 1周测试 + 每周滚动`。
  - 周频 `valid/test` 必须按 window 单独训练、单独打分、再汇总，不能压平成全局三桶。
  - 当前标签已从旧的 `positive_return_10d` 切成 `positive_return_1w`。
- 已完成的设计决策：
  - `direction_head` 在日经周频路线下按“当前周锚点 -> 下一周锚点”定义标签。
  - artifact 训练样本与 rolling-window 评估样本已分离。
  - 真实训练继续使用：
    - 现货：`NK225.IDX`
    - 期货：`NK225_F1.FUT`
    - 资金流库：`E:\SM\.stockmind_runtime\capital_flow_real_2016_2025_20260422_b\security_capital_flow.db`
- 仍未收口的点：
  - 真实 artifact 的 `model_id / artifact_path / registry_id` 之前仍出现 `10d` 残留，需要用最新编译产物再确认一次是否已完全切到 `1w`。
  - “资金流向是否持续增大”当前实际接入的是 `4w` 因子，不是用户要的 `1年` 口径。

## Evidence And Verification
- 已验证事实：
  - 周频 split 合同修复后，相关测试通过：
    - `cargo test weekly_ --test security_scorecard_training_cli -- --nocapture`
  - 标签层已切成 `1w`：
    - 单测已补：
      - `build_artifact_serializes_target_label_definition_for_up_and_down_heads`
      - `resolve_weekly_direction_label_uses_next_anchor_close_direction`
  - 真实训练已跑通一版，运行目录：
    - [run_output.json](/E:/SM/.stockmind_runtime/nikkei_weekly_direction_head_20260423_1w/run_output.json)
  - 真实训练核心结果：
    - `target_label_definition = positive_return_1w`
    - `positive_label_definition = positive_return_1w`
    - `rolling_window_count = 462`
    - `valid.sample_count = 462`
    - `valid.accuracy ≈ 0.4697`
    - `test.sample_count = 462`
    - `test.accuracy ≈ 0.4870`
    - `post_validation_holdout.sample_count = 12`
    - `post_validation_holdout.accuracy = 0.75`
- 已确认的问题：
  - holdout 样本只有 `12` 周，不能把 `0.75` 当成稳定结论。
  - 当前 artifact 里的资金持续性因子实际是：
    - `overseas_flow_persistence_4w`
    - `domestic_flow_persistence_4w`
    - `mof_foreign_japan_equity_net_4w`
    - `overseas_vs_domestic_spread`
  - 这与用户想看的“1年持续增强”不一致。
- 仍未验证：
  - 最新源码下重新编译后的真实 artifact / registry 命名是否完全从 `10d` 切成 `1w`。
  - diagnostics 子块是否也需要统一改成 `1w` 命名与口径。

## Open Risks And Blockers
- 当前最大业务风险：
  - 这版训练口径已可信，但 `valid/test` 仍接近 50%，模型还不可用。
- 当前最大合同风险：
  - 资金持续增强因子时间尺度漂移。
  - 用户要求的是“1年持续增强”，当前训练实际只用了 `4周持续增强`。
- 当前最大工程阻塞：
  - 本地 Rust 编译极慢，且此前残留 `cargo/rustc` 进程造成锁冲突，导致最终“命名合同收口”没有完成实证确认。
- 明确不能猜的事项：
  - 不能把 `4w persistence` 当成 `1y persistence` 对外汇报。
  - 不能把 `holdout 0.75` 当成模型已经显著可用。
  - 不能假定最新 artifact 文件名已经正确，必须重新编译并重跑最小真实训练确认。

## Truth File Routing
- Current-status truth:
  - 本文档作为当前日经 `1w` 训练专项交接。
  - 最新真实训练结果以 [run_output.json](/E:/SM/.stockmind_runtime/nikkei_weekly_direction_head_20260423_1w/run_output.json) 为准。
- Unresolved issues:
  - `1w` 标签已生效，但 artifact/registry 命名是否完全改成 `1w` 仍待确认。
  - 资金持续增强因子仍是 `4w`，尚未改成 `1y`。
  - 模型效果仍不达标，需要继续做因子重构或删减。
- Historical context:
  - 周滚动 split 污染问题已修复，旧的 `fixsplit` 结果不应再作为正式结论引用。
  - 黄金、台股当前均不在本次专项范围内。

## Resume Guide
- Read first:
  - [2026-04-24-nikkei-1w-training-handoff.md](/E:/SM/docs/handoff/2026-04-24-nikkei-1w-training-handoff.md)
  - [security_scorecard_training.rs](/E:/SM/src/ops/security_scorecard_training.rs)
  - [security_scorecard_training_cli.rs](/E:/SM/tests/security_scorecard_training_cli.rs)
  - [run_output.json](/E:/SM/.stockmind_runtime/nikkei_weekly_direction_head_20260423_1w/run_output.json)
- Re-run first:
  - 先检查是否存在残留 `cargo/rustc` 进程，避免继续卡编译锁。
  - 再用最新二进制做最小真实训练验证，只核对：
    - `artifact.model_id`
    - `artifact_path`
    - `registry_id`
    是否全部从 `10d` 切成 `1w`。
- Next action:
  1. 完成 `1w` 命名合同收口验证。
  2. 明确新增“1年持续增强”资金因子方案，不再沿用当前 `4w` 结果冒充。
  3. 重训一版日经 `1w`，再做因子拆解。

## Memory Points
- 用户优先级：
  - 结果可信 > 先跑出一个数字。
  - 时间尺度必须对齐，`4周` 不能替代 `1年`。
- 本次关键断点：
  - 训练完成，不代表可用。
  - `1w` 标签已切换成功，不代表资金因子已经满足原始要求。
