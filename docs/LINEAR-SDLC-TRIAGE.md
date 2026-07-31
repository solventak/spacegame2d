# Linear SDLC Triage

This repository uses repository-backed, outcome-focused triage for the Swarm (`SWA`) Linear team. The goal is to make issues clear enough for implementation without requiring estimates or velocity tracking.

## Working agreement

- Inspect the repository, relevant tests, and project guidance before making a recommendation.
- Read the complete Linear issue, including status, priority, ownership, cycle, estimate, project, milestone, and relations.
- Ask one decision question at a time.
- Estimates are optional and are not part of this workflow. Velocity is not a process goal.
- Keep each ticket’s scope explicit. Split only when ownership, delivery, or acceptance boundaries materially differ.
- Do not change Linear without explicit user approval.
- Once a proposed change has been approved, apply it directly and report exactly what changed. Preserve unrelated fields.

## Triage sequence

For every ticket, document:

1. Desired outcome.
2. Current repository truth: what exists, what is missing, and what is only planned.
3. Scope, non-goals, dependencies, and risks.
4. Acceptance and verification needs.
5. Readiness for the next lifecycle stage.

Use the team’s closest available states. The normal progression is:

```text
Backlog → Discovery → Needs Refinement → In Progress → Review → QA → Done
```

Use `Needs Refinement` when the outcome, boundaries, acceptance criteria, and major technical decisions need one final implementation-focused pass. It does not imply an estimate or planning-poker step.

## Compatibility and shared-code changes

Protocol and simulation compatibility versions are the compatibility contract between server and client. Agents must bump the relevant version whenever a shared wire contract or deterministic simulation behavior changes. Do not use Git branch names or commit SHAs as the compatibility gate unless the team explicitly changes this policy.

When a ticket concerns compatibility, distinguish clearly between:

- rejecting an incompatible connection;
- allowing the connection while logging a diagnostic; and
- documenting a development-only warning.

## Overlapping work

When tickets overlap, keep ownership and acceptance boundaries explicit rather than silently merging them. For example, current authoritative-state delivery for late joiners belongs with snapshot work; shared simulation/reset defines the common deterministic scenario and reset behavior.

## Handoff

After an approved Linear mutation, report the affected issue identifier, resulting state, and untouched fields. Keep implementation plans in the canonical planning system referenced by `AGENTS.md`; do not duplicate full plans in Linear or this document.
