# Record performed sets as ordered exercise portions

The recorded-performance model below is confirmed. Implementation remains deferred.

A performance means a performed set. A set contains one or more ordered set portions, each referencing an exercise and using its primary measurement mode; it does not require an overall exercise defining the combination. This supports ordinary sets and partially completed complexes without creating an exercise for every composition or order. Consecutive repetitions may share a portion, with notes describing individual attempts when needed; unsuccessful repetitions do not require separate portions.

One set uses a shared, unchanged load setup, with optional kilograms and load description recorded once for the set. Load conventions remain exercise-specific, and portions within one set need not share a convention: each interprets the shared value using its exercise's convention. Unknown kilograms remain unspecified. Users explicitly distinguish a multi-exercise set from separate sets grouped as a superset; identical equipment or load does not determine that distinction. A failed movement does not itself end a set: continuing from a retained position may remain within it, while resetting and restarting the sequence begins a new set. Drop sets are outside the MVP.

Warmup and main are the only set types. Optional RPE describes the whole set and accepts whole or half-point values from 1 to 10. Optional whole-set white and red flag counts total three; a majority of red flags indicates a failed judgment. Physical completion is independent of judgment, and reaching muscular failure is recorded only in a note. Metrics do not require configuration on exercises.

Main is the default set type. Repetition counts include unsuccessful attempts: eight completed repetitions followed by an unsuccessful ninth may be recorded as nine reps with a note identifying the failed ninth. Duration and distance record actual activity without comparing it to a target or inferring failure. Physical completion, unsuccessful attempts, and pressouts are described in notes only, with no structured completion field. Failure sets remain future work.

Every set portion requires its primary measurement. Explicit zero is valid and distinct from an absent record. Repetition counts are nonnegative whole numbers; duration and rest use nonnegative seconds, and distance uses nonnegative metres, with fractional seconds and metres allowed. Secondary measurements remain notes.

Workouts require a start date and optionally record start and end times. They preserve a user-recorded sequence of sets independently of superset membership. Superset sets need not be contiguous, and recording order does not establish execution order. Explicit rest durations describe actual rest between successive sets in this sequence, disregarding equipment-change time; they are not inferred from elapsed gaps or intervals between sets of the same exercise.

Unilateral repetition counts record only the minimum recorded count on the left and right sides for the MVP, using the convention that repetition counts include unsuccessful attempts. There is no structured side field; notes may preserve individual side counts.

Unknown rest remains unspecified; explicit zero means no rest. Rest describes transitions only, with none before the first or after the last set. A workout may contain one set, but a superset requires at least two. Supplied workout timestamps retain dates and timezone offsets, including the end date for cross-midnight sessions. Unknown times are not invented.

Supersets preserve user-designated grouping of actual sets within one workout without requiring paired rounds, equal set counts per exercise, or strict alternation. A set belongs to at most one superset, and a superset may include multi-exercise sets; supersets do not nest. For example, three bench sets and two row sets may belong to one superset. Circuits and EMOMs are deferred. This resolves the set and workout organisation deferred in ADR 0001. Prescription, programming, planned sets, workout templates, and implementation remain outside this design session.
