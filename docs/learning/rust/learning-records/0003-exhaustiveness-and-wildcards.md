# Exhaustiveness checks coverage, not the intended mapping

The learner correctly predicted Measurement::Duration.as_str(), rejection of a missing explicit variant arm, and acceptance of an incorrect string output. They also correctly predicted that a wildcard permits a future variant and maps a hypothetical Energy variant to "distance", choosing explicit arms to force reconsideration of the mapping.

Evidence: the learner ran their scratch exercise, supplied rustc E0004 naming Measurement::Distance as uncovered, identified that message as the missing-case evidence, restored the arm, and supplied successful compilation. Code-reading predictions and compiler verification are demonstrated; later recall remains pending. Milestone #16 remains open for borrowed strings, traits, Result, and its remaining demonstrations.
