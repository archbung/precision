# Learn Rust through Precision

[Parent goal: #15](https://github.com/archbung/precision/issues/15). GitHub Issues track completion; this directory holds teaching state and evidence. GitHub file links in issues become available once these artifacts are pushed.

## Start here

Read the [mission](MISSION.md) and [background/preferences](NOTES.md). Start [lesson 0001: exhaustive match](lessons/0001-exhaustive-match.html), aimed at a Haskell/Ruby programmer beginning Rust. Budget 30 minutes. Predict first, attempt the scratch experiment yourself, and return your explanation for feedback.

## Current progress

Lesson 0001’s core exercise is complete: the learner demonstrated missing-arm rejection, restored compilation, a failing mapping assertion, and a repaired passing execution. See learning records [0003](learning-records/0003-exhaustiveness-and-wildcards.md) and [0004](learning-records/0004-coverage-versus-behavior.md). Next: borrowed strings in `as_str()` (`&'static str`). Later recall and independent test design remain pending.

## Initial milestones

| Order | Goal | Demonstration | Issue |
| --- | --- | --- | --- |
| 1 | Exercise types: enum/match, traits, borrowed strings, Result | Predict outputs, explain signatures, reproduce a compiler diagnostic, and write a contract check | [#16](https://github.com/archbung/precision/issues/16) |
| 2 | Ownership, borrowing, validation, errors | Explain ownership at a call boundary, repair a borrow/move error, and trace rejected input | [#17](https://github.com/archbung/precision/issues/17) |
| 3 | One CLI command through SQLite and integration tests; one small change | Produce a trace, write a failing behavior test, implement the change yourself, and explain the passing diff | [#18](https://github.com/archbung/precision/issues/18) |
| 4 | Representative command assessment and realistic measurement | Classify bounded findings, compare tradeoffs, and produce a reproducible performance baseline | [#19](https://github.com/archbung/precision/issues/19) |

Milestones can contain several lessons, each about one concept. Progress follows demonstrated understanding rather than a fixed calendar. No milestone is complete yet. Each issue contains detailed observable completion criteria.

## Adapted /teach workspace

- [MISSION.md](MISSION.md): concrete purpose and boundaries.
- [RESOURCES.md](RESOURCES.md): annotated official sources and optional community.
- [NOTES.md](NOTES.md): background, preferences, and how to resume.
- `lessons/`: numbered HTML lessons, using `assets/course.css`.
- `reference/`: printable syntax references; start with [enum and match](reference/enum-match.html).
- `learning-records/`: numbered records of demonstrated understanding or stated prior knowledge. Coverage is not mastery.
- `assets/`: reusable presentation components. Check these before making another lesson.

Keep Precision domain terms in the root [GLOSSARY.md](../../../GLOSSARY.md); do not introduce another domain glossary. Add learned Rust terms to reference material only after the learner demonstrates understanding. Use scratch files under this directory for isolated compiler exercises; exercises are deliberately unimplemented at setup.

## Feedback and completion

Each lesson follows prediction → learner attempt → hint → revised attempt → explanation. Return answers in conversation or save them here and link evidence in the milestone. Start later sessions with recall, then transfer a familiar concept to new code. Close a milestone only when its demonstrations have been observed; the teacher's implementation does not count.

Treat existing code as evidence to examine. Keep design choices, correctness problems, style preferences, and measured performance findings distinct. Track justified engineering improvements in separate issues; do not fold a broad refactor into this learning track.

Initial source snapshot: `71461a0671b37767421da0cbe0b6f97728fddab0`. Recheck examples when production code changes.
