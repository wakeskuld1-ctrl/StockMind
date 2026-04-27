# Nikkei Volume Strength Filter Study Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Test whether volume expansion around 20MA consecutive breakouts helps identify fast-bull follow-through and improves breakout quality.

**Architecture:** Use packaged Nikkei daily price-volume history and the existing 20MA consecutive breakout event set as immutable inputs. Compare volume features between fast-bull and slow-bull breakout samples, then evaluate simple volume-based filters on later path quality. No production rule changes are allowed in this phase.

**Tech Stack:** Python, pandas, packaged CSV artifacts, `.verification` outputs

---

## Execution Contract

- **Chosen approach:** Volume feature comparison plus simple filter evaluation on 20MA breakout events.
- **Allowed change boundary:** Analysis-only outputs and tracking docs.
- **Explicit non-goals:**
  - Do not modify production regime logic.
  - Do not deploy any volume filter without later approved rule design.
- **Acceptance checks:**
  - Volume features are computed consistently for all breakout events.
  - Fast-bull versus slow-bull volume differences are reported.
  - At least one simple volume filter is tested on 180-day path quality.

## Core Questions

1. Do fast-bull 20MA breakout events show clearer volume expansion than slow-bull events?
2. Can a simple volume filter improve the ratio of breakout events whose later path stays mostly above the signal point?
3. Does volume help more for 3-day confirmation or 5-day confirmation?

## Candidate Volume Features

- breakout-day volume / 20-day average volume
- breakout-day volume / 60-day average volume
- average volume ratio across confirmation window
- maximum volume ratio within confirmation window
- breakout-day price return with volume ratio

## Candidate Filters (Evaluation Only)

- breakout-day volume ratio above median
- confirmation-window average volume ratio above median
- breakout-day volume ratio above fixed thresholds such as `1.1x`, `1.2x`, `1.3x`

## Expected Outputs

- One fast-vs-slow volume comparison table
- One volume-filter performance table on 180-day path quality
- One concise memo on whether volume is useful as a key strength filter
