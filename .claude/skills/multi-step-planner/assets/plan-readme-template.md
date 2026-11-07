# Plan: <Title>

## Goal

<1-3 sentences: what this plan achieves and why.>

## Context & key decisions

<Anything a fresh session needs to know before touching this plan: constraints discovered during investigation, decisions already made (and why), things intentionally out of scope for the whole plan.>

## Steps

| # | Step | Status | Depends on |
|---|------|--------|------------|
| 1 | [<short name>](steps/step-01-<slug>.md) | pending | — |
| 2 | [<short name>](steps/step-02-<slug>.md) | pending | 1 |

> Status values: `pending`, `in-progress`, `done`, `blocked`.

## How to execute this plan

Run the `execute-plan-step` skill (e.g. "execute the next step of the `<slug>` plan"). It will:
1. Pick the next `pending` step whose dependencies are `done`.
2. Enter plan mode to surface and resolve that step's open questions with you before writing any code.
3. Implement just that step, then mark it `done` in this table and in the step file.

Call it again to move on to the following step — steps are executed one at a time, not all at once.
