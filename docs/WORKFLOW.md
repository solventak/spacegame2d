# Work Item Workflow

## Linear states

| State | Meaning |
|---|---|
| Backlog | Captured, but not yet evaluated |
| Triage | Awaiting classification, priority, or ownership |
| Discovery | Requirements or technical approach are unclear |
| Ready | Defined well enough to implement |
| In Progress | Actively being implemented |
| In Review | Awaiting code or product review |
| QA | Being validated against acceptance criteria |
| Ready to Deploy | Approved and awaiting release |
| Done | Released and verified |
| Canceled | Intentionally stopped |

Use the closest existing Linear state if the workspace names differ. Do not create duplicate states without an explicit team decision.

## Required issue information

Implementation issues should include:

- desired outcome
- requirements and scope
- acceptance criteria
- out-of-scope items
- dependencies and risks
- verification plan
- owner and priority when known; estimates and cycles are optional

The Linear ticket's `## Plan` section should point to the approved Notion implementation plan. Do not duplicate the full plan in the ticket.

## Safe agent behavior

Agents may read Linear context and prepare drafts automatically. Before writing to Linear, they must show the proposed changes and receive confirmation for:

- creating issues or projects
- changing ownership, priority, estimate, cycle, or state in bulk
- adding comments on behalf of a person
- archiving or deleting work

After a write, report the affected issue identifiers and links and verify the resulting state.
