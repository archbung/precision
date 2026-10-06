mod support;
use serde_json::{Value, json};
use std::process::{Command, Output};
use tempfile::TempDir;
fn run(db: &TempDir, args: &[&str]) -> Output {
    support::empty_database(&db.path().join("db.sqlite3"));
    Command::new(env!("CARGO_BIN_EXE_precision"))
        .arg("--db")
        .arg(db.path().join("db.sqlite3"))
        .args(support::with_workout_revision(
            &db.path().join("db.sqlite3"),
            args,
        ))
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
    let mut document = document.clone();
    if document.get("original_source_name").is_some() {
        for field in [
            "intention",
            "source_routine_id",
            "original_source_id",
            "original_source_name",
        ] {
            document.as_object_mut().unwrap().remove(field);
        }
    }
    write_raw(db, &document)
}
fn write_raw(db: &TempDir, document: &Value) -> String {
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
fn empty_drafts_survive_restart_and_can_be_discarded_but_not_finished() {
    let db = TempDir::new().unwrap();
    ok(&db, &["workout", "start", "--date", "2026-10-05"]);
    let draft = show(&db, "1");
    assert_eq!(draft["state"], "draft");
    assert_eq!(draft["sets"], json!([]));
    assert_eq!(ok(&db, &["workout", "list", "--json"]).trim(), "[]");
    assert!(ok(&db, &["workout", "list", "--drafts"]).contains("draft"));
    assert!(!run(&db, &["workout", "finish", "1"]).status.success());
    assert_eq!(show(&db, "1"), draft);
    ok(&db, &["workout", "discard", "1"]);
    assert!(!run(&db, &["workout", "show", "1"]).status.success());
}
#[test]
fn actual_activity_preserves_zero_attempt_notes_complexes_and_cross_midnight_times() {
    let db = TempDir::new().unwrap();
    exercise(&db, "Squat", "repetitions", "external");
    exercise(&db, "Hang", "duration", "added-bodyweight");
    exercise(&db, "Carry", "distance", "external");
    ok(
        &db,
        &[
            "workout",
            "start",
            "--date",
            "2026-10-05",
            "--start",
            "2026-10-05T23:50:00+07:00",
        ],
    );
    let document = json!({"schema_version":1,"date":"2026-10-05","start":"2026-10-05T23:50:00+07:00","notes":"session note","sets":[
    {"type":"warmup","kilograms":0,"rpe":1,"white_flags":1,"red_flags":2,"notes":"set note","portions":[{"exercise_id":1,"repetitions":0,"notes":"unsuccessful attempt"}]},
    {"load_description":"purple band","rpe":9.5,"portions":[{"exercise_id":2,"seconds":0.125},{"exercise_id":3,"metres":1.234567},{"exercise_id":1,"repetitions":9,"notes":"ninth unsuccessful; sides 9/10"}]}
    ]});
    let path = write(&db, &document);
    ok(&db, &["workout", "update", "1", "--file", &path]);
    let saved = show(&db, "1");
    assert_eq!(saved["sets"][0]["portions"][0]["repetitions"], 0);
    assert_eq!(saved["sets"][1]["kilograms"], Value::Null);
    assert_eq!(saved["sets"][1]["portions"][1]["metres"], json!(1.234567));
    assert_eq!(saved["notes"], "session note");
    assert_eq!(saved["sets"][0]["type"], "warmup");
    assert_eq!(saved["sets"][1]["type"], "main");
    assert_eq!(saved["sets"][1]["rpe"], json!(9.5));
    assert_eq!(
        saved["sets"][1]["portions"][2]["notes"],
        "ninth unsuccessful; sides 9/10"
    );
    let human = ok(&db, &["workout", "show", "1"]);
    for label in [
        "draft",
        "unknown",
        "failed judgment",
        "attempted repetitions",
        "session note",
        "set note",
        "unsuccessful attempt",
        "purple band",
    ] {
        assert!(human.contains(label), "{human}");
    }
    let path = write(&db, &saved);
    ok(&db, &["workout", "update", "1", "--file", &path]);
    let mut expected = saved.clone();
    expected["revision"] = json!(2);
    assert_eq!(show(&db, "1"), expected);
    ok(
        &db,
        &[
            "workout",
            "finish",
            "1",
            "--end",
            "2026-10-06T00:20:00+07:00",
        ],
    );
    let finished = show(&db, "1");
    assert_eq!(finished["state"], "finished");
    assert_eq!(finished["end"], "2026-10-06T00:20:00+07:00");
    assert!(ok(&db, &["workout", "list"]).contains("finished"));
    for args in [
        vec!["workout", "update", "1", "--file", &path],
        vec!["workout", "discard", "1"],
        vec!["workout", "finish", "1"],
    ] {
        assert!(!run(&db, &args).status.success());
        assert_eq!(show(&db, "1"), finished);
    }
}
#[test]
fn invalid_updates_and_finishes_roll_back_every_field() {
    let db = TempDir::new().unwrap();
    exercise(&db, "Squat", "repetitions", "external");
    exercise(&db, "Hang", "duration", "added-bodyweight");
    ok(&db, &["workout", "start", "--date", "2026-10-05"]);
    let path = write(
        &db,
        &json!({"schema_version":1,"date":"2026-10-05","sets":[{"portions":[{"exercise_id":1,"repetitions":1}]}]}),
    );
    ok(&db, &["workout", "update", "1", "--file", &path]);
    let before = show(&db, "1");
    for (pointer, value) in [
        ("/id", json!(999)),
        ("/schema_version", json!(2)),
        ("/state", json!("finished")),
        ("/date", json!("2026-02-30")),
        ("/start", json!("23:50")),
        ("/start", json!("2026-10-06T00:00:00Z")),
        ("/end", json!("2026-10-04T23:00:00Z")),
        ("/sets/0/id", json!(999)),
        ("/sets/0/type", json!("failure")),
        ("/sets/0/rpe", json!(5.25)),
        ("/sets/0/rpe", json!(0)),
        ("/sets/0/rpe", json!(10.5)),
        ("/sets/0/kilograms", json!(-1)),
        ("/sets/0/kilograms", json!(0.0000001)),
        (
            "/sets/0/kilograms",
            json!({"$serde_json::private::Number":"5"}),
        ),
        ("/sets/0/white_flags", json!(3)),
        ("/sets/0/red_flags", json!(-1)),
        ("/sets/0/portions", json!([])),
        ("/sets/0/portions/0/id", json!(999)),
        ("/sets/0/portions/0/exercise_id", json!(999)),
        ("/sets/0/portions/0/repetitions", Value::Null),
        ("/sets/0/portions/0/repetitions", json!(-1)),
        ("/sets/0/portions/0/repetitions", json!(1.5)),
        ("/sets/0/portions/0/exercise_id", json!(2)),
    ] {
        let mut doc = before.clone();
        doc["notes"] = json!("must roll back");
        *doc.pointer_mut(pointer).unwrap() = value;
        let path = write(&db, &doc);
        assert!(
            !run(&db, &["workout", "update", "1", "--file", &path])
                .status
                .success(),
            "{pointer}"
        );
        assert_eq!(show(&db, "1"), before, "{pointer}");
    }
    for (pointer, field, value) in [
        ("", "intention", json!({})),
        ("", "source_routine_id", json!(1)),
        ("/sets/0", "rest", json!(0)),
        ("/sets/0", "completion", json!(true)),
        ("/sets/0/portions/0", "sides", json!([1, 1])),
        ("/sets/0/portions/0", "seconds", json!(2)),
    ] {
        let mut doc = before.clone();
        for field in [
            "intention",
            "source_routine_id",
            "original_source_id",
            "original_source_name",
        ] {
            doc.as_object_mut().unwrap().remove(field);
        }
        doc.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(field.into(), value);
        let path = write_raw(&db, &doc);
        assert!(
            !run(&db, &["workout", "update", "1", "--file", &path])
                .status
                .success()
        );
        assert_eq!(show(&db, "1"), before);
    }
    assert!(
        !run(
            &db,
            &["workout", "finish", "1", "--end", "2026-10-04T23:59:59Z"]
        )
        .status
        .success()
    );
    assert_eq!(show(&db, "1"), before);
    ok(&db, &["workout", "finish", "1"]);
    assert_eq!(show(&db, "1")["start"], Value::Null);
    assert_eq!(show(&db, "1")["end"], Value::Null);
}
#[test]
fn retained_identities_follow_order_and_removed_or_foreign_ids_cannot_return() {
    let db = TempDir::new().unwrap();
    exercise(&db, "Pull", "repetitions", "added-bodyweight");
    for _ in 0..2 {
        ok(&db, &["workout", "start", "--date", "2026-10-05"]);
    }
    let path = write(
        &db,
        &json!({"schema_version":1,"date":"2026-10-05","sets":[{"kilograms":-20,"portions":[{"exercise_id":1,"repetitions":3}]},{"portions":[{"exercise_id":1,"repetitions":4}]}]}),
    );
    for id in ["1", "2"] {
        ok(&db, &["workout", "update", id, "--file", &path]);
    }
    let before = show(&db, "1");
    let foreign = show(&db, "2");
    let mut edited = before.clone();
    edited["sets"].as_array_mut().unwrap().reverse();
    edited["sets"]
        .as_array_mut()
        .unwrap()
        .push(json!({"portions":[{"exercise_id":1,"repetitions":0}]}));
    edited["rest"] = json!([null, null]);
    let path = write(&db, &edited);
    ok(&db, &["workout", "update", "1", "--file", &path]);
    let after = show(&db, "1");
    assert_eq!(after["sets"][0], before["sets"][1]);
    assert_eq!(after["sets"][1], before["sets"][0]);
    assert!(after["sets"][2]["id"].as_i64().unwrap() > foreign["sets"][1]["id"].as_i64().unwrap());
    for replacement in [foreign["sets"][0].clone(), after["sets"][1].clone()] {
        let mut bad = after.clone();
        bad["sets"][0] = replacement;
        let path = write(&db, &bad);
        assert!(
            !run(&db, &["workout", "update", "1", "--file", &path])
                .status
                .success()
        );
        assert_eq!(show(&db, "1"), after);
    }
    let mut reduced = after.clone();
    reduced["sets"] = json!([after["sets"][0].clone()]);
    reduced["rest"] = json!([]);
    let path = write(&db, &reduced);
    ok(&db, &["workout", "update", "1", "--file", &path]);
    let path = write(&db, &after);
    assert!(
        !run(&db, &["workout", "update", "1", "--file", &path])
            .status
            .success()
    );
    reduced["revision"] = json!(3);
    assert_eq!(show(&db, "1"), reduced);
}
#[test]
fn mixed_load_conventions_and_timestamp_instants_are_validated_independently() {
    let db = TempDir::new().unwrap();
    exercise(&db, "Hang", "duration", "added-bodyweight");
    exercise(&db, "Carry", "distance", "external");
    for args in [
        vec!["workout", "start", "--date", "2026-02-30"],
        vec![
            "workout",
            "start",
            "--date",
            "2026-10-05",
            "--start",
            "2026-10-04T23:00:00Z",
        ],
    ] {
        assert!(!run(&db, &args).status.success());
    }
    ok(
        &db,
        &[
            "workout",
            "start",
            "--date",
            "2026-10-05",
            "--start",
            "2026-10-05T23:00:00-02:00",
        ],
    );
    let mut doc = json!({"schema_version":1,"date":"2026-10-05","start":"2026-10-05T23:00:00-02:00","sets":[{"kilograms":-10.5,"portions":[{"exercise_id":1,"seconds":0.5},{"exercise_id":1,"seconds":0}]}]});
    let path = write(&db, &doc);
    ok(&db, &["workout", "update", "1", "--file", &path]);
    let before = show(&db, "1");
    doc["sets"][0]["portions"]
        .as_array_mut()
        .unwrap()
        .push(json!({"exercise_id":2,"metres":0}));
    let path = write(&db, &doc);
    assert!(
        !run(&db, &["workout", "update", "1", "--file", &path])
            .status
            .success()
    );
    assert_eq!(show(&db, "1"), before);
    doc["sets"][0]["kilograms"] = json!(0);
    let path = write(&db, &doc);
    ok(&db, &["workout", "update", "1", "--file", &path]);
    let before = show(&db, "1");
    assert!(
        !run(
            &db,
            &[
                "workout",
                "finish",
                "1",
                "--end",
                "2026-10-06T01:30:00+02:00"
            ]
        )
        .status
        .success()
    );
    assert_eq!(show(&db, "1"), before);
    ok(
        &db,
        &[
            "workout",
            "finish",
            "1",
            "--end",
            "2026-10-06T03:30:00+02:00",
        ],
    );
    assert_eq!(show(&db, "1")["start"], "2026-10-05T23:00:00-02:00");
}
