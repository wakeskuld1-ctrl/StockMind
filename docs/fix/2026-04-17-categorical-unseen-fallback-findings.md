# 2026-04-17 Categorical Unseen Fallback Findings

## 背景

- 真实 40 股训练在 2026-04-17 的诊断复跑里失败，错误为：`no categorical bin matched feature 'has_risk_warning_notice' value 'true'`。
- 本轮目标不是升级模型，而是先把训练链路的稳定性收住，确保真实训练不会因为验证集或测试集出现 train 未见过的分类值而直接中断。

## 根因

- 当前 `FeatureModel` 的 categorical bins 只基于 `train split` 构建。
- 4/17 新增的 training diagnostics 会重新编码 `train + valid + test` 三个 split，而不再只消费训练期编码结果。
- 因此，一旦 `valid/test` 首次出现 train 没见过的分类值，例如 `has_risk_warning_notice=true`，旧实现就在 `resolve_feature_woe(...)` 直接报错。

## 为什么之前不卡、现在会卡

- 之前训练链主要停在 artifact / registry / refit 输出，没有对全量 `train + valid + test` 样本再次做 diagnostics 编码。
- 所以旧链虽然存在“未知分类值无落点”这个结构性缺口，但没有被真实触发出来。
- 4/17 补上 diagnostics 以后，链路第一次正式要求对验证集和测试集做同口径重编码，这个潜在缺口才被暴露出来。
- 结论：这不是“新 bug 凭空出现”，而是训练诊断链把旧训练合同里的隐藏缺口真实揭开了。

## 本轮修复

- 在 `security_scorecard_training.rs` 中为每个 categorical feature 追加 `__unseen__` fallback bin。
- `resolve_feature_woe(...)` 在 exact match 失败时回退到 `__unseen__`。
- fallback bin 采用 neutral `woe = 0.0`，避免对未知类别注入伪方向偏置。
- 清理 `security_scorecard_training_cli.rs` 中误插入到 happy-path 测试的 disclosure no-op 调用，避免 fixture 污染主链测试。

## 验证结果

- 定向测试通过：
  - `security_scorecard_training_generates_artifact_and_registers_refit_outputs`
  - `security_scorecard_training_keeps_numeric_feature_contract_when_fundamental_metrics_are_missing`
  - `security_scorecard_training_tolerates_unseen_categorical_values_in_diagnostic_splits`
- 真实 40 股训练已复跑成功，未再因 unseen categorical value 中断。

## 真实 40 股复跑结论

- 运行状态：`ok`
- promotion decision：`candidate_only`
- production readiness：`blocked`
- 样本与特征：
  - `sample_count = 160`
  - `feature_count = 36`
  - `sample_per_feature = 4.44`
- 指标：
  - `train_accuracy = 0.9875`
  - `valid_accuracy = 0.525`
  - `test_accuracy = 0.675`
  - `walk_forward_mean_accuracy = 0.6125`
- 当前阻塞原因：
  - `sample_per_feature_is_below_minimum`
  - `high_correlation_pairs_detected`
  - `counterintuitive_bins_detected`

## 判断

- 代码稳定性问题：本轮已收住。真实训练现在可以跑完，不再被 unseen categorical value 直接打断。
- 当前主要矛盾：已经从“代码崩溃”切换成“训练质量不足”。
- 优先级判断：
  1. 先补训练样本厚度与标签覆盖，尤其是风险/公告相关样本。
  2. 再治理高相关和反直觉分箱。
  3. 最后才值得讨论更重的模型升级。

## 风险与建议测试

- 风险：如果后续再新增 categorical feature，但没有走统一 fallback 合同，真实 rerun 仍可能在别的字段复发。
  - 建议测试：补一条“test-only 新分类值”回归测试，覆盖更多 categorical field，而不是只锁 `has_risk_warning_notice`。
- 风险：当前 `__unseen__` 为 neutral，不会崩，但也不会给出方向信息。
  - 建议测试：补一条 diagnostics artifact 检查，确保所有 categorical feature 都显式落盘 `__unseen__` bin。
- 风险：训练可跑通不代表已可实盘放行。
  - 建议测试：继续保留真实 40 股 rerun 冒烟测试，固定检查 `production_readiness != ready` 时必须输出完整 warnings。
