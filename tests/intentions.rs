use serde_json::{Value, json};
use std::process::{Command, Output};
use tempfile::TempDir;
fn run(db: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_precision"))
        .arg("--db")
        .arg(db.path().join("db.sqlite3"))
        .args(args)
        .output()
        .unwrap()
}
fn ok(db: &TempDir, args: &[&str]) -> String {
    let out = run(db, args);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}
fn write(db: &TempDir, document: &Value) -> String {
    let path = db.path().join("workout.json");
    std::fs::write(&path, serde_json::to_vec(document).unwrap()).unwrap();
    path.to_str().unwrap().into()
}
fn show(db: &TempDir, id: &str) -> Value {
    serde_json::from_str(&ok(db, &["workout", "show", id, "--json"])).unwrap()
}
fn exercise(db: &TempDir, name: &str, mode: &str, load: &str) {
    ok(
        db,
        &[
            "exercise",
            "create",
            "--name",
            name,
            "--measurement",
            mode,
            "--load-convention",
            load,
        ],
    );
}
#[test]
fn routine_copy_is_independent_and_survives_source_deletion() {
    let db = TempDir::new().unwrap();
    exercise(&db, "Squat", "repetitions", "external");
    let path = write(
        &db,
        &json!({"schema_version":1,"name":"Original","notes":"routine notes","sets":[{"type":"warmup","kilograms":0,"rpe":5.5,"notes":"set note","portions":[{"exercise_id":1,"repetitions":null,"notes":"portion note"}]}]}),
    );
    ok(&db, &["routine", "create", "--file", &path]);
    ok(
        &db,
        &["workout", "start", "--date", "2026-10-05", "--routine", "1"],
    );
    let before = show(&db, "1");
    assert_eq!(before["intention"]["notes"], "routine notes");
    assert_eq!(
        before["intention"]["sets"][0]["portions"][0]["repetitions"],
        Value::Null
    );
    let mut source: Value =
        serde_json::from_str(&ok(&db, &["routine", "show", "1", "--json"])).unwrap();
    source["name"] = json!("Changed");
    source["sets"][0]["portions"][0]["repetitions"] = json!(8);
    let path = write(&db, &source);
    ok(&db, &["routine", "update", "1", "--file", &path]);
    assert_eq!(show(&db, "1"), before);
    let path = write(
        &db,
        &json!({"schema_version":1,"date":"2026-10-05","sets":[{"portions":[{"exercise_id":1,"repetitions":9}]},{"portions":[{"exercise_id":1,"repetitions":0}]}]}),
    );
    ok(&db, &["workout", "update", "1", "--file", &path]);
    assert_eq!(show(&db, "1")["intention"], before["intention"]);
    let path = write(&db, &json!({"schema_version":1,"notes":"today","sets":[]}));
    ok(
        &db,
        &["workout", "intention", "update", "1", "--file", &path],
    );
    let edited = show(&db, "1");
    assert_eq!(edited["intention"]["sets"], json!([]));
    assert_eq!(edited["sets"].as_array().unwrap().len(), 2);
    ok(&db, &["workout", "finish", "1"]);
    assert!(
        !run(
            &db,
            &["workout", "intention", "update", "1", "--file", &path]
        )
        .status
        .success()
    );
    ok(&db, &["routine", "delete", "1"]);
    let deleted = show(&db, "1");
    assert_eq!(deleted["source_routine_id"], Value::Null);
    assert_eq!(deleted["original_source_id"], 1);
    assert_eq!(deleted["original_source_name"], "Original");
    assert_eq!(deleted["intention"], edited["intention"]);
    assert_eq!(deleted["sets"], edited["sets"]);
}
#[test]
fn intention_validation_and_owned_identities_are_atomic() {
    let db = TempDir::new().unwrap();
    exercise(&db, "Squat", "repetitions", "external");
    exercise(&db, "Hang", "duration", "added-bodyweight");
    for _ in 0..2 {
        ok(&db, &["workout", "start", "--date", "2026-10-05"]);
    }
    assert_eq!(show(&db, "1")["intention"]["sets"], json!([]));
    let path = write(
        &db,
        &json!({"schema_version":1,"notes":"today","sets":[
        {"type":"warmup","kilograms":0,"rpe":5.5,"load_description":"bar","notes":"set","portions":[{"exercise_id":1,"repetitions":0,"notes":"portion"},{"exercise_id":2,"seconds":null}]},
        {"kilograms":-20.5,"portions":[{"exercise_id":2,"seconds":1.25}]}]}),
    );
    for id in ["1", "2"] {
        ok(
            &db,
            &["workout", "intention", "update", id, "--file", &path],
        );
    }
    let before = show(&db, "1");
    let foreign = show(&db, "2");
    for (pointer, value) in [
        ("/schema_version", json!(2)),
        ("/sets/0/id", foreign["intention"]["sets"][0]["id"].clone()),
        (
            "/sets/0/portions/0/id",
            foreign["intention"]["sets"][0]["portions"][0]["id"].clone(),
        ),
        ("/sets/0/rpe", json!(5.25)),
        ("/sets/0/kilograms", json!(-1)),
        ("/sets/0/portions/0/repetitions", json!(1.5)),
        ("/sets/0/portions/0/exercise_id", json!(999)),
        ("/sets/0/portions", json!([])),
        ("/sets/1/id", before["intention"]["sets"][0]["id"].clone()),
    ] {
        let mut document = before["intention"].clone();
        document["notes"] = json!("must roll back");
        *document.pointer_mut(pointer).unwrap() = value;
        let path = write(&db, &document);
        assert!(
            !run(
                &db,
                &["workout", "intention", "update", "1", "--file", &path]
            )
            .status
            .success(),
            "{pointer}"
        );
        assert_eq!(show(&db, "1"), before);
        assert_eq!(show(&db, "2"), foreign);
    }
    for (pointer, field, value) in [
        ("", "source_routine_id", json!(1)),
        ("", "name", json!("routine")),
        ("/sets/0", "white_flags", json!(3)),
        ("/sets/0/portions/0", "seconds", json!(1)),
    ] {
        let mut document = before["intention"].clone();
        document
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(field.into(), value);
        let path = write(&db, &document);
        assert!(
            !run(
                &db,
                &["workout", "intention", "update", "1", "--file", &path]
            )
            .status
            .success()
        );
        assert_eq!(show(&db, "1"), before);
    }
    let mut reordered = before["intention"].clone();
    reordered["sets"].as_array_mut().unwrap().reverse();
    let path = write(&db, &reordered);
    ok(
        &db,
        &["workout", "intention", "update", "1", "--file", &path],
    );
    assert_eq!(show(&db, "1")["intention"], reordered);
    let mut reduced = reordered.clone();
    reduced["sets"] = json!([reordered["sets"][0].clone()]);
    let path = write(&db, &reduced);
    ok(
        &db,
        &["workout", "intention", "update", "1", "--file", &path],
    );
    let path = write(&db, &reordered);
    assert!(
        !run(
            &db,
            &["workout", "intention", "update", "1", "--file", &path]
        )
        .status
        .success()
    );
    assert_eq!(show(&db, "1")["intention"], reduced);
    let human = ok(&db, &["workout", "show", "1"]);
    assert!(human.contains("Intention notes: today"));
    assert!(human.contains("minimum seconds: 1.25"));
    assert!(human.contains("Actual activity"));
    assert!(
        !run(
            &db,
            &[
                "workout",
                "start",
                "--date",
                "2026-10-05",
                "--routine",
                "999"
            ]
        )
        .status
        .success()
    );
    let drafts: Value =
        serde_json::from_str(&ok(&db, &["workout", "list", "--drafts", "--json"])).unwrap();
    assert_eq!(drafts.as_array().unwrap().len(), 2);
}
#[test]
fn deleted_sources_preserve_nonempty_drafts_and_completed_intentions() {
    let db = TempDir::new().unwrap();
    exercise(&db, "Squat", "repetitions", "external");
    let path = write(
        &db,
        &json!({"schema_version":1,"name":"Source","sets":[{"portions":[{"exercise_id":1,"repetitions":5}]},{"portions":[{"exercise_id":1,"repetitions":8}]}]}),
    );
    ok(&db, &["routine", "create", "--file", &path]);
    for _ in 0..2 {
        ok(
            &db,
            &["workout", "start", "--date", "2026-10-05", "--routine", "1"],
        );
    }
    let path = write(
        &db,
        &json!({"schema_version":1,"date":"2026-10-05","sets":[{"portions":[{"exercise_id":1,"repetitions":3}]}]}),
    );
    ok(&db, &["workout", "update", "2", "--file", &path]);
    ok(&db, &["workout", "finish", "2"]);
    let snapshots = [show(&db, "1"), show(&db, "2")];
    ok(&db, &["routine", "delete", "1"]);
    let path = write(
        &db,
        &json!({"schema_version":1,"name":"Source","sets":[{"portions":[{"exercise_id":1}]}]}),
    );
    ok(&db, &["routine", "create", "--file", &path]);
    for (id, before) in ["1", "2"].iter().zip(snapshots) {
        let mut expected = before;
        expected["source_routine_id"] = Value::Null;
        assert_eq!(show(&db, id), expected);
    }
    ok(&db, &["workout", "discard", "1"]);
    assert_eq!(
        show(&db, "2")["intention"]["sets"][0]["portions"][0]["repetitions"],
        5
    );
    assert_eq!(show(&db, "2")["sets"][0]["portions"][0]["repetitions"], 3);
}

#[test]
fn actual_export_is_editable_without_prescription_or_provenance_fields() {
    let db = TempDir::new().unwrap();
    ok(&db, &["workout", "start", "--date", "2026-10-05"]);
    let document: Value = serde_json::from_str(&ok(
        &db,
        &["workout", "show", "1", "--json", "--actual-only"],
    ))
    .unwrap();
    assert!(document.get("intention").is_none());
    assert!(document.get("source_routine_id").is_none());
    let path = write(&db, &document);
    ok(&db, &["workout", "update", "1", "--file", &path]);
}
