# Deliverables — cycle-6

`ART` rows register artifacts with **at least three** supporting IDs and a `Verified by` command.
An artifact with fewer is a TODO, not a deliverable.

| ID | Artifact | Path | Type | Supporting evidence | Confidence | Verified by |
|---|---|---|---|---|---|---|
| ART-001 | Cycle-6 goal contract (D0–D5, constraints, stop criteria, round plan) | `docs/goal-cycle-6/goal.md` | document | EVD-237, EVD-238, EVD-239 | High | written by me in round 1; every dimension names its command and expected observable |
| ART-002 | Linux CI reproduction environment (WSL2 Ubuntu 24.04 + .NET 10.0.401 + repo copy at `~/ls`) | `docs/goal-cycle-6/wsl-setup.sh`, `docs/goal-cycle-6/linux-ci-loop.sh`, `docs/goal-cycle-6/linux-full-job.sh` | test-suite | EVD-239, EVD-240 | High | ran the loop in WSL and read the per-project exit codes; the failure reproduces at the same loop position as the runner |
| ART-003 | CI failure diagnosis (job, step, test, message, root cause) | `docs/goal-cycle-6/evidence.md` EVD-237…EVD-241; `journal.md` OBS-001…OBS-005, VAL-001, DEC-001 | report | EVD-237, EVD-238, EVD-239, EVD-240, EVD-241 | High | three independent sources agree: the GitHub jobs API, the Linux reproduction, and the product sources the renderer reads |
