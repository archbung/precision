# Preserve workout intentions separately from performance

The prescription model below is confirmed. Implementation remains deferred.

A workout routine describes reusable intended activity, and a prescribed set means one intended set. Intended targets and recorded performances remain independently available: actual activity does not overwrite intention, because discrepancies are useful information. This continues ADR 0002's distinction between prescribed and performed sets without changing the confirmed recorded-performance model.

Workouts need not follow a routine. A user may begin without prescribed activity and save a performed workout as a routine for future use under the conversion rules below. Warmup sets may be prescribed or added during performance. Programming remains future work.

Each workout preserves its own copy of the intended activity. Later changes to the reusable routine do not change earlier workout intentions, and today's intention may be deliberately edited without changing the reusable routine. Actual performance never overwrites either intention automatically.

There is no set-level mapping between prescribed and performed sets. The user wants to compare intended activity with actual activity, such as five sets of five versus four sets of five, without identifying which particular intended set was skipped. Compare intention and performance side-by-side, including notes, grouped by exercise and set type across the workout. Repeated appearances of the same exercise and set type belong to the same comparison group; individual targets and actual values remain visible.

Prescribed repetitions mean successful repetitions, whereas ADR 0002's recorded repetitions count attempts, including unsuccessful attempts. Prescriptions do not require prescribing failure. Numeric recorded counts alone therefore cannot establish whether successful-repetition targets were achieved; unsuccessful attempts remain notes under the confirmed recorded model. Side-by-side presentation preserves this distinction without automatically declaring target achievement.

When reusing a workout that followed a routine, the default is to use the current reusable source routine, rather than the workout's preserved intention or actual activity. The user may explicitly replace that source routine using just-performed activity or create a new routine instead, under the conversion rules below. These actions do not revise the completed workout's preserved intention. Programming, including rules for autoregulation, remains future work.

Deleting a source routine preserves associated workouts' intentions and recorded performances. Reusing the current source routine becomes unavailable once it is deleted; the preserved workout remains available for creating a new routine under the conversion rules below.

Routines are ordered sequences of prescribed sets, each containing one or more ordered prescribed set portions. Each portion references an exercise and uses its fixed primary measurement mode. This supports prescribed complexes without defining a separate complex exercise. Each prescribed set has one shared, unchanged intended load setup, an optional kilogram target and load description, and a warmup or main set type, defaulting to main. Load changes require separate prescribed sets; drop sets remain deferred. These are explicitly adopted prescription rules, not a blanket inheritance of every recorded-performance rule.

The MVP uses single numeric bounds rather than ranges: repetition targets mean at least the specified number of successful repetitions, duration and distance targets mean at least the specified quantity, and kilogram targets mean at least the specified signed load. Thus an assisted pull-up minimum of -20 kg permits -10 kg, zero, or positive added load but excludes -30 kg. Each portion interprets the shared load using its exercise's convention. RPE targets mean at most the specified whole-set RPE, using the recorded scale of 1–10 in whole or half-point increments. Percentages remain deferred to programming.

Primary measurement targets, kilograms, and RPE may be unspecified; omission means no numeric instruction, distinct from zero. Load descriptions may prescribe a setup with unknown kilograms. An unspecified primary target is permitted in a prescribed portion without relaxing ADR 0002's requirement for an actual measurement in every recorded portion. More elaborate instructions may be expressed in notes.

Optional prescribed rest is a minimum duration between successive prescribed sets, with none before the first or after the last. Zero imposes no minimum; an unspecified duration provides no instruction. Intended and actual rest are shown within their respective sequences, without mapping transitions across extra or omitted sets.

Unilateral repetition targets specify the minimum successful repetitions on each side. There are no structured side-specific targets in the MVP; asymmetric instructions may be notes. Actual unilateral counts retain ADR 0002's minimum of attempted counts on both sides.

Prescribed supersets group at least two prescribed sets independently of sequence, with at most one group per set and no nesting. They require neither paired rounds, equal counts, strict alternation, nor contiguous membership. Intended and actual grouping are independent, without group correspondence. Circuits and EMOMs remain deferred.

When explicitly creating or replacing a routine from actual activity, copy set order, portions, set types, load setup, superset grouping, and notes for review. Propose recorded primary measurements as editable targets before saving; recorded attempts are not inferred to be successes, including by interpreting failure notes. Omit judging flags and leave RPE and rest targets unspecified by default, since observations would otherwise become maximum exertion and minimum rest instructions. The user reviews and can adjust these proposed future targets.

An empty working intention is allowed when starting a workout, but a saved reusable routine requires at least one prescribed set. A saved recorded workout still requires at least one performed set under ADR 0002; starting a workout does not assert performed activity. Prescribed order communicates intended order without constraining actual activity. Both sequences are preserved independently, and comparison does not judge order compliance.

Repetition targets are nonnegative whole numbers. Duration, distance, and rest targets are nonnegative seconds or metres, permitting fractions. Total external load targets are nonnegative kilograms; added bodyweight load targets permit signed kilograms, and both permit fractions. RPE remains 1–10 in half-point increments. Zero measurement targets impose no minimum quantity, while unspecified targets give no numeric instruction.
