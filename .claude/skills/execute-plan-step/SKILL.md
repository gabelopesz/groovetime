---
name: execute-plan-step
description: Executes the next pending step from a multi-step plan produced by the "multi-step-planner" skill (a plans/ directory containing a plan folder with a README.md and steps/*.md). Make sure to use this skill whenever the user says things like "execute the next step", "continue the plan", "work on step N", "implement the plan", or references an existing plan directory and wants to move it forward. Every step execution MUST begin in plan mode — investigate the step and surface/resolve ambiguities with the user BEFORE writing or editing any code. Only ever executes ONE step per invocation; call again to continue.
---

# Execute Plan Step

Executes exactly one pending step from a plan (created by `multi-step-planner`), never more. Every invocation starts in **plan mode**: understand the step, find ambiguities, resolve them with the user — and only after that, implement.

## Workflow

### 1. Locate the plan and the step

- If the user names a plan or step explicitly, use that.
- Otherwise look for `plans/*/README.md` (or `docs/plans/*/README.md`) in the repo. If there's exactly one plan, use it. If there are several, ask the user which one.
- Open the plan's `README.md`, read the step table. Pick the first step whose `Status` is `pending` AND whose `Depends on` steps are all `done`.
  - If no such step exists (all done, or the next pending step has unmet dependencies), tell the user the plan's current state and stop — don't guess around a dependency.
  - If the user explicitly asked for a specific step out of order, confirm that's intentional (its dependencies may not be done yet) rather than silently skipping the check.
- Read that step's file in full.

### 2. Enter plan mode — investigate and surface ambiguities

**Do not write or edit any files yet.** This is the core discipline of this skill: every step, however small it looks, starts with an investigation phase.

- Read the step's "Open questions / ambiguities" section — these are known unknowns flagged at planning time and must be resolved now.
- Independently re-check the step against the current state of the code: plans can go stale (files may have moved, been renamed, or the assumption behind the step may no longer hold). Look for anything the plan didn't anticipate.
- Think through the acceptance criteria: are they still achievable as written? Is anything underspecified (which library, which exact file, which naming convention, error-handling behavior, edge cases)?
- Compile a short list of concrete questions/decisions, if any. Small, low-stakes details you can reasonably default on — just note the default you're picking. Reserve real questions for things that would change the shape of the implementation or are hard to reverse.

**If Claude Code's plan mode is active (an `ExitPlanMode` tool is available):** present your implementation plan for this one step through that tool, folding in answers to any questions you needed to ask the user first. Don't exit plan mode until genuine ambiguities are resolved.

**If plan mode isn't a distinct tool in this environment:** simulate the same discipline — present your proposed approach for the step in the chat, ask any necessary clarifying questions, and get an explicit go-ahead from the user before making any edits.

If there were truly no open questions and the step is unambiguous, say so briefly and proceed — this step doesn't exist to manufacture questions, only to catch real ones.

### 3. Implement the step

Once ambiguities are resolved:

- Implement only what this step's "In scope" section covers. If you notice something that belongs to a later step, leave it — don't scope-creep.
- Follow the step's "Context" and "Notes for execution" sections.
- Work through the acceptance criteria as you go; run tests/commands where applicable to verify them rather than assuming.

### 4. Close out the step

- Update the step file: set `**Status:**` to `done`, check off completed acceptance criteria, and add a short "Result" note (what actually happened, any deviations from the plan and why, any follow-ups discovered for later steps).
- Update the plan's `README.md` step table: mark this step `done`.
- If you discovered something during this step that affects a later step (new open question, a dependency that no longer holds, scope that needs to shift), add it to that later step's "Open questions / ambiguities" section rather than silently pressing ahead into it.
- Report to the user what was done and what the next pending step is. **Stop here** — do not automatically start the next step. The user calls this skill again to continue.

## Guardrails

- One step per invocation, always. If the user asks to "run the whole plan", explain that steps execute one at a time (each needs its own plan-mode check-in) and start with the first pending one.
- Never mark a step `done` if its acceptance criteria aren't actually met.
- If a step turns out to be blocked (missing dependency, decision the user needs to make outside your authority, external resource unavailable), set its status to `blocked` with a note explaining why, instead of forcing an implementation.
