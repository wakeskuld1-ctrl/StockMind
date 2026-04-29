# 黄金 ETF 518800.SH 持仓复盘卡

## 1. 文档目标

这份文档用于跟踪 `518800.SH` 当前持仓、后续加仓、以及从 `2026-04-29` 起未来 `15` 个交易日的持续复盘。

当前用途只限于：

- 记录用户确认过的真实持仓信息
- 记录后续新增买卖
- 记录规则信号状态
- 记录每日复盘结论与下一动作

当前不用于：

- 自动对接券商
- 替代正式回测台账
- 强行反推不一致历史口径

## 2. 当前已确认信息

### 2.1 历史成交记录

| 日期 | 动作 | 数量 | 价格 | 金额 | 备注 |
| --- | --- | ---: | ---: | ---: | --- |
| 2026-02-04 | 买入 | 2900 | 10.824 | 31389.60 | 用户提供 |
| 2026-04-03 | 卖出 | 900 | 9.782 | 8803.80 | 用户提供 |

### 2.2 当前持仓快照

| 截止日 | 标的 | 剩余持仓 | 当前成本 | 备注 |
| --- | --- | ---: | ---: | --- |
| 2026-04-29 | 518800.SH | 1885 | 11.988 | 用户提供当前券商口径 |

### 2.3 口径提醒

- 历史成交记录与当前持仓快照之间存在未解释差异。
- 当前不强行把 `2900 - 900` 与 `1885` 做机械对齐。
- 在拿到更多真实成交记录前，以“用户提供当前持仓快照”为实盘判断基准。

## 3. 当前策略约束

### 3.1 主规则

- 标的：`518800.SH`
- 主规则：`fail_to_rebound_d5_hold_20d`
- 入场母体：黄金 `Au99.99`
- 入场条件：
  - `5D收益率 <= -2.0%`
  - `close_vs_ma20 <= -1.5%`
- 执行口径：
  - `T` 日收盘出信号
  - `T+1` 日开盘执行

### 3.2 当前决策冻结

- `2026-04-28` 收盘，黄金母体正式触发入场信号。
- `2026-04-29` 对空仓账户属于正式可买日。
- 对当前已有旧仓，采用“受规则约束的分层加仓”。

### 3.3 分层加仓计划

| 层级 | 触发条件 | 计划资金 | 状态 |
| --- | --- | ---: | --- |
| 第一笔 | 已在 `2026-04-29` 正式可买日 | 18000 | 待成交 |
| 第二笔 | 后续再次满足正式母体信号，且相对上一笔加仓价再跌约 `3%~4%` | 20000 | 未触发 |
| 第三笔 | 后续再次满足正式母体信号，且相对第二笔加仓价再跌约 `3%~4%` | 24000 | 未触发 |

## 4. 后续新增成交记录区

| 记录时间 | 交易日 | 动作 | 数量 | 价格 | 金额 | 执行原因 | 更新后总持仓 | 更新后综合成本 |
| --- | --- | --- | ---: | ---: | ---: | --- | ---: | ---: |
| 2026-04-29 | 2026-04-29 | 第一笔加仓买入 | 1800 | 10.112 | 18201.60 | `2026-04-28` 母体信号触发，`2026-04-29` 执行 | 3685 | 11.0732 |

## 5. 15 交易日复盘模板

| 交易日序号 | 日期 | 黄金母体状态 | ETF 收盘 | 相对综合成本盈亏 | 当日动作 | 规则判断 | 风险备注 | 下一步 |
| --- | --- | --- | ---: | ---: | --- | --- | --- | --- |
| D1 | 2026-04-29 | 第一笔正式母体信号执行日 | 待收盘 | 盘后补 | 已完成第一笔加仓买入 | 继续观察修复是否延续，不追第二笔 | 短期 `1D~5D` 不稳，仍需防范继续下探 | 等待收盘后更新 D1 结论 |
| D2 | 待补充 | 待补充 |  |  |  |  |  |  |
| D3 | 待补充 | 待补充 |  |  |  |  |  |  |
| D4 | 待补充 | 待补充 |  |  |  |  |  |  |
| D5 | 待补充 | 待补充 |  |  |  |  |  |  |
| D6 | 待补充 | 待补充 |  |  |  |  |  |  |
| D7 | 待补充 | 待补充 |  |  |  |  |  |  |
| D8 | 待补充 | 待补充 |  |  |  |  |  |  |
| D9 | 待补充 | 待补充 |  |  |  |  |  |  |
| D10 | 待补充 | 待补充 |  |  |  |  |  |  |
| D11 | 待补充 | 待补充 |  |  |  |  |  |  |
| D12 | 待补充 | 待补充 |  |  |  |  |  |  |
| D13 | 待补充 | 待补充 |  |  |  |  |  |  |
| D14 | 待补充 | 待补充 |  |  |  |  |  |  |
| D15 | 待补充 | 待补充 |  |  |  |  |  |  |

## 6. 当前复盘结论

- 当前这笔仓位不是规则带来的亏损单，而是历史执行偏离规则后的遗留仓位。
- `2026-04-29` 这次处理，逻辑不是追涨，而是基于黄金母体超跌修复信号做受约束加仓。
- 从历史独立样本看，这类信号短期 `1D~5D` 不稳，但 `10D~15D` 修复概率更高。
- 因此当前处理目标不是立刻解套，而是压低成本并等待修复窗口。
- 第一笔加仓已成交：`1800 @ 10.112`，成交金额 `18201.60`。
- 以用户提供的旧仓快照 `1885 @ 11.988` 为基准，本次成交后总持仓约 `3685`，新综合成本约 `11.0732`。

## 7. 当前待办

- [x] 建立持仓台账
- [x] 建立 15 交易日复盘卡
- [x] 录入 `2026-04-29` 第一笔加仓真实成交
- [x] 根据真实成交价重算综合成本
- [ ] 等待 `2026-04-29` 收盘后补全 D1 收盘复盘
- [ ] 从第一笔真实成交日起滚动更新 `D2~D15` 复盘

## 8. Candidate Partial-Exit Rule Mapping (added 2026-04-29)

This clean appendix records the candidate exit rule that passed the partial-exit stability review.

### Rule status

- Status: candidate formal rule, not yet the only live rule.
- Entry and two-layer add rules stay unchanged.
- The old fixed `20D` exit is not deleted.

### Candidate rule

- If `D15` strategy return is above `1%`, sell `70%` of the strategy position on `D16` open.
- Track the remaining `30%` from `D18`.
- If the remaining position falls `1.0%` from the post-`D18` highest close, sell the rest on the next open.
- If `D15` return is not above `1%`, do not partial-sell and allow the position to run to `D60`, unless another hard exit triggers.

### Current position mapping

- D1: `2026-04-29`
- D5: `2026-05-08`
- D10: `2026-05-15`
- D15: `2026-05-22`
- D16: `2026-05-25`
- D18: `2026-05-27`
- D20: `2026-05-29`
- D45: `2026-07-03`
- D60: `2026-07-24`

### Current trigger estimate

- Current shares: `3685`
- Current blended cost: about `11.0732`
- D15 trigger level: `11.0732 * 1.01 = 11.1839`
- If `2026-05-22` close is above about `11.184`, the candidate rule triggers a `D16` partial sell.
- `70%` of `3685` is about `2580` shares before execution rounding.
- Remaining shares would be about `1105`, then tracked from `D18` for the `1.0%` high-watermark drawdown rule.

### Current action

- Before D15: observe.
- Do not apply the candidate partial-exit rule early.
- Continue to watch the second-layer add zone around `9.61`; a second layer still requires renewed parent signal confirmation.

## 9. D15-Unmet Branch Correction (added 2026-04-29)

User correction: if D15 is below the trigger but D16-D20 later repairs above the trigger, the rule cannot be left undefined.

Updated data-backed result:

- Tested rolling trigger windows ending at D15, D18, and D20.
- Tested unmet exits at D20, D30, D45, and D60.
- Best result still uses only the D15 trigger point.
- If D15 is not above the trigger, later D16-D20 closes above the trigger do not retroactively trigger the 70% partial sell.
- Best unmet branch remains hold-to-D60, unless another hard exit or manual risk rule is introduced.
- Remaining-position high-watermark drawdown is updated from `1.0%` to `1.2%` after this branch-efficiency review.

Best branch result:

- Rule: `p0.70_thr0.010_win15_dd0.012_off2_unmet60`
- Cumulative return: `141.64%`
- Max drawdown: `-14.27%`
- Return/drawdown ratio: `9.92`
- CAGR: about `8.93%`
- Average hold: about `27.56` trading days

Current practical reading:

- If D15 close is above about `11.184`, D16 open sells about `70%`.
- Remaining shares start high-watermark tracking from D18.
- Remaining shares exit if they fall `1.2%` from the post-D18 highest close.
- If D15 close is not above about `11.184`, do not partial sell and do not re-trigger on D16-D20; continue toward D60 unless another hard exit applies.
