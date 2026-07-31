# Software Development Lifecycle

This repository uses a lightweight, outcome-focused lifecycle. Linear tracks work state; Notion is the canonical store for implementation plans, as described in [`AGENTS.md`](../AGENTS.md).

## Lifecycle

```text
Idea → Triage → Discovery → MVP Definition → Ready →
In Progress → Review → QA → Ready to Deploy → Released → Learn
```

An issue may move backward when new information changes its scope or acceptance criteria.

## Stages and outputs

| Stage | Purpose | Output |
|---|---|---|
| Idea | Capture a problem, request, bug, or opportunity | Initial Linear issue |
| Triage | Decide whether the work is valid, urgent, and owned | Priority, owner, and next action |
| Discovery | Clarify users, constraints, risks, and technical approach | Discovery notes and open questions |
| MVP Definition | Define the smallest valuable outcome and non-goals | Approved MVP scope |
| Ready | Break work into actionable issues with clear acceptance criteria | Ready issues |
| In Progress | Implement and test the change | Working code |
| Review | Validate the implementation and acceptance criteria | Review decision |
| QA | Run automated and relevant interactive verification | QA result and evidence |
| Ready to Deploy | Confirm release notes, rollout, and dependencies | Release candidate |
| Released | Ship and verify the change | Production result |
| Learn | Review outcomes, feedback, and process | Follow-up work or improvement |

## Ceremonies

- **Standup:** inspect active work, blockers, and next actions.
- **Triage:** classify new work, reject duplicates, assign ownership, and set priority.
- **Refinement/grooming:** clarify scope, split issues, identify dependencies, and confirm readiness.
- **Work selection:** choose Ready issues based on current priority and capacity; velocity is not a process goal.
- **Review/demo:** validate delivered behavior against the intended outcome.
- **Retrospective:** identify one or two concrete process improvements.

The team may use Scrum, Kanban, Scrumban, or a custom cadence. The lifecycle stays stable; ceremonies and cycle boundaries are configurable.

## Operating principles

1. Every active issue has an owner and a next action.
2. Every implementation issue has testable acceptance criteria.
3. Keep issues small enough to complete independently within one cycle when practical.
4. Record scope and architectural decisions before implementation begins.
5. Do not mark work Done until the change is verified and Linear reflects the final state.
