---
name: health
description: Analyze Apple Health and Health Connect exports — workouts, sleep, trends. Use when: health, workouts, sleep, steps, heart rate, apple health, healthkit, or health connect.
invocation: model+user
---

# Health

## When to use
Answering questions about the user's activity, sleep, workouts, and trends
from their own exported health data.

## Setup
The user exports once from their phone and points you at the files:

- iPhone: Health app → profile → Export All Health Data → `export.xml`
- Android: Health Connect → Export data

Fail loud when no export is present. Never ask for health-app credentials —
exports only.

## Workflow
1. Parse `export.xml` (or the Health Connect export) with a script; do not
   paste the whole file into context.
2. Answer the question asked: totals, averages, trends over explicit date ranges.
3. Charts on request via the `dataviz` skill.

## Non-goals
- Do not diagnose, prescribe, or interpret data as medical advice.
- Do not copy health data anywhere except the analysis the user asked for.
- Do not retain health facts in memory unless the user explicitly asks.
