# Teaching notes

## Background assessment (2026-10-06)
- Haskell and Ruby experience; user directed us to github.com/archbung for project examples. Public repository metadata includes haskellet, tamarin-macro, and stonks; metadata does not establish authorship depth or mastery.
- Comfortable with Git, terminal commands, SQL, and unit tests; other automated testing experience is uncertain.
- Rust experience: hello world and compiling Precision multiple times.
- Target lesson length: 30 minutes. Use familiar pattern matching as a bridge; confirm transfer through predictions rather than reteaching general programming.

## Teaching preferences
- Predict behavior, explain code, and attempt changes before solutions.
- Offer one hint at a time; request the learner's next attempt before revealing a solution.
- Start later sessions with recall of a previously demonstrated concept; revisit it in a different context after a gap. Do not mark exposure as mastery.
- Distinguish existing design choices, demonstrated correctness problems, style preferences, and measured performance findings. Attach evidence to each claim.
- Use official Rust documentation for language explanations. Consult dependency documentation when a later lesson requires it.
- Keep domain vocabulary in the root GLOSSARY.md and respect docs/adr/. Rust syntax references live here; do not create a competing domain glossary.

## Resuming
Read MISSION.md, README.md, these notes, and learning-records/ first. Read the current milestone and its GitHub comments before teaching. Capture only demonstrated understanding or explicitly stated prior knowledge in learning records. Close milestones only after their observable demonstrations; use issue comments to link evidence.

## Engineering discoveries
Record a bounded observation with code location, category, evidence, and uncertainty. Create a separate engineering issue when justified, following docs/agents/issue-tracker.md and applicable triage instructions. Link it to the learning milestone without expanding the lesson into implementation work.

## Current progress and next lesson
- Lesson 0001 core exercise is complete; see learning records 0003 and 0004 and `scratch.rs` for the repaired learner exercise.
- GitHub evidence: https://github.com/archbung/precision/issues/16#issuecomment-6007687999. Milestone #16 remains open.
- Next lesson: the borrowed string returned by `Measurement::as_str`, `&'static str`. Start with brief recall of explicit arms versus wildcard coverage, then focus on what the returned reference points to and why returning a string literal is valid.
- Aim for 30 minutes, drawing on Haskell/Ruby experience. Verify explanations against official Rust docs; create a numbered HTML lesson with the shared stylesheet. Keep `self`/Copy, trait implementations, and Result for separate lessons.
- Use learner predictions and a scratch compiler experiment contrasting a literal reference with returning a reference into a local String. Give hints before solutions and leave edits to the learner. Do not claim that `'static` means the reference variable must remain alive forever.
