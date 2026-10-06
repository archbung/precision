# Precision

A local CLI for exercise definitions and reusable workout routines, backed by
SQLite. Build with `cargo build`;
run `cargo run -- --help` or use `target/debug/precision`.

New databases start with 60 exercises captured in migration 0008, including their
equipment, primary muscles, and secondary muscles. Existing databases keep their
exercise catalog unchanged. Starter exercises can be edited like user-created ones.

The default database is `$HOME/.precision/precision.sqlite3` on every platform.
All commands accept `--db PATH`, before or after subcommands. Parent directories
are created on first use. To keep a separate catalog:

```sh
precision --db ./training.sqlite3 catalog equipment
precision --db ./training.sqlite3 catalog muscles
precision --db ./training.sqlite3 exercise create \
  --name 'Barbell squat' --measurement repetitions --load-convention external \
  --equipment 1 --primary-muscle 9 --secondary-muscle 8
precision --db ./training.sqlite3 exercise list
precision --db ./training.sqlite3 exercise show 1 --json
precision --db ./training.sqlite3 exercise update 1 --name 'Back squat'
```

## Commands

- `catalog equipment` and `catalog muscles` print the supplied stable IDs and
  names. These catalogs cannot be extended by CLI users. No exercises are seeded.
- `exercise create --name NAME --measurement repetitions|duration|distance
  --load-convention external|added-bodyweight` creates an exercise and prints its
  ID and definition. Optional `--equipment ID` and `--secondary-muscle ID` may be
  repeated; `--primary-muscle ID` accepts one muscle. IDs come from catalog reads.
- `exercise list [--json]` lists definitions in stable ID order.
- `exercise show ID [--json]` reads one definition. Human output is the default.
- `exercise update ID` accepts `--name NAME`, `--primary-muscle ID`, repeated
  `--secondary-muscle ID`, `--clear-primary-muscle`, and
  `--clear-secondary-muscles`. Omitted fields retain their values. Supplied
  secondary muscles replace the entire secondary set. Clearing and setting the
  same field together is rejected. An update requires at least one edit option.

Names are trimmed and must be nonempty and unique under full, non-Turkic Unicode
case folding, including `Straße`/`STRASSE` and Greek sigma variants. Display
spelling is retained. No Unicode normalization is applied. Equipment and
secondary muscle references are sets, so repeated IDs are deduplicated and
exports sort them by ID. Primary and secondary muscles cannot overlap;
secondary muscles do not require a primary muscle.

Measurement, load convention, and defining equipment are creation-only. Create
another exercise for changed definitions. `external` means total external
kilograms, including the barbell/sled/both dumbbells. `added-bodyweight` means
signed added kilograms: positive adds weight, negative denotes assistance, and
zero means no addition or assistance. Exercise commands define activities;
they do not record performances.

## JSON reads

`show --json` produces one object; `list --json` produces an array of the same
objects (an empty catalog produces `[]`). The versioned read schema is:

```json
{
  "schema_version": 1,
  "id": 1,
  "name": "Back squat",
  "measurement": "repetitions",
  "load_convention": "external",
  "equipment": [1],
  "primary_muscle": 9,
  "secondary_muscles": [8]
}
```

An unspecified primary muscle is `null`; absent equipment or secondary muscles
are empty arrays. IDs are durable integer identities, never list positions or
names. Exercise edits use CLI flags; JSON-file exercise imports are not supported.

## Persistence and errors

Versioned SQL migrations live in `migrations/`. Version 1 installs exactly the
catalogs in ADR 0001 and the exercise tables. SQLite `user_version` records the
schema version; migration and catalog installation run in one transaction.
Restarts preserve data and IDs. A newer, unsupported schema version produces an
error instead of changing the schema. Foreign keys are enabled on every
connection. Writes validate references and name uniqueness within an immediate
transaction; failed writes roll back. SQLite waits up to five seconds for a busy
writer before returning an error.

Successful commands exit zero and write data to stdout. Invalid options,
unknown IDs, empty/duplicate names, overlapping muscle references, and storage
failures produce errors on stderr with a nonzero exit status. No partial
exercise changes are saved. Run `cargo test` for integration tests invoking
separate compiled CLI processes against temporary databases.

## Workout routines

Routines are reusable ordered prescriptions. Names are trimmed and nonempty,
with duplicates allowed because IDs distinguish routines.

```sh
precision routine create --file routine.json
precision routine list                 # human output; also accepts --json
precision routine show 1 --json > edited-routine.json
precision routine update 1 --file edited-routine.json
precision routine delete 1
```

Create a repetition exercise and a distance exercise first (use their displayed
IDs instead of the example's `1` and `2`). Save this as `routine.json`:

```json
{
  "schema_version": 1,
  "name": "Squat and carry",
  "notes": "Keep the complex load setup unchanged",
  "sets": [
    {
      "type": "warmup",
      "kilograms": 0,
      "rpe": 5.5,
      "load_description": "empty bar",
      "notes": "Controlled tempo",
      "portions": [{"exercise_id": 1, "repetitions": 5}]
    },
    {
      "kilograms": 20.125,
      "portions": [
        {"exercise_id": 1, "repetitions": 5, "notes": "Successful reps on each side if unilateral"},
        {"exercise_id": 2, "metres": 12.5},
        {"exercise_id": 1, "repetitions": null}
      ]
    }
  ]
}
```

A routine requires at least one set; each set requires at least one portion.
Array order determines set and portion order, including repeated exercises in
a complex. `type` is `warmup` or `main`, defaulting to `main`. Portions require
an existing `exercise_id`. Their optional quantity field is `repetitions`,
`seconds`, or `metres`, matching the exercise's measurement mode. Repetitions
are whole nonnegative minimum **successful** counts (on each side for unilateral
activity). Seconds and metres are nonnegative minima and may be fractional.
Other measurement fields cannot contain a quantity; use notes for secondary
measurements. Numeric values never infer target achievement.

Set `kilograms` is a shared minimum load, interpreted using every portion's
exercise load convention. Negative kilograms are accepted only if all portions
use `added-bodyweight`; mixed conventions are allowed with nonnegative or
unspecified kilograms. Set `rpe` is an optional whole-set maximum from 1 to 10
in half-point steps. Optional `load_description` can prescribe resistance with
unknown kilograms. `notes` is an optional string at routine, set, and portion
levels. Omitted or `null` optional targets mean no numeric instruction; explicit
zero remains zero. `show --json` includes null optional fields and generated IDs.

Create documents omit routine, set, and portion `id` fields (null is also
accepted). Update documents may retain exported IDs belonging to that routine;
the routine ID, if supplied, must match the command destination. New sets and
portions omit IDs. Duplicate or foreign nested IDs are rejected; removed IDs
cannot be resurrected or recycled. Updates replace the complete routine,
including notes and optional values; omission clears optional values instead
of retaining the old values. An update may reorder retained entries, move a
portion between sets within its routine, add entries, or remove entries.
Validation and replacement occur in one transaction, so any failure leaves the
original intact. Deletion removes only the routine and its prescribed children;
it introduces no workout cascade.

All JSON objects reject unknown fields. Judging flags in prescriptions,
structured failure, ranges, and programming data are rejected. Malformed JSON,
unsupported document versions, wrong field types, and invalid references fail
with a nonzero exit status and stderr explanation.

### Exact decimal precision

Supply JSON numbers, not quoted strings. Quantities and kilograms support values
with at most **12 integer places and 6 fractional places** after removing
insignificant zeros; the largest magnitude is `999999999999.999999`, and the
smallest nonzero fractional step is `0.000001`. Exponent notation is accepted
when the exact value fits these bounds (`1e-6` fits; `1e-7` does not). Repetition
quantities must additionally be whole numbers. No value is rounded to fit.
NaN and infinity are invalid JSON and rejected. Decimal spellings are parsed
without binary floating point and stored as text in SQLite; RPE is stored as
integer half-point units. Numeric meaning is preserved on export, though
insignificant spelling differences such as RPE `5` versus `5.0` may change.

Migration version 2 adds routines and ordered prescribed sets/portions with
relational exercise references. Existing exercises and catalogs survive the
migration unchanged. Stable IDs use monotonic SQLite identities, and every
connection enforces foreign keys. CLI integration tests cover restart reads,
replacement order/identity, rejected replacement rollback, decimal limits,
complexes, omitted versus zero quantities, and deletion.

## Standalone workouts

Start a draft, export it, edit its actual activity, and resume by updating the
same ID from any later CLI process:

```sh
precision workout start --date 2026-10-05 --start 2026-10-05T23:50:00+07:00
precision workout list --drafts
precision workout show 1 --json --actual-only > workout.json
precision workout update 1 --file workout.json
precision workout show 1 --json --actual-only > latest.json
# Use the revision in latest.json (1 after the first successful update):
precision workout finish 1 --revision 1 --end 2026-10-06T00:20:00+07:00
precision workout list
```

`workout start` requires `--date YYYY-MM-DD`; `--start RFC3339` is optional.
Start prints the durable workout ID and creates a clearly marked draft with no
performed sets. `workout list [--json]` lists finished workouts in ID order;
`--drafts` includes drafts as well. `workout show ID [--json]` inspects either
state. `workout discard ID --revision REVISION` permanently removes a draft and its activity.
Finished workouts are read-only: update, finish again, and discard fail.
`finish` requires at least one performed set and atomically saves the state and
optional end timestamp; omission retains an end already supplied by update.

Workout mutations use a **transactional revision**. Migration 9 gives existing
workouts revision `0`; each confirmed actual/intention update and finish advances
it by one. Actual exports include `revision`, a nonnegative JSON integer; retain
that value unchanged while editing. `workout update` requires a matching revision,
including when replacing empty activity. Missing, null, negative, noninteger,
and stale values fail: reload using `workout show ID --json --actual-only`, inspect
saved activity, and deliberately reapply an edit to a fresh export. There is no
unconditional overwrite option. Rejected writes change neither activity nor
revision. This deliberately changes the earlier update contract: old documents
without revisions must be re-exported before submitting them.

`finish`, `discard`, and `intention update` require `--revision REVISION` from
`workout show ID --json`. An intention update preserves actual activity while
advancing the same workout revision, invalidating older actual editors. Actual
updates preserve intention and source provenance. Reusable routine revisions
remain separate from workout revisions.

An update replaces the entire session metadata and actual activity. Example
(use an existing repetition exercise ID):

```json
{
  "schema_version": 1,
  "revision": 0,
  "date": "2026-10-05",
  "start": "2026-10-05T23:50:00+07:00",
  "end": null,
  "notes": "Late session",
  "sets": [{
    "type": "main",
    "kilograms": null,
    "load_description": "purple band",
    "rpe": 9.5,
    "white_flags": 1,
    "red_flags": 2,
    "notes": "Judgment is independent of physical completion",
    "portions": [{
      "exercise_id": 1,
      "repetitions": 9,
      "notes": "Ninth attempt unsuccessful; unilateral attempted counts 9/10"
    }]
  }]
}
```

Exports additionally include `id`, `state`, and stable nested set/portion IDs.
`state` may be omitted on update (defaults to `draft`); a supplied state must be
`draft`. The top-level ID, if supplied, must match the destination. Retain IDs
for existing entries; omit them for new entries. Array order controls sequence.
Unknown, foreign, duplicate, or previously removed nested IDs fail. Drafts may
contain zero sets, but each recorded set requires nonempty ordered portions.
Each portion requires exactly one non-null primary measurement matching its
exercise: `repetitions`, `seconds`, or `metres`. Zero is valid; missing/null
primary activity is rejected. Repetitions count nonnegative integral attempts,
including unsuccessful attempts; unilateral counts use the minimum across
sides. Fractional nonnegative seconds/metres are allowed. Describe unsuccessful
attempts and side counts in notes, without structured completion or side fields.

Set types default to `main`, with `warmup` also accepted. Load is shared across
all portions. Unknown kilograms are null/omitted, distinct from zero. Negative
kilograms require every portion to use added-bodyweight load; mixed conventions
are permitted otherwise. Actual RPE is optional, whole-set, 1–10 in half steps.
White/red flags must both be null/omitted or nonnegative integers totaling
three. A red majority is displayed as failed judgment, without inferring
physical completion. Session, set, and portion notes retain their own context.
The exact decimal precision documented above also applies to actual values.

Dates and full offset timestamps retain their supplied spelling, dates, and
zone offsets. The start timestamp's local date must match `date`. With both
times present, the end instant cannot precede the start instant. Without a
start time, the end's supplied local date cannot precede `date`. Cross-midnight
end dates are supported. Missing times remain unknown; no time or rest is
invented. Malformed dates/timestamps fail.

Update rejects intention/source fields and all unknown fields. Intention and provenance are read-only fields in workout exports; remove them
before submitting an actual update, or use `show --json --actual-only`
to export an editable actual update document directly. Validation and replacement run within one
transaction: any error leaves metadata, IDs, state, and performance unchanged.
Migration version 3 adds workouts and independently owned ordered performed
sets/portions with relational exercise references; existing data is preserved.

## Preserved workout intention

`workout start --date YYYY-MM-DD --routine ID` copies the current routine's
complete supported prescriptions and notes into the new draft in one transaction.
Routine-free starts have an empty intention. Each copy owns fresh set/portion
identities; order, types, load setup, optional targets, and all notes are preserved.
Routine edits, intention edits, and actual updates affect only their own aggregate.
Actual measurements never fill unspecified targets or create set correspondence.
Rest transitions and supersets copy with fresh intention-owned identities.

`workout show ID --json` and `workout list --json` include `intention` and source
provenance alongside the actual update fields. To edit prescriptions, extract the
`intention` object into a file (for example, `precision workout show 1 --json |
jq .intention > intention.json`) and submit:

```sh
precision workout intention update 1 --revision CURRENT_REVISION --file intention.json
```

An intention document contains `schema_version: 1`, optional `notes`, `rest`,
`supersets`, and `sets` using the prescribed set/portion schema above; it has no routine name or
aggregate ID. Empty intention is valid:

```json
{"schema_version": 1, "notes": "Spontaneous session", "sets": []}
```

Retain exported intention set/portion IDs for existing entries; omit IDs for new
entries. Foreign, duplicate, or removed IDs fail, and prescriptions use the same
validation and exact decimals as routines without requiring primary targets.
Replacement is atomic and updates only intention, including its own notes.
Finished workouts retain their intention and reject intention updates.

`source_routine_id` is the live, nullable source reference. `original_source_id`
and `original_source_name` preserve the original provenance independently of
source renames/deletion. Deleting the routine clears only the live reference and
removes the routine's children; associated drafts, finished workouts, intention,
and actual activity survive. Creating a routine with the same name never restores
the link. Human workout output shows provenance, prescribed activity, intention
notes, and actual activity in their separate contexts.

Migration version 4 adds independent workout-owned prescription tables and source
metadata. Existing version 3 workouts receive empty intentions and no provenance;
existing routines, exercises, and performances remain intact.

### Reuse a workout's current source

```sh
precision workout reuse 1 --date 2026-10-06 --start 2026-10-06T18:00:00+07:00
```

`workout reuse ID --date YYYY-MM-DD [--start RFC3339]` accepts either a draft
or a finished workout and creates a new draft from its current live source
routine. Source lookup and the complete intention copy run in one transaction,
using the same copy behavior as `workout start --routine ID`. The new draft
has fresh intention identities, current source provenance, and empty actual
activity; prior session notes and times are not copied. Its supplied date and
optional start follow the normal workout timestamp validation.

Reuse reads current prescriptions, including notes, rest, and supersets, even
after source edits or independent adjustments to the historical intention.
It never copies preserved intention or performance. Unknown workout IDs,
routine-free workouts, and deleted sources fail with an error and no new draft.
Deleting a source preserves both draft and finished workout records across
restarts. A routine created under the same name receives a different ID and
does not restore reuse for those records. Direct `workout start --routine ID`
remains available for live routines and rejects missing/deleted IDs atomically.


## Rest transitions and supersets

Routine, intention, and actual documents each accept `rest` and `supersets`.
Exports always include both arrays. For N sets, `rest` contains exactly
`max(N-1, 0)` values describing successive sequence transitions in seconds:

```json
"rest": [0, null, 1.25],
"supersets": [{"id": 7, "set_ids": [11, 13, 14]}]
```

This example describes four sets with a noncontiguous group of three. Supply
actual exported IDs from the same aggregate. Rest has no leading/trailing
value. Nonnegative fractional seconds use the exact decimal precision above.
Prescribed rest is a minimum; zero imposes no minimum and null gives no numeric
instruction. Actual rest is an observation; zero means no rest and null means
unknown. Timestamps and equipment-change time never determine rest.
Omitting `rest` initializes all transitions to null; it clears prior durations
on replacement. Explicit `rest: null` is invalid. Reordering retained sets
requires an explicit complete replacement array, even if all values are null;
old transitions are never remapped. When adding/removing sets, adjust any
supplied array to the new count. Zero/one-set sequences use `[]`.

Each superset has an optional stable `id` and a `set_ids` array of at least two
distinct owner-local set IDs. Each set belongs to at most one group; foreign
IDs, duplicate membership, overlaps, and nesting are rejected. Noncontiguous
sets, unequal exercise counts, repeated exercises, and multi-exercise sets are
allowed. Membership does not change sequence or imply execution order. Groups
have no intended/actual correspondence. Retain exported group IDs on updates;
new groups omit `id`. Omitted `supersets` clears grouping.

The CLI generates IDs. To group newly created sets, first save the sets, export
the aggregate, then add `supersets` referencing its generated set IDs and update.
This also applies when adding new sets to an existing aggregate. Routine copies
remap every group member to the new intention's set identity. Intention and
actual edits affect only their own organization. Routine edits/deletion preserve
workout organization. Human reads label prescribed minima and actual observations
separately and show group IDs and member set IDs.

Migration version 5 adds independent relational rest, superset, and membership
tables for routines, intentions, and actual activity. Owner-scoped foreign keys
prevent cross-owner memberships. Existing sequences receive null rest
transitions and no groups. Validation and persistence run in the aggregate's
transaction; invalid replacement leaves all original data intact.

### Compare intention and performance

Run `precision --db PATH workout compare ID` for a draft or finished workout.
The command is read-only and uses the workout's preserved intention, including
its notes and original routine context; later routine edits do not change it.

Groups are keyed by exercise ID and warmup/main set type, in first intention
appearance order followed by actual-only groups. Each group has independent
prescribed and actual columns, ordered by original set and portion position.
**Visual rows do not pair sets.** Five prescribed sets and four actual sets
remain separate lists; a group missing on one side explicitly says so.

Entries show stable IDs and positions, ordered complex composition, shared
kilograms/load description, whole-set RPE and notes, relevant portion notes,
judging flags, and independent superset membership. Counts distinguish unique
sets from portions; repeated observations for multiple portions still belong
to one shared set. Each exercise's load convention is shown.

Prescribed repetitions are **minimum successful repetitions** (on each side
for unilateral activity); actual repetitions are **attempts**, including
unsuccessful attempts. Prescribed duration, distance, kilograms and rest are
minima; prescribed RPE is a maximum. `unspecified` means no numeric target or
an unknown actual observation, while `0` remains explicit. Notes are displayed
without interpreting them as successful repetitions. Majority-red judging
flags mean failed judgment independently of physical completion.

Separate full intended and actual sequence/rest sections preserve all portions,
notes, transitions, and noncontiguous superset membership. These sequences,
transitions and groups have no inferred correspondence. Comparison does not
identify skipped prescribed sets, declare target achievement, score order,
or estimate equivalence or improvement. An unknown workout ID exits nonzero
with an error on stderr.

## Reviewed workout conversion

```sh
precision routine propose --from-workout 1 --file review.json
# Edit review.json, including routine.name and future targets, then explicitly save:
precision routine create --file review.json --reviewed-from-workout 1
# Or replace the workout's current source routine:
precision routine replace-source --from-workout 1 --file review.json --reviewed
```

Only finished workouts can be converted. Proposal writes a file and does not
save or update a routine. The review envelope has `schema_version: 1`,
`from_workout`, `source_routine_id`, `source_revision`, `review_guidance`, and
`routine`. The nested routine uses the ordinary routine schema. Session notes
become routine notes; actual set/portion order, types, exercise references,
shared load, notes, and supersets are copied. Negative nested IDs are local
review references, remapped to fresh destination identities when saved. Retain
these IDs when editing copied entries; new entries omit IDs. Foreign IDs are
rejected. Supersets reference set IDs within the review's routine.

Recorded repetition values are **attempts**, including unsuccessful attempts;
they are proposed for review as minimum future **successful repetitions**.
Notes are copied verbatim and never parsed to infer success. Seconds, metres,
and signed kilograms propose minimum targets. RPE means a maximum; rest means
a minimum. Both default to null (no numeric instruction), with exactly N−1
null rest transitions. Judging flags are omitted. Zero and null remain distinct.
Review and adjust values, notes, composition, and name before saving.

Both save paths validate the entire prescription and require the explicit
review flag/source option. Ordinary routine CRUD remains independent. Keep
provenance metadata intact: the originating workout and its original source ID
are checked. A routine-free workout has null source ID/revision. A proposal
made after source deletion retains the original ID and has null revision.
New-routine creation remains available after source deletion or source edits.
Replacement keeps the source routine's ID and compares its revision within the
write transaction. Deleted or stale sources fail without partial changes;
generate and review a fresh proposal before trying again. Every ordinary update
and reviewed replacement advances the source revision. Workout intentions and
performance stay unchanged; subsequent reuse reads the replaced current source.

The bundled muscle catalog includes Adductors (ID 13) for hip adduction.
Existing databases receive this entry automatically through migration 0007;
all earlier muscle IDs remain stable.

## Interactive draft lifecycle

Launch `precision tui [--db PATH]` in an interactive terminal (80×24 or larger).
The selected database and current draft ID stay visible. Tab cycles startup
fields: today's local date, optional full RFC3339 start time, and optional routine
ID. Available routines are listed by ID; Up/Down scrolls longer lists. Blank time remains unknown; blank
routine starts without intention. F2 or Enter starts and immediately saves a draft.

F3 lists only drafts; arrows and Enter explicitly resume one. F4 lists workouts
for reuse of their **current live source routine**, using the startup date/time.
Missing or deleted sources produce a visible error and create no draft. Escape
returns to startup while retaining fields. F10 (or Ctrl+C) quits and retains saved
drafts. Text fields accept normal typing and Backspace; F1 displays help.

Inside a draft, Tab switches actual activity and separately labelled read-only
preserved intention. Up/Down selects actual sets and scrolls intention; PgUp/PgDown scrolls details. Actual repetition
counts mean attempts; prescribed counts mean minimum successful repetitions.
Views introduce no correspondence or target-achievement inference.

F5 searches exercises by case-insensitive name substring. Results show equipment,
measurement mode, and load convention; arrows select, Enter opens an ordinary
single-portion recording form, and Escape cancels. Tab/Up/Down moves between
warmup/main type, kilograms, load description, measurement, RPE, white/red flags,
and set/portion/session notes. Optional blanks stay unknown; explicit zero and
exact decimal text are preserved. Enter or F12 validates and immediately saves.
Enter on a selected actual set edits it. F6 duplicates its structure/type/load
into an unsaved form with blank quantity, observations and set/portion notes;
rest and grouping are not copied. F7 in the intention view seeds a form from
prescribed structure/type/load with blank actual quantity and separately labelled
targets. Existing complex sets remain inspectable; use the CLI to edit complexes,
rest, or supersets in this slice.

Escape cancels only the current form; confirmed saves remain durable. Errors
retain input for Enter/F12 retry or Escape cancel. F11 inspects latest saved
activity without changing the rejected form; Escape returns to it. After a stale
write, Shift-F11 explicitly abandons the edit and reopens latest activity. Nothing
automatically merges or retries against a new revision.

F8 opens an explicit finish confirmation: requires saved performed activity and
no unresolved form, makes activity read-only, and optionally accepts an end time.
F9 separately confirms permanent removal of a draft. Neither action updates a
routine. Resize retains text. Unconfirmed forms are memory-only; confirmed edits
survive restart. Normal and error exits restore the terminal. Noninteractive
startup fails before opening the database.

The `precision` library exposes the existing `Store` operations and supported
application types to both adapters. Validation, decimals, migrations, and atomic
storage remain shared. `tui::Session` provides the headless user-action/visible-state
interface tested against temporary databases.

On Unix, `cargo test --test terminal` checks real keyboard delivery, resize,
confirmed recording recovery, and terminal restoration via a PTY (requires Python 3).
After `cargo build`, `python3 tests/terminal_smoke.py` also runs it directly.
