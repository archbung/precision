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
fn proposal_is_read_only_and_labels_attempts() {
    let db = TempDir::new().unwrap();
    exercise(&db, "Squat", "repetitions", "external");
    ok(&db, &["workout", "start", "--date", "2026-10-05"]);
    let path = write(
        &db,
        &json!({"schema_version":1,"date":"2026-10-05","notes":"session","sets":[{"type":"warmup","kilograms":20,"rpe":9,"white_flags":1,"red_flags":2,"notes":"set","portions":[{"exercise_id":1,"repetitions":9,"notes":"ninth unsuccessful"}]}]}),
    );
    ok(&db, &["workout", "update", "1", "--file", &path]);
    let review = db.path().join("review.json");
    let review = review.to_str().unwrap();
    assert!(
        !run(
            &db,
            &[
                "routine",
                "propose",
                "--from-workout",
                "1",
                "--file",
                review
            ]
        )
        .status
        .success()
    );
    ok(&db, &["workout", "finish", "1"]);
    let before = show(&db, "1");
    let output = ok(
        &db,
        &[
            "routine",
            "propose",
            "--from-workout",
            "1",
            "--file",
            review,
        ],
    );
    assert!(output.contains("attempts"));
    let proposal: Value = serde_json::from_slice(&std::fs::read(review).unwrap()).unwrap();
    assert_eq!(proposal["routine"]["notes"], "session");
    assert_eq!(
        proposal["routine"]["sets"][0]["portions"][0]["repetitions"],
        9
    );
    assert_eq!(proposal["routine"]["sets"][0]["rpe"], Value::Null);
    assert!(proposal["routine"]["sets"][0].get("white_flags").is_none());
    assert_eq!(proposal["routine"]["rest"], json!([]));
    assert_eq!(show(&db, "1"), before);
    assert_eq!(ok(&db, &["routine", "list", "--json"]).trim(), "[]");
}

#[test]
fn reviewed_creation_copies_complex_grouping_and_remaps_local_ids() {
    let db = TempDir::new().unwrap();
    exercise(&db, "Squat", "repetitions", "external");
    exercise(&db, "Hang", "duration", "added-bodyweight");
    exercise(&db, "Carry", "distance", "external");
    ok(&db, &["workout", "start", "--date", "2026-10-05"]);
    let path = write(
        &db,
        &json!({"schema_version":1,"date":"2026-10-05","notes":"session","rest":[30,0],"sets":[
        {"type":"warmup","kilograms":0,"load_description":"bar","notes":"set","portions":[{"exercise_id":1,"repetitions":9,"notes":"ninth unsuccessful"},{"exercise_id":2,"seconds":1.25}]},
        {"kilograms":null,"load_description":"band","portions":[{"exercise_id":3,"metres":0}]},
        {"kilograms":-20.5,"portions":[{"exercise_id":2,"seconds":0}]}]}),
    );
    ok(&db, &["workout", "update", "1", "--file", &path]);
    let mut actual: Value = serde_json::from_str(&ok(
        &db,
        &["workout", "show", "1", "--json", "--actual-only"],
    ))
    .unwrap();
    actual["supersets"] = json!([{"set_ids":[actual["sets"][0]["id"],actual["sets"][2]["id"]]}]);
    let path = write(&db, &actual);
    ok(&db, &["workout", "update", "1", "--file", &path]);
    ok(&db, &["workout", "finish", "1"]);
    let before = show(&db, "1");
    let review = db.path().join("review.json");
    let review = review.to_str().unwrap();
    ok(
        &db,
        &[
            "routine",
            "propose",
            "--from-workout",
            "1",
            "--file",
            review,
        ],
    );
    let mut proposal: Value = serde_json::from_slice(&std::fs::read(review).unwrap()).unwrap();
    assert_eq!(proposal["routine"]["rest"], json!([null, null]));
    proposal["routine"]["name"] = json!("Reviewed");
    proposal["routine"]["sets"][0]["portions"][0]["repetitions"] = json!(8);
    proposal["routine"]["sets"][0]["rpe"] = json!(7.5);
    proposal["routine"]["rest"] = json!([60, null]);
    let path = write(&db, &proposal);
    assert!(
        !run(&db, &["routine", "create", "--file", &path])
            .status
            .success()
    );
    assert!(
        !run(
            &db,
            &[
                "routine",
                "create",
                "--file",
                &path,
                "--reviewed-from-workout",
                "999"
            ]
        )
        .status
        .success()
    );
    ok(
        &db,
        &[
            "routine",
            "create",
            "--file",
            &path,
            "--reviewed-from-workout",
            "1",
        ],
    );
    let routine: Value =
        serde_json::from_str(&ok(&db, &["routine", "show", "1", "--json"])).unwrap();
    assert_eq!(routine["name"], "Reviewed");
    assert_eq!(routine["notes"], "session");
    assert_eq!(routine["rest"], json!([60, null]));
    assert_eq!(routine["sets"][0]["type"], "warmup");
    assert_eq!(routine["sets"][0]["rpe"], 7.5);
    assert_eq!(routine["sets"][0]["load_description"], "bar");
    assert_eq!(routine["sets"][0]["notes"], "set");
    assert_eq!(routine["sets"][0]["portions"][0]["repetitions"], 8);
    assert_eq!(
        routine["sets"][0]["portions"][0]["notes"],
        "ninth unsuccessful"
    );
    assert_eq!(routine["sets"][0]["portions"][1]["seconds"], 1.25);
    assert_eq!(routine["sets"][1]["kilograms"], Value::Null);
    assert_eq!(routine["sets"][1]["portions"][0]["metres"], 0);
    assert_eq!(routine["sets"][2]["kilograms"], -20.5);
    assert!(routine["sets"][0]["id"].as_i64().unwrap() > 0);
    assert_eq!(
        routine["supersets"][0]["set_ids"],
        json!([routine["sets"][0]["id"], routine["sets"][2]["id"]])
    );
    assert_eq!(show(&db, "1"), before);
}

fn source_workout(db: &TempDir) {
    exercise(db, "Squat", "repetitions", "external");
    let path = write(
        db,
        &json!({"schema_version":1,"name":"Source","sets":[{"portions":[{"exercise_id":1,"repetitions":5}]}]}),
    );
    ok(db, &["routine", "create", "--file", &path]);
    ok(
        db,
        &["workout", "start", "--date", "2026-10-05", "--routine", "1"],
    );
    let path = write(
        db,
        &json!({"schema_version":1,"date":"2026-10-05","sets":[{"portions":[{"exercise_id":1,"repetitions":9,"notes":"ninth failed"}]}]}),
    );
    ok(db, &["workout", "update", "1", "--file", &path]);
    ok(db, &["workout", "finish", "1"]);
}
fn propose(db: &TempDir) -> Value {
    let path = db.path().join("review.json");
    ok(
        db,
        &[
            "routine",
            "propose",
            "--from-workout",
            "1",
            "--file",
            path.to_str().unwrap(),
        ],
    );
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn routine(db: &TempDir) -> Value {
    serde_json::from_str(&ok(db, &["routine", "show", "1", "--json"])).unwrap()
}
#[test]
fn reviewed_replacement_preserves_history_and_reuse_sees_current_source() {
    let db = TempDir::new().unwrap();
    source_workout(&db);
    let before = show(&db, "1");
    let mut proposal = propose(&db);
    assert_eq!(proposal["source_routine_id"], 1);
    assert_eq!(proposal["source_revision"], 1);
    proposal["routine"]["name"] = json!("Reviewed source");
    proposal["routine"]["sets"][0]["portions"][0]["repetitions"] = json!(8);
    let path = write(&db, &proposal);
    assert!(
        !run(
            &db,
            &[
                "routine",
                "replace-source",
                "--from-workout",
                "1",
                "--file",
                &path
            ]
        )
        .status
        .success()
    );
    ok(
        &db,
        &[
            "routine",
            "replace-source",
            "--from-workout",
            "1",
            "--file",
            &path,
            "--reviewed",
        ],
    );
    assert_eq!(routine(&db)["id"], 1);
    assert_eq!(routine(&db)["sets"][0]["portions"][0]["repetitions"], 8);
    assert_eq!(show(&db, "1"), before);
    ok(&db, &["workout", "reuse", "1", "--date", "2026-10-06"]);
    assert_eq!(
        show(&db, "2")["intention"]["sets"][0]["portions"][0]["repetitions"],
        8
    );
    let saved = routine(&db);
    let out = run(
        &db,
        &[
            "routine",
            "replace-source",
            "--from-workout",
            "1",
            "--file",
            &path,
            "--reviewed",
        ],
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("stale"));
    assert_eq!(routine(&db), saved);
    assert_eq!(show(&db, "1"), before);
    assert_eq!(propose(&db)["source_revision"], 2);
}
#[test]
fn deleted_source_allows_new_conversion_but_replacement_requires_fresh_live_source() {
    let db = TempDir::new().unwrap();
    source_workout(&db);
    let proposal = propose(&db);
    let mut source = routine(&db);
    source["name"] = json!("Edited");
    let path = write(&db, &source);
    ok(&db, &["routine", "update", "1", "--file", &path]);
    assert_eq!(propose(&db)["source_revision"], 2);
    let path = write(&db, &proposal);
    assert!(
        !run(
            &db,
            &[
                "routine",
                "replace-source",
                "--from-workout",
                "1",
                "--file",
                &path,
                "--reviewed"
            ]
        )
        .status
        .success()
    );
    ok(&db, &["routine", "delete", "1"]);
    let before = show(&db, "1");
    assert!(
        !run(
            &db,
            &[
                "routine",
                "replace-source",
                "--from-workout",
                "1",
                "--file",
                &path,
                "--reviewed"
            ]
        )
        .status
        .success()
    );
    ok(
        &db,
        &[
            "routine",
            "create",
            "--file",
            &path,
            "--reviewed-from-workout",
            "1",
        ],
    );
    let fresh = propose(&db);
    assert_eq!(fresh["source_routine_id"], 1);
    assert_eq!(fresh["source_revision"], Value::Null);
    let path = write(&db, &fresh);
    ok(
        &db,
        &[
            "routine",
            "create",
            "--file",
            &path,
            "--reviewed-from-workout",
            "1",
        ],
    );
    assert_eq!(show(&db, "1"), before);
}
#[test]
fn invalid_review_rolls_back_and_rejects_foreign_identity_and_provenance() {
    let db = TempDir::new().unwrap();
    source_workout(&db);
    let proposal = propose(&db);
    let saved = routine(&db);
    for (pointer, value) in [
        ("/from_workout", json!(999)),
        ("/source_routine_id", json!(999)),
        ("/source_revision", json!(0)),
        ("/source_revision", Value::Null),
        ("/schema_version", json!(2)),
        ("/routine/id", json!(1)),
        ("/routine/sets/0/id", json!(1)),
        ("/routine/sets/0/portions/0/id", json!(1)),
        ("/routine/sets/0/portions/0/repetitions", json!(1.5)),
        ("/routine/sets/0/portions/0/exercise_id", json!(999)),
        ("/routine/sets/0/kilograms", json!(-1)),
        ("/routine/sets/0/rpe", json!(5.25)),
        ("/routine/rest", json!([0])),
        ("/routine/sets", json!([])),
    ] {
        let mut invalid = proposal.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        let path = write(&db, &invalid);
        for args in [
            vec![
                "routine",
                "replace-source",
                "--from-workout",
                "1",
                "--file",
                &path,
                "--reviewed",
            ],
            vec![
                "routine",
                "create",
                "--file",
                &path,
                "--reviewed-from-workout",
                "1",
            ],
        ] {
            assert!(!run(&db, &args).status.success(), "{pointer}");
            assert_eq!(routine(&db), saved);
            let list: Value =
                serde_json::from_str(&ok(&db, &["routine", "list", "--json"])).unwrap();
            assert_eq!(list.as_array().unwrap().len(), 1);
        }
    }
}
