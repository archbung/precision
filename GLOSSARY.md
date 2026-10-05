# Exercise tracking

The language used to describe exercises, workout routines, and recorded performances.

## Language

**Workout routine**:
A reusable ordered sequence of one or more prescribed sets that a user can open to guide recording what they actually perform.

**Prescribed set**:
One intended unit of exercise activity containing one or more ordered prescribed set portions, with a shared, unchanged intended load setup and a warmup or main set type. Its optional targets remain available independently of what is actually performed.
_Avoid_: Planned set

**Prescribed set portion**:
An ordered part of a prescribed set referencing one exercise and optionally specifying a minimum target in that exercise's fixed primary measurement mode.

**Prescribed repetition count**:
The minimum intended number of successfully completed repetitions, distinct from the recorded repetition count of attempts.

**Prescribed load**:
An optional minimum load in kilograms shared across a prescribed set's portions, interpreted using each exercise's load convention. For signed added bodyweight load, a negative minimum limits how much assistance is intended.

**Prescribed RPE**:
An optional maximum intended rating of perceived exertion for a prescribed set as a whole, using the 1–10 scale in whole or half-point increments.

**Prescribed rest duration**:
An optional minimum intended rest duration between successive prescribed sets, with none before the first or after the last. Zero imposes no minimum; an unspecified duration provides no numeric instruction.

**Prescribed superset**:
A user-designated grouping of at least two prescribed sets within a workout routine or a workout's preserved intention, independent of their sequence. Each prescribed set belongs to at most one such group, without nesting or requirements for paired rounds, equal counts, strict alternation, or contiguous membership.

**Exercise**:
A user-defined, named activity with a fixed primary measurement mode and load convention. Its name communicates technique, and different equipment types distinguish exercises.

**Measurement mode**:
The quantity used to record an exercise's performance: repetitions, duration, or distance.

**Load convention**:
The fixed meaning of an exercise's optional load in kilograms: total external load or signed added bodyweight load.

**Total external load**:
The combined external weight used in an exercise, including the barbell, sled, or both dumbbells when applicable.

**Added bodyweight load**:
Load added to or assistance provided against bodyweight: positive kilograms denote added weight, zero denotes no added weight or assistance, and negative kilograms denote assistance.

**Load description**:
An optional description of a performance's resistance or assistance setup, such as a purple band, including when its load in kilograms is unknown.

**Performance**:
A performed set, as distinct from a prescribed or planned set.

**Set**:
A user-delimited unit of performed exercise activity containing one or more ordered set portions with a shared, unchanged load setup and optional load in kilograms and load description. Each portion interprets the load using its exercise's convention; unspecified kilograms remain unknown, and restarting after a failed attempt begins a new set.

**Set type**:
The classification of a performed set as warmup or main, with main as the default.

**Set portion**:
An ordered part of a performed set that references one exercise and records execution using that exercise's primary measurement mode. Consecutive repetitions may share a portion, with notes describing individual attempts when needed.

**Unilateral repetition count**:
The minimum recorded repetition count on the left and right sides, rather than their sum.

**Repetition count**:
The number of repetitions attempted, including unsuccessful repetitions. A note may identify an unsuccessful repetition.

**Rest duration**:
The explicitly recorded duration of rest between successive sets in a workout's sequence, disregarding equipment-change time. Zero denotes no rest; an unspecified duration is unknown.

**Superset**:
A user-designated grouping of at least two separate performed sets within one workout, without requiring paired rounds or equal set counts for its exercises. A set belongs to at most one superset; its sets may use the same or different load setups.

**Set metric**:
An optional observation recorded for a performed set, without requiring metric configuration on its exercise.

**RPE**:
An optional numeric rating of perceived exertion for a performed set as a whole, from 1 to 10 in whole or half-point increments.

**Judging flags**:
Optional white-flag and red-flag counts judging a performed set as a whole, totalling three. More red flags than white flags indicate a failed judged attempt, independently of physical completion.

**Workout**:
A session of one or more performed sets whose boundary is independent of calendar dates, with a required start date, optional start and end times, and a user-recorded sequence of sets.

**Equipment type**:
A category of equipment from the supplied catalog that defines an exercise's loading setup, rather than every item needed to perform it. An exercise may reference none or several; cable and machine are distinct equipment types.

**Muscle group**:
A group of muscles that an exercise may optionally reference.

**Primary muscle group**:
The optional muscle group identified as the main target of an exercise. Fullbody denotes a whole-body target.

**Secondary muscle group**:
A muscle group identified as supporting an exercise rather than being its main target. An exercise may have several secondary muscle groups, none of which is also its primary group.
