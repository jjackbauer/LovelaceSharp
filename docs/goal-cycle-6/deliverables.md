# Deliverables — cycle-6

`ART` rows register artifacts with **at least three** supporting IDs and a `Verified by` command.
An artifact with fewer is a TODO, not a deliverable.

| ID | Artifact | Path | Type | Supporting evidence | Confidence | Verified by |
|---|---|---|---|---|---|---|
| ART-001 | Cycle-6 goal contract (D0–D5, constraints, stop criteria, round plan) | `docs/goal-cycle-6/goal.md` | document | EVD-237, EVD-238, EVD-239 | High | written by me in round 1; every dimension names its command and expected observable |
| ART-002 | Linux CI reproduction environment (WSL2 Ubuntu 24.04 + .NET 10.0.401 + repo copy at `~/ls`) | `docs/goal-cycle-6/wsl-setup.sh`, `linux-ci-loop.sh`, `linux-full-job.sh`, `linux-console-check.sh`, `linux-pin-control.sh` | test-suite | EVD-239, EVD-240, EVD-244 | High | ran the loop in WSL and read the per-project exit codes; the failure reproduced at the same loop position and elapsed budget as the runner |
| ART-003 | CI failure diagnosis (job, step, test, message, root cause) | `docs/goal-cycle-6/evidence.md` EVD-237…EVD-241; `journal.md` OBS-001…OBS-005, VAL-001, DEC-001 | report | EVD-237, EVD-238, EVD-239, EVD-240, EVD-241 | High | three independent sources agree: the GitHub jobs API, the Linux reproduction, and the product sources the renderer reads |
| ART-004 | **The CI fix** — the two stale `solve` help pins re-pinned to the product's declared text, on both the summary and the return kind | `Lovelace.Console.Tests/ReplOutputTests.cs` (commit `98a9049`) | implementation | EVD-240, EVD-241, EVD-244, EVD-245 | High | ran the suite on Windows (15/0) and Linux with coverage (15/0); built the 2×2 control showing old-test/old-product passes and new-test/old-product fails at `ReplOutputTests.cs:67`; CI run #28 green |
| ART-005 | Round-1 falsification reports (two independent Falsifiers, identical prompt, unanimous `Supported`) | `docs/goal-cycle-6/round-1/falsify-A.md`, `falsify-B.md`, `implementation.md` | report | EVD-240, EVD-244, EVD-245 | High | opened both tables in full, checked their citations against the sources, and reproduced the decisive control myself |
