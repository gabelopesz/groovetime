---
name: multi-step-planner
description: Creates a multi-step implementation plan for a feature, refactor, migration, or any non-trivial coding task, and writes it to disk as a plan directory with a README.md index and one file per step. Make sure to use this skill whenever the user asks to "make a plan", "break this into steps", "plan out" a feature, wants a roadmap/checklist before coding starts, or asks for something to be "planned" before implementation — even if they don't explicitly say "multi-step plan". This skill only produces the plan; it does NOT implement any step. Use the companion "execute-plan-step" skill to actually execute steps from a plan produced here.
---

# Multi-Step Planner

Turns a task into a written, on-disk plan: a `README.md` index plus one file per step. The plan is the contract that the `execute-plan-step` skill later reads and executes one step at a time — so the quality of this skill's output directly determines how well execution goes. Write for a future Claude that has NO memory of this conversation.

## When to use this

Use whenever the task is non-trivial enough that jumping straight into code would be risky: multiple files/modules touched, unclear ordering, unclear scope, or the user explicitly wants a plan before implementation. Don't use it for a single quick edit — just do that directly.

## Workflow

### 1. Understand the goal and investigate

Before writing anything, make sure you actually understand the codebase and the goal:

- Read the relevant parts of the repo (search for related files, existing patterns, conventions, tests).
- Identify constraints: frameworks in use, existing architecture, things that must not break.
- If the request is ambiguous in a way that would change the shape of the plan (e.g., "add auth" — which provider? session or token based?), ask the user now. Don't guess on decisions that are expensive to reverse. Small implementation details can be left as open questions inside the relevant step instead of blocking here.

### 2. Break the work into steps

Good steps are:

- **Independently executable** — a step should be completable and verifiable (tests pass, feature works) without requiring unstated knowledge from other steps beyond its declared dependencies.
- **Small enough to review** — roughly "one sitting" of work. If a step feels like it's actually 3 steps, split it.
- **Ordered by dependency, not just convenience** — foundational/shared pieces (schema, types, config) before things that build on them.
- **Scoped explicitly** — each step says what's in scope and what's explicitly NOT in scope, so the executor doesn't scope-creep into later steps.

Aim for the minimum number of steps that keeps each one reviewable — don't over-fragment trivial work, and don't cram unrelated concerns into one step.

### 3. Choose a plan directory

Default location: `plans/<slug>/` at the repo root, where `<slug>` is a short kebab-case name for the task (e.g. `plans/add-stripe-billing/`). If a `plans/` directory (or similar, e.g. `docs/plans/`) already exists in the repo, use that location's convention instead of inventing a new one.

Structure:

```
plans/<slug>/
├── README.md              # index: goal, step table, how to execute
└── steps/
    ├── step-01-<slug>.md
    ├── step-02-<slug>.md
    └── ...
```

### 4. Write the step files

Use `assets/step-template.md` as the structure for each step file. Read it now before writing steps. Every step file must be self-contained enough that a fresh Claude session with no other context could read just that one file (plus the README for overall goal) and know what to do.

Critically, each step file must include an **"Open questions / ambiguities"** section — even if empty. This is what the `execute-plan-step` skill uses to enter plan mode and interrogate the step before touching code. If you already know of an ambiguity while planning, write it down there instead of silently deciding for the user.

### 5. Write the README.md index

Use `assets/plan-readme-template.md` as the structure. It must include:

- The overall goal and any important context/decisions from step 1.
- A step table with columns: `#`, `Step`, `Status` (`pending` / `in-progress` / `done` / `blocked`), `Depends on`.
- All steps start as `pending`.
- A short note pointing at the `execute-plan-step` skill for execution, so future sessions know how this plan gets carried out.

### 6. Confirm with the user

Show the user the step table (names + one-line summaries) before considering the task done. Plans are cheap to change now and expensive to change once execution has started — briefly surface any assumptions you made in step 1. Ask if the breakdown looks right, but don't block on approval if the user just asked you to "make the plan" — writing the files IS the deliverable.

## Updating an existing plan

If the user asks to add/remove/reorder steps in an existing plan: renumber files only if necessary (prefer inserting as e.g. `step-02b-...md` over a full renumber if execution may already be in progress), update the README table, and preserve the `Status` of untouched steps.

## Notes

- Never write implementation code from this skill — that's `execute-plan-step`'s job. This skill only produces planning documents.
- Keep step files in plain Markdown, no exotic formatting — they need to be easy for both humans and future Claude sessions to parse.
