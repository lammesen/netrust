# Pilot Rollout Plan

## Objectives
- Validate end-to-end workflow (inventory import → approvals → transaction → execution → telemetry) with real users.
- Gather feedback on CLI ergonomics, GUI usability, and plugin/install experience.
- Identify gaps before GA (service mode).

## Participants
- 3–5 network engineers from target teams (one per region).
- 1 SRE acting as pilot coordinator.

## Timeline
| Week | Activities |
|------|------------|
| Week 1 | Onboard users, share Quick Start, collect environment requirements. |
| Week 2 | Run supervised jobs (canary + batches) on lab gear, enable telemetry dashboards. |
| Week 3 | Autonomous runs + feedback survey, aggregate issues, prioritize fixes. |

## Success Metrics
- ≥2 successful production-like job executions per participant.
- Approval + notification flow exercised end-to-end.
- No critical blockers (P0/P1) open at end of pilot.

## Feedback Loop
- Capture issues/requests in tracker labeled `pilot`.
- Weekly sync to review progress.
- Post-mortem document summarizing wins, gaps, and next milestones.

