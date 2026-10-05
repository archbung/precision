# Precision

A local exercise-definition CLI backed by SQLite. Build with `cargo build`;
run `cargo run -- --help` or use `target/debug/precision`.

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
zero means no addition or assistance. This slice defines activities; it does
not record loads or performances.

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
