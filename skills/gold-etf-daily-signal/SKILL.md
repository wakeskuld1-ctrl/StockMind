---
name: gold-etf-daily-signal
description: Use when the user wants a daily next-trading-day signal, position review, two-layer add decision, or 10D-15D repair-window follow-up for 518800.SH using the approved gold ETF mean-reversion rulebooks and local research artifacts.
---

# Gold ETF Daily Signal

Use this skill when the user asks for:

- tomorrow's gold ETF buy point
- whether to buy or sell `518800.SH`
- a daily rule-based decision for the gold ETF strategy
- a next-open recommendation using the approved gold ETF rule
- whether to add the second layer
- whether to keep observing the current 10D-15D repair window
- a daily review of the current `518800.SH` position

## Workflow

1. Read the rulebook at:
   - `E:\SM\docs\product\2026-04-28-gold-etf-518800-rulebook.md`
   - `E:\SM\docs\product\2026-04-29-gold-etf-518800-two-layer-rulebook.md`
   - `E:\SM\docs\product\2026-04-29-gold-etf-518800-candidate-partial-exit-rule.md`
   - `E:\SM\docs\product\2026-04-29-gold-etf-518800-position-review.md`
2. Run the daily tool:
   - `E:\SM\scripts\research\gold_etf_daily_signal_tool.py`
3. For flat-account questions, interpret the output conservatively:
   - `buy_next_open`
   - `hold`
   - `sell_next_open`
   - `observe`
4. For the active user position, apply the two-layer rule first:
   - current position after 2026-04-29 first layer: `3685` shares
   - current blended cost after first layer: about `11.0732`
   - first-layer anchor price: `10.112`
   - first layer is considered complete under the normalized strategy frame
   - second-layer trigger observation price: about `9.61` (`10.112 * 0.95`)
   - second layer requires both price trigger and renewed gold-parent signal
   - no third layer is allowed
5. State clearly that:
   - execution is based on `T` close and `T+1` open
   - proxy premium is not part of the formal live rule
   - this is a rule-based decision, not certainty of profit
   - current live handling is to observe the 10D-15D repair window unless the second-layer trigger is met

## Required Inputs

- `--as-of-date YYYY-MM-DD`

Optional position inputs:

- `--position-entry-date YYYY-MM-DD`
- `--position-entry-price <float>`
- `--strategy-name fail_to_rebound_d5_hold_20d`

## Default Recommendation Stack

- Current active rule:
  - `518800.SH + two-layer position rule`
- Layer 1:
  - parent signal triggers on `T` close
  - buy on `T+1` open
  - target weight: `50%`
- Layer 2:
  - price falls about `-5%` from first-layer anchor
  - gold parent signal triggers again
  - buy on next open
  - target weight: `40%`
- No third layer.
- Exit frame:
  - `max_hold_days = 20`
  - `rebound_check_day = 5`
- Exit research note:
  - `15D` return-anchor extension was tested through `200D`
  - best ranked variant still stopped at `max_hold_days = 20`
  - do not promote `15D` anchor trailing into the live rule unless a later approved study replaces it
- Candidate partial-exit rule:
  - status: candidate formal rule, not yet the only live rule
  - if `D15` strategy return is above `1%`, sell `70%` of the strategy position on `D16` open
  - track the remaining `30%` from `D18`
  - if the remaining position falls `1.2%` from the post-`D18` highest close, sell the rest on the next open
  - if `D15` return is not above `1%`, do not partial-sell and allow the position to run to `D60` unless another hard exit triggers
  - do not retroactively trigger partial sell if `D16-D20` later moves above the `D15` threshold; this branch was tested and did not rank best
  - for the active 2026-04-29 first-layer position, provisional dates are `D15 = 2026-05-22`, `D16 = 2026-05-25`, `D18 = 2026-05-27`, `D60 = 2026-07-24`

## Output Shape

Always summarize with:

1. next-day action
2. target ETF
3. rule trigger reason
4. current rule state
5. risk note

For the current live position, also include:

1. current layer state
2. first-layer anchor price
3. second-layer trigger zone
4. whether renewed parent signal exists
5. whether the position is still in the 10D-15D repair window
6. whether the candidate partial-exit rule has reached `D15`
7. if `D15` has arrived, whether the close is above the current trigger level

## Current Live Position State

Use this as the active position context unless the user gives a newer fill:

- symbol: `518800.SH`
- old snapshot: `1885 @ 11.988`
- first-layer fill: `1800 @ 10.112` on `2026-04-29`
- total position: `3685`
- blended cost: about `11.0732`
- first-layer anchor: `10.112`
- second-layer trigger zone: about `9.61`
- current action after first-layer fill: observe, do not add unless second-layer conditions are both met
- candidate partial-exit trigger:
  - D15 date: `2026-05-22`
  - D15 trigger level using current blended cost: about `11.184`
  - if triggered, D16 planned reduction is about `70%` of `3685`, roughly `2580` shares before execution rounding

## Daily Review Rules

When reviewing the current position:

- If price is above the second-layer trigger zone, do not suggest another add.
- If price is near or below `9.61`, check whether the gold parent signal is renewed before suggesting layer 2.
- If price is near or below `9.61` but the gold parent signal is not renewed, recommend observation rather than adding.
- Treat `1D-5D` movement as noisy unless a formal exit condition triggers.
- Treat `10D-15D` as the main repair window.
- Do not apply the candidate partial-exit rule before `D15`.
- On `D15`, compare the closing price against the current trigger level before suggesting partial sell.
- After any partial sell, record the remaining shares and the post-`D18` highest close before applying the `1.2%` drawdown exit.
- Update the position review document when a real fill, exit, or daily close review is recorded.

## Refusal Rules

Do not guess when:

- the requested date is outside local history
- required position inputs are missing for a sell/hold judgment
- local research artifacts are missing or inconsistent
- the user asks for a second-layer add but the latest price or renewed parent-signal state is unknown
- the user asks for a third-layer add
