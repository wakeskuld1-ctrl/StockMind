# 日经 ETF HGB/RF V3 模型总览（2026-04-28）

## 1. 文档目的

这份文档用于把当前日经 ETF 主线研究收口成一份可直接阅读的总览，重点回答 5 个问题：

1. 现在到底在用什么算法。
2. 这些算法对应什么交易规则。
3. 训练是怎么做的，为什么这样做。
4. 目前准确率、回测收益和风险表现怎样。
5. 哪些结论已经可以引用，哪些还不能当成正式结论。

本文只基于当前研究包内已经落盘的真实产物整理，不使用聊天记忆补数字。

## 2. 当前模型体系

当前主线不是“预测日经明天涨还是跌”，而是“当日经进入调仓点时，ETF 应该维持什么风险仓位，并如何在 159866 / 513520 之间执行”。

现有体系分成 3 层：

### 2.1 底层规则层：V3 基础仓位框架

V3 是底层仓位锚：

- 它先根据趋势、支撑阻力、市场状态给出一个 `base_position_v3`
- 然后再决定是否需要在这个基础上减仓、维持或加仓

这层不是黑盒模型，而是规则化框架。相关对比见：

- `artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/adjustment_point_analysis/54_strategy_risk_adjusted_metrics.csv`

### 2.2 模型层：HGB 增强 V3 与 RF 增强 V3

在 V3 基础仓位之上，再用分类模型判断调仓动作：

- `-1`：降低风险仓位
- `0`：维持基础仓位
- `1`：试仓或加仓

目前主要比较两条模型线：

- `hgb_l2_leaf20`
- `rf_depth4_leaf20`

当前研究结论：

- `HGB` 是主模型，用来做风险仓位调整判断
- `RF` 是辅助模型，用来做分歧检测和二次解释，不是替代主模型

相关产物：

- `artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/adjustment_point_analysis/58_v3_adjustment_model_backtest_summary.csv`
- `artifacts/03_daily_hgb_rf_scoring_full_snapshot/02_model_validation_metrics_live_pre_year.csv`
- `artifacts/03_daily_hgb_rf_scoring_full_snapshot/03_global_feature_importance_live_pre_year.csv`

### 2.3 执行层：ETF 实盘化执行规则

执行层不直接交易日经指数，而是交易日经 ETF：

- `159866`
- `513520`

当前研究包里，已经把模型信号做成了“更接近实盘”的回测口径：

- 使用 `T-1` 收盘后得到的信号
- 在 `T` 日 ETF 开盘价执行
- 计入单边 `3bp` 成本
- 买入时优先选择开盘溢价代理值更低的 ETF
- 去掉调仓死区后的“双 ETF 择低溢价买入”是当前更接近实盘的执行版本

相关产物：

- `artifacts/02_live_like_backtest_full_snapshot/08_no_deadband_decision_summary.csv`
- `artifacts/02_live_like_backtest_full_snapshot/09_no_deadband_decision_audit.csv`

## 3. 当前核心规则

### 3.1 调仓规则不是纯涨跌预测

研究过程中已经放弃“泛化的未来涨跌预测”作为唯一目标，原因是：

- 牛市阶段里，大量 `10D` 标签天然偏正
- 这样会把真正重要的“什么时候该调仓”泛化掉

因此现在的规则重点是：

- 突破阻力位后是否站稳
- 量能是否配合
- 是否远离支撑位
- 是否进入高位风险区域

### 3.2 开仓/加仓规则

研究主张不是“只要看多就满仓”，而是围绕突破后的站稳过程做确认：

- `3D` 站稳：更适合试仓
- `5D` 站稳：更适合确认加仓
- `10D`：更偏向判断这次突破是否已经走得太晚

量能在这里是“确认条件”，不是独立决定因素。

### 3.3 减仓规则

减仓逻辑主要由下面几类信号触发：

- 价格显著远离支撑位
- 高位放量但支撑保护不足
- 下跌阶段量价结构变差
- HGB 给出 `-1` 风险调整

在当前实盘解释里，如果：

- HGB 明确减仓
- RF 只是中性

不能直接判定模型失效，而应该理解为：

- 趋势还没完全坏
- 但风险收益比已经变差

### 3.4 ETF 执行规则

当前更接近实盘的 ETF 执行规则是：

- 信号依据 `T-1`
- 执行使用 `T` 日开盘
- 买入时在 `159866` 与 `513520` 中择低溢价
- 卖出不受高溢价阻断
- 默认单边成本 `3bp`

这套规则已经在研究包里有独立回测结果，但还不是最终生产 Tool 的完整自动化体系。

## 4. 训练思路

### 4.1 训练目标的变化

训练目标经历过两次关键收敛：

1. 从“泛化的未来方向预测”转向“调仓点预测”
2. 从旧的 `10d` 周期残留逐步切换到当前日经周频 `1w` 口径

当前研究包里的周频正式训练对象是：

- `instrument_subscope = nikkei_index`
- `target_head = direction_head`
- `target_label_definition = positive_return_1w`

可以在这里看到：

- `artifacts/01_training_and_intermediate_full_snapshot/training_result.json`

### 4.2 训练数据与切窗

当前主训练结果对应：

- 训练区间：`2020-10-01..2025-09-30`
- 验证区间：`2025-10-01..2025-12-31`
- OOT / 测试观察窗口：`2026-01-01..2026-04-17`

样本切法不是简单随机拆分，而是按时间顺序切分，避免未来信息泄漏。

### 4.3 当前实盘口径

2026-04-28 起，当前批准口径不再是“拿一份旧 daily snapshot 手工解释”，而是：

1. 扩窗重训
2. 每日 walk-forward
3. 只消费 `live_pre_year`

具体含义：

- 治理层的重训 / 年度 walk-forward 回测按扩窗口径推进，而不是长期冻结旧训练窗口。
- 评分层按日执行 walk-forward 式 live workflow。
- `known_labels_asof` 只保留为诊断口径，不能进入实盘信号链。
- 每次运行都要同时暴露 `as_of_date` 和 `effective_signal_date`，避免把“请求日期”误当成“真实有效信号日”。

需要特别注意：

- 当前 2026 年 `live_pre_year` 日常 workflow 的训练/验证切分是固定的：`train through 2025-09-30`，`validate on 2025Q4`，然后对请求区间逐日打分。
- 扩窗 cadence 主要体现在治理层重训和年度 HGB walk-forward，不等于“每天都重写 2026 live 训练窗”。

当前操作入口是：

- `python D:\SM\scripts\run_nikkei_hgb_rf_daily_workflow.py --as-of-date 2026-04-27 --score-start-date 2026-04-01 --journal-dir D:\SM\docs\trading-journal\nikkei`

这个 workflow 会：

- 跑一轮 `live_pre_year` daily scoring
- 读取 `05_latest_adjustment_artifacts_live_pre_year.csv`
- 读取 `06_daily_workflow_manifest_live_pre_year.json`
- 输出 HGB / RF 的稳定摘要
- 可选自动写入 `docs/trading-journal/nikkei/`

如果请求日已经晚于最后一个有效 live row，workflow 会回退到 `<= as_of_date` 的最后一个可用信号日，并明确写出 `effective_signal_date`。因此实盘解释必须跟随 `effective_signal_date`，不能只看请求日期。

实现字段名分别是：

- artifact table：`requested_as_of_date`、`effective_as_of_date`
- workflow manifest：`latest_artifact_as_of_date`
- stdout 摘要：`effective_signal_date=...`

### 4.4 特征设计思路

当前主线特征分为两大组：

#### 价格行为特征

例如：

- `weekly_spot_return_*`
- `weekly_spot_close_position`
- `weekly_spot_drawdown`
- `weekly_spot_rebound`

#### 量能与量价行为特征

例如：

- `weekly_volume_ratio_4w`
- `weekly_volume_ratio_13w`
- `weekly_volume_ratio_26w`
- `weekly_volume_ratio_52w`
- `weekly_price_position_52w`
- `weekly_volume_accumulation_26w`
- `weekly_volume_accumulation_52w`
- `weekly_high_volume_low_price_signal`
- `weekly_high_volume_breakout_signal`
- `weekly_up_day_volume_share`
- `weekly_down_day_volume_share`
- `weekly_volume_price_confirmation`

### 4.5 资金面处理原则

资金面目前已经从“直接进训练”降级为“观察层”。

原因不是资金面一定无用，而是当前这条线尚未证明：

- 表达方式稳定
- 时间尺度正确
- 对主模型有净增益

这也是当前一个明确未收口点：

- 用户要求的是 `1y` 持续增强口径
- 但部分资金持续性表达之前仍停留在 `4w`

所以目前更稳妥的结论是：

- 资金可以看
- 但不能把现有 `4w` 结果冒充成 `1y` 结果

## 5. 当前准确率与模型效果

## 5.1 周频正式训练结果

来自：

- `artifacts/01_training_and_intermediate_full_snapshot/training_result.json`

当前主结果：

| 指标 | 数值 |
|---|---:|
| feature_count | 22 |
| sample_count | 244 |
| train_accuracy | 0.6267 |
| valid_accuracy | 0.4840 |
| test_accuracy | 0.5251 |
| post_validation_holdout_accuracy | 0.4074 |
| post_validation_holdout_sample_count | 27 |
| walk_forward_mean_accuracy | 0.4918 |
| production_readiness | caution |
| sample_per_feature | 11.09 |

这组结果说明：

- 训练链条已经跑通
- 口径也已经基本统一
- 但纯“未来一周方向”预测力仍然不强

### 5.2 周频不同特征版本比较

来自：

- `artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/01_metrics_comparison.csv`

| run | feature_count | valid | test | holdout | walk_forward | readiness |
|---|---:|---:|---:|---:|---:|---|
| no_proxy | 14 | 0.4475 | 0.4566 | 0.3704 | 0.5328 | caution |
| short_proxy | 14 | 0.4749 | 0.4566 | 0.4074 | 0.5164 | caution |
| yfinance_10y | 14 | 0.5205 | 0.5023 | 0.3704 | 0.5328 | caution |
| long_volume_behavior | 22 | 0.4840 | 0.5251 | 0.4074 | 0.4918 | caution |

这张表反映的不是“特征越多越好”，而是：

- 长周期量能行为特征在 `test` 上略有改善
- 但 `walk_forward` 反而变弱
- 说明这组增强还没有形成稳定优势

### 5.3 HGB / RF 日度实盘化验证结果

来自：

- `artifacts/03_daily_hgb_rf_scoring_full_snapshot/02_model_validation_metrics_live_pre_year.csv`

| model | validation_accuracy | validation_balanced_accuracy | validation_rows |
|---|---:|---:|---:|
| hgb_l2_leaf20_live | 0.4839 | 0.4074 | 62 |
| rf_depth4_leaf20_live | 0.5484 | 0.3148 | 62 |

解释：

- `RF` 的表面准确率更高
- 但 `HGB` 的平衡准确率更好
- 这和两者行为风格一致：
  - `HGB` 更愿意主动提示风险下降
  - `RF` 更容易给中性/观望

因此当前主线仍然保留：

- `HGB` 为主
- `RF` 为辅

### 5.4 调仓模型回测表现

来自：

- `artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/adjustment_point_analysis/58_v3_adjustment_model_backtest_summary.csv`

关键对比：

| strategy | total_return | CAGR | Sharpe | Max Drawdown |
|---|---:|---:|---:|---:|
| hgb_l2_leaf20 | 2.5175 | 0.1533 | 1.3647 | -0.1064 |
| rf_depth4_leaf20 | 1.6054 | 0.1147 | 1.0205 | -0.1933 |
| V3_base_comparable | 1.0076 | 0.0823 | 0.7603 | -0.1598 |
| BUY_HOLD_comparable | 2.0693 | 0.1356 | 0.7183 | -0.3180 |

这说明：

- `HGB` 相比 `V3 base` 明显改善
- `HGB` 相比 `BUY_HOLD` 并不是收益绝对最高，但风险调整后更优
- `RF` 明显弱于 `HGB`

### 5.5 2022-2026 走行 HGB 结果

来自：

- `artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/adjustment_point_analysis/64_walk_forward_hgb_backtest_summary.csv`

关键对比：

| strategy | final_jpy | total_return | CAGR | Sharpe | Max Drawdown | avg_position |
|---|---:|---:|---:|---:|---:|---:|
| WF_HGB_adjusted_V3_2022_2026 | 18,696,346.55 | 0.8696 | 0.1615 | 1.3653 | -0.0958 | 0.4788 |
| V3_base_2022_2026 | 17,317,802.39 | 0.7318 | 0.1404 | 1.1455 | -0.1520 | 0.5357 |
| BUY_HOLD_2022_2026 | 20,379,704.31 | 1.0380 | 0.1858 | 0.8647 | -0.2626 | 1.0000 |

结论：

- `BUY_HOLD` 绝对收益更高
- 但 `WF_HGB_adjusted_V3_2022_2026` 的 Sharpe 更高，回撤显著更小
- 如果目标是“实盘持仓纪律 + 风险控制”，HGB 增强 V3 仍是当前最值得保留和理解的主模型

### 5.6 ETF 实盘化回测结果

来自：

- `artifacts/02_live_like_backtest_full_snapshot/08_no_deadband_decision_summary.csv`

当前更接近实盘的组合结果：

| portfolio | strategy | final_cny | return | annualized | sharpe | max_drawdown |
|---|---|---:|---:|---:|---:|---:|
| dual_low_premium | dual_low_premium_buy_no_deadband_3bp | 1,800,728.04 | 0.8007 | 0.1465 | 0.9943 | -0.1329 |

解释：

- 这套结果已经把模型信号接到了 ETF 执行层
- 不是纯指数回测，而是更接近现实交易的口径
- 目前 Sharpe 接近 1，但还不能简单宣称为“生产级稳定实盘策略”

## 6. 当前最重要的解释性结论

### 6.1 HGB 最看重什么

来自：

- `artifacts/03_daily_hgb_rf_scoring_full_snapshot/03_global_feature_importance_live_pre_year.csv`

HGB 的前几项重要度是：

1. `dist_sup60`
2. `dist_res20`
3. `dist_sup20`
4. `weighted_b60_vol`
5. `ma50_over_ma200`
6. `dist_ma200`

含义很明确：

- HGB 不是单纯追涨模型
- 它最关心的是“价格离支撑有多远、离阻力有多远、量能是否在高位放大”
- 所以在牛市高位，HGB 比 RF 更容易提示减仓

### 6.2 RF 为什么经常更中性

RF 的重要度更偏向：

- `ma50_over_ma200`
- `dist_sup20`
- `component_above200_breadth`
- `dist_ma200`

这意味着 RF 更依赖：

- 趋势仍在不在
- 成分股广度是否还健康

所以当市场已经涨得比较远，但广度和趋势还没坏时：

- HGB 会先减风险
- RF 往往还会偏中性

## 7. 当前未收口问题

### 7.1 训练链已跑通，但方向预测力仍弱

周频正式训练虽然已经口径收口，但：

- `valid_accuracy = 0.4840`
- `walk_forward_mean_accuracy = 0.4918`
- `post_validation_holdout_accuracy = 0.4074`

这说明：

- 当前一周方向预测本身并不强
- 模型价值更可能体现在“调仓约束”和“风险管理”，而不是单纯押涨跌

### 7.2 registry / refit 的旧 `10d` 残留已经完成整包刷新

这一条现在要分成“已经修完的部分”和“仍未修完的部分”来看：

1. 代码与测试层已经修复
2. 研究包训练快照也已经在 2026-04-28 完成 post-fix rerun 刷新

代码层修复内容没有变：

- `registry_id`
- `candidate_registry_ref`

现在会优先依据 artifact truth / model identity 生成日经周频 `1w` token，而不是继续从通用 `horizon_days=10` 回退成旧的 `10d` 后缀。

对应保护已经进入测试：

- `tests/security_scorecard_refit_cli.rs`
- `tests/security_scorecard_training_cli.rs`

而且这次已经把研究包里的训练快照同步刷新：

- `training_result.json`
- `scorecard_model_registry/*.json`
- `scorecard_refit_runs/*.json`
- `artifact_manifest.csv`

因此当前正确口径应是：

- 当前研究包训练快照里的正式 registry/refit 元数据已经与代码口径一致，使用 `1w` token
- 如果后续新的周频 rerun 再出现 `...10d-direction_head`，应直接视为回归缺陷，而不是历史残留

### 7.3 资金持续性时间尺度还没完全对齐

当前最重要的未收口点之一：

- 用户要的是 `1y` 持续增强
- 但历史研究里部分资金持续性特征和结论仍是 `4w`

这部分现在只能说：

- 观察逻辑已经有了
- 但还不能当成最终资金增强版本

### 7.4 量能增强并没有自动带来明显提升

当前研究已经证明：

- 量能很重要
- 但“量能重要”不等于“随便加量能字段就会变强”

尤其是长周期量能行为版本，虽然局部 `test` 有改善，但整体稳定性并没有同步提升。

## 8. 当前可以怎么用，不能怎么用

### 8.1 可以这样用

- 把 `HGB 增强 V3` 当成当前主风险仓位模型
- 把 `RF` 当成辅助解释和分歧检测器
- 把 ETF 执行层理解为“择低溢价买入 + 开盘执行 + 3bp 成本”的实盘近似口径
- 把 `scripts/run_nikkei_hgb_rf_daily_workflow.py` 当成当前标准 daily operator 入口
- 只消费 `live_pre_year` artifact，并用 `effective_signal_date` 解释当天有效信号
- 把 `src/ops/security_nikkei_etf_position_signal.rs` 理解为只接受 `live_pre_year` HGB artifact 的正式 ETF Tool
- 把 journal 默认落盘位置理解为 `D:\SM\docs\trading-journal\nikkei\`

### 8.2 不能这样用

- 不能把 `known_labels_asof` 当成实盘信号
- 不能把请求日 `as_of_date` 直接当成有效信号日，必须同时确认 `effective_signal_date`
- 不能把周频 `holdout` 或单次高命中率样本直接当成生产结论
- 不能把 `4w` 资金持续性结果当成 `1y` 结果对外汇报
- 不能把当前模型说成“已经稳定预测日经未来一周涨跌”
- 不能把旧的无 `train_policy` JSON 或非 policy-qualified JSON 当成正式 live artifact
- 不能把 `known_labels_asof` artifact 或非 policy 文件名喂给 `security_nikkei_etf_position_signal`

## 9. 当前总判断

如果只用一句话总结当前研究状态：

**这套体系已经从“泛化的涨跌预测”收敛到“围绕调仓点做风险仓位管理”，其中 HGB 增强 V3 仍是当前最优主线；当前 live 口径也已经收口到“扩窗重训 + 每日 walk-forward + live_pre_year artifact only”，但周频 `1w` 方向训练本身预测力仍弱，资金持续性时间尺度也还没有完全对齐，因此现在适合当研究型实盘辅助系统，不适合被表述成完全成熟的自动交易模型。**

## 10. 推荐阅读顺序

1. `README.md`
2. `ALGORITHM_HANDOFF_MANUAL.md`
3. `artifacts/01_training_and_intermediate_full_snapshot/analysis_exports/adjustment_point_analysis/64_walk_forward_hgb_backtest_summary.csv`
4. `artifacts/02_live_like_backtest_full_snapshot/08_no_deadband_decision_summary.csv`
5. `artifacts/03_daily_hgb_rf_scoring_full_snapshot/02_model_validation_metrics_live_pre_year.csv`
6. `artifacts/03_daily_hgb_rf_scoring_full_snapshot/03_global_feature_importance_live_pre_year.csv`

## 11. Continuation Head Update (2026-04-28)

The research stack is now four-layer:

1. base position framework
2. HGB / RF adjustment model
3. replay classifier
4. continuation head

The continuation head is not a replacement for the HGB/RF line. It is a second-stage research layer that compresses replay truth into a binary continuation-quality target over `1D / 3D / 5D`.

Current reality:

- the pipeline now exists and is reproducible;
- `1D` continuation is more learnable than `5D`;
- raw validation accuracy is not a safe headline because the target is extremely imbalanced toward continuation;
- operator integration is not approved yet.
