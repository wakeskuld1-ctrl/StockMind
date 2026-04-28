---
name: gold-etf-daily-signal
description: Use when the user wants a daily next-trading-day signal for the gold ETF rule, including whether to buy, hold, sell, or observe on 518800.SH using the approved mean-reversion rulebook and local research artifacts.
---

# Gold ETF Daily Signal

Use this skill when the user asks for:

- tomorrow's gold ETF buy point
- whether to buy or sell `518800.SH`
- a daily rule-based decision for the gold ETF strategy
- a next-open recommendation using the approved gold ETF rule

## Workflow

1. Read the rulebook at:
   - `E:\SM\docs\product\2026-04-28-gold-etf-518800-rulebook.md`
2. Run the daily tool:
   - `E:\SM\scripts\research\gold_etf_daily_signal_tool.py`
3. Interpret the output conservatively:
   - `buy_next_open`
   - `hold`
   - `sell_next_open`
   - `observe`
4. State clearly that:
   - execution is based on `T` close and `T+1` open
   - proxy premium is not part of the formal live rule
   - this is a rule-based decision, not certainty of profit

## Required Inputs

- `--as-of-date YYYY-MM-DD`

Optional position inputs:

- `--position-entry-date YYYY-MM-DD`
- `--position-entry-price <float>`
- `--strategy-name fail_to_rebound_d5_hold_20d`

## Default Recommendation Stack

- Primary rule:
  - `518800.SH + fail_to_rebound_d5_hold_20d`
- Backup reference:
  - `518800.SH + fail_to_rebound_d3_hold_20d`

## Output Shape

Always summarize with:

1. next-day action
2. target ETF
3. rule trigger reason
4. current rule state
5. risk note

## Refusal Rules

Do not guess when:

- the requested date is outside local history
- required position inputs are missing for a sell/hold judgment
- local research artifacts are missing or inconsistent
