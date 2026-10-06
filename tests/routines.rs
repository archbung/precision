mod support;
use serde_json::{Value, json};
use std::process::{Command, Output};
use tempfile::TempDir;
fn run(db: &TempDir, args: &[&str]) -> Output {
    support::empty_database(&db.path().join("db.sqlite3"));
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
    let path = db.path().join("routine.json");
    std::fs::write(&path, serde_json::to_vec(document).unwrap()).unwrap();
    path.to_str().unwrap().into()
}
fn show(db: &TempDir, id: &str) -> Value {
    serde_json::from_str(&ok(db, &["routine", "show", id, "--json"])).unwrap()
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
fn ordinary_sets_and_complexes_preserve_targets_notes_order_and_ids_after_restart() {
    let db = TempDir::new().unwrap();
    exercise(&db, "Squat", "repetitions", "external");
    exercise(&db, "Carry", "distance", "external");
    assert_eq!(ok(&db, &["routine", "list", "--json"]).trim(), "[]");
    let doc = json!({"schema_version":1,"name":"  Strength  ","notes":"session instruction","sets":[
        {"type":"warmup","kilograms":0,"rpe":5.5,"load_description":"bar","notes":"set note","portions":[{"exercise_id":1,"repetitions":5,"notes":"portion note"}]},
        {"kilograms":12.125,"portions":[{"exercise_id":1},{"exercise_id":2,"metres":0.125},{"exercise_id":1,"repetitions":0}]}
    ]});
    let path = write(&db, &doc);
    ok(&db, &["routine", "create", "--file", &path]);
    let saved = show(&db, "1");
    assert_eq!(saved["name"], "Strength");
    assert_eq!(saved["notes"], "session instruction");
    assert_eq!(saved["sets"][0]["notes"], "set note");
    assert_eq!(saved["sets"][0]["rpe"], json!(5.5));
    assert_eq!(saved["sets"][0]["portions"][0]["notes"], "portion note");
    assert_eq!(saved["sets"][1]["type"], "main");
    assert_eq!(saved["sets"][1]["portions"][0]["repetitions"], Value::Null);
    assert_eq!(saved["sets"][1]["portions"][2]["repetitions"], 0);
    assert_eq!(saved["sets"][1]["kilograms"], json!(12.125));
    assert_eq!(saved["sets"][1]["portions"][1]["metres"], json!(0.125));
    assert!(saved["id"].as_i64().unwrap() > 0);
    assert!(saved["sets"][0]["id"].as_i64().unwrap() > 0);
    assert!(saved["sets"][0]["portions"][0]["id"].as_i64().unwrap() > 0);
    assert_eq!(show(&db, "1"), saved);
    let human = ok(&db, &["routine", "show", "1"]);
    for label in [
        "Strength",
        "Squat",
        "Carry",
        "minimum successful repetitions",
        "maximum RPE",
        "unspecified",
        "session instruction",
        "set note",
        "portion note",
    ] {
        assert!(human.contains(label), "{human}");
    }
    ok(&db, &["routine", "create", "--file", &path]);
    assert!(ok(&db, &["routine", "list"]).contains("2: Strength"));
}
#[test]
fn replacement_preserves_retained_ids_assigns_fresh_ids_and_deletes_atomically() {
    let db = TempDir::new().unwrap();
    exercise(&db, "Pull-up", "repetitions", "added-bodyweight");
    let path = write(
        &db,
        &json!({"schema_version":1,"name":"Pull","sets":[{"portions":[{"exercise_id":1,"repetitions":3}]},{"portions":[{"exercise_id":1,"repetitions":4}]}]}),
    );
    ok(&db, &["routine", "create", "--file", &path]);
    let before = show(&db, "1");
    let mut replacement = before.clone();
    replacement["sets"].as_array_mut().unwrap().reverse();
    replacement["name"] = json!("Edited");
    replacement["sets"][0]["portions"]
        .as_array_mut()
        .unwrap()
        .insert(0, json!({"exercise_id":1,"repetitions":0,"notes":"new"}));
    replacement["sets"]
        .as_array_mut()
        .unwrap()
        .insert(0, json!({"kilograms":-20,"portions":[{"exercise_id":1}]}));
    replacement["rest"] = json!([null, null]);
    let path = write(&db, &replacement);
    ok(&db, &["routine", "update", "1", "--file", &path]);
    let after = show(&db, "1");
    assert_eq!(after["id"], before["id"]);
    assert_eq!(after["sets"][1]["id"], before["sets"][1]["id"]);
    assert_eq!(after["sets"][2], before["sets"][0]);
    assert_eq!(
        after["sets"][1]["portions"][1],
        before["sets"][1]["portions"][0]
    );
    assert!(after["sets"][0]["id"].as_i64().unwrap() > before["sets"][1]["id"].as_i64().unwrap());
    assert!(
        after["sets"][1]["portions"][0]["id"].as_i64().unwrap()
            > before["sets"][1]["portions"][0]["id"].as_i64().unwrap()
    );
    assert_eq!(show(&db, "1"), after);
    // Removing children never allows their exported identities to be reused.
    let mut reduced = after.clone();
    reduced["sets"] = json!([after["sets"][2].clone()]);
    reduced["rest"] = json!([]);
    let path = write(&db, &reduced);
    ok(&db, &["routine", "update", "1", "--file", &path]);
    let path = write(&db, &after);
    bad(
        &db,
        &["routine", "update", "1", "--file", &path],
        "foreign set ID",
    );
    assert_eq!(show(&db, "1")["sets"], reduced["sets"]);
    ok(&db, &["routine", "delete", "1"]);
    bad(
        &db,
        &["routine", "show", "1", "--json"],
        "unknown routine ID",
    );
    bad(&db, &["routine", "delete", "1"], "unknown routine ID");
    assert_eq!(ok(&db, &["routine", "list", "--json"]).trim(), "[]");
    let path = write(
        &db,
        &json!({"schema_version":1,"name":"Pull","sets":[{"portions":[{"exercise_id":1}]}]}),
    );
    ok(&db, &["routine", "create", "--file", &path]);
    assert!(
        show(&db, "2")["sets"][0]["id"].as_i64().unwrap()
            > after["sets"][0]["id"].as_i64().unwrap()
    );
    assert_eq!(
        serde_json::from_str::<Value>(&ok(&db, &["exercise", "show", "1", "--json"])).unwrap()["name"],
        "Pull-up"
    );
}
fn bad(db: &TempDir, args: &[&str], message: &str) {
    let out = run(db, args);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(message),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
#[test]
fn invalid_replacements_leave_the_complete_original_unchanged() {
    let db = TempDir::new().unwrap();
    exercise(&db, "Squat", "repetitions", "external");
    exercise(&db, "Hang", "duration", "added-bodyweight");
    exercise(&db, "Carry", "distance", "external");
    let path = write(
        &db,
        &json!({"schema_version":1,"name":"Original","notes":"keep","sets":[{"portions":[{"exercise_id":1,"repetitions":5}]},{"portions":[{"exercise_id":2,"seconds":0.5}]}]}),
    );
    ok(&db, &["routine", "create", "--file", &path]);
    ok(&db, &["routine", "create", "--file", &path]);
    let before = show(&db, "1");
    let foreign = show(&db, "2");
    let edits: Vec<(&str, Value, &str)> = vec![
        ("/name", json!(" "), "name must not be empty"),
        ("/sets", json!([]), "at least one prescribed set"),
        ("/schema_version", json!(2), "schema_version"),
        ("/id", json!(2), "routine ID"),
        (
            "/sets/0/id",
            foreign["sets"][0]["id"].clone(),
            "foreign set ID",
        ),
        (
            "/sets/0/portions/0/id",
            foreign["sets"][0]["portions"][0]["id"].clone(),
            "foreign portion ID",
        ),
        (
            "/sets/0/id",
            before["sets"][1]["id"].clone(),
            "duplicate set ID",
        ),
        (
            "/sets/1/portions/0/id",
            before["sets"][0]["portions"][0]["id"].clone(),
            "duplicate portion ID",
        ),
        ("/sets/0/portions", json!([]), "at least one portion"),
        (
            "/sets/0/portions/0/exercise_id",
            json!(999),
            "unknown exercise ID",
        ),
        ("/sets/0/type", json!("failure"), "warmup or main"),
        ("/sets/0/rpe", json!(0.5), "RPE"),
        ("/sets/0/rpe", json!(10.5), "RPE"),
        ("/sets/0/rpe", json!(5.25), "RPE"),
        ("/sets/0/kilograms", json!(-0.5), "negative kilograms"),
        ("/sets/0/portions/0/repetitions", json!(-1), "nonnegative"),
        (
            "/sets/0/portions/0/repetitions",
            json!(1.5),
            "whole numbers",
        ),
        ("/sets/0/portions/0/repetitions", json!("5"), "invalid type"),
        ("/sets/1/portions/0/seconds", json!(-0.1), "nonnegative"),
        ("/sets/1/portions/0/exercise_id", json!(3), "incompatible"),
        (
            "/sets/0/kilograms",
            serde_json::from_str("0.0000001").unwrap(),
            "precision",
        ),
        (
            "/sets/0/kilograms",
            serde_json::from_str("1000000000000").unwrap(),
            "precision",
        ),
    ];
    for (pointer, value, message) in edits {
        let mut doc = before.clone();
        doc["name"] = json!("Invalid replacement");
        *doc.pointer_mut(pointer).unwrap() = value;
        let path = write(&db, &doc);
        bad(&db, &["routine", "update", "1", "--file", &path], message);
        assert_eq!(show(&db, "1"), before, "{pointer}");
    }
    for (pointer, field, value) in [
        ("", "typo", json!(true)),
        ("/sets/0", "white_flags", json!(3)),
        ("/sets/0", "rest", json!(0)),
        ("/sets/0/portions/0", "seconds", json!(2)),
        ("/sets/0/portions/0", "sides", json!([5, 5])),
    ] {
        let mut doc = before.clone();
        doc.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(field.into(), value);
        let path = write(&db, &doc);
        bad(
            &db,
            &["routine", "update", "1", "--file", &path],
            if field == "seconds" {
                "incompatible"
            } else {
                "unknown field"
            },
        );
        assert_eq!(show(&db, "1"), before);
    }
    let path = write(&db, &before);
    bad(
        &db,
        &["routine", "create", "--file", &path],
        "omit IDs on create",
    );
    bad(
        &db,
        &["routine", "update", "999", "--file", &path],
        "unknown routine ID",
    );
    assert_eq!(show(&db, "1"), before);
    assert_eq!(show(&db, "2"), foreign);
}
#[test]
fn decimal_meaning_is_exact_and_signed_load_requires_every_portion_to_allow_it() {
    let db = TempDir::new().unwrap();
    exercise(&db, "Hang", "duration", "added-bodyweight");
    exercise(&db, "Assisted carry", "distance", "added-bodyweight");
    exercise(&db, "Squat", "repetitions", "external");
    let raw = r#"{"schema_version":1,"name":"Decimals","sets":[
        {"kilograms":-999999999999.999999,"rpe":1,"portions":[{"exercise_id":1,"seconds":0.000001},{"exercise_id":2,"metres":1.234567}]},
        {"kilograms":1e-6,"rpe":10,"portions":[{"exercise_id":3,"repetitions":999999999999}]},
        {"kilograms":null,"load_description":"purple band","portions":[{"exercise_id":1,"seconds":null}]}]}"#;
    let path = db.path().join("exact.json");
    std::fs::write(&path, raw).unwrap();
    ok(
        &db,
        &["routine", "create", "--file", path.to_str().unwrap()],
    );
    let exported = ok(&db, &["routine", "show", "1", "--json"]);
    assert!(exported.contains("-999999999999.999999"));
    assert!(exported.contains("1.234567"));
    assert!(exported.contains("999999999999"));
    let before: Value = serde_json::from_str(&exported).unwrap();
    let path = write(&db, &before);
    ok(&db, &["routine", "update", "1", "--file", &path]);
    assert_eq!(show(&db, "1"), before);
    for invalid in [
        "0.1234567",
        "999999999999.9999991",
        "1e-7",
        "1e100",
        "1e-100",
        "NaN",
        "Infinity",
        "1e9999999999",
    ] {
        let raw = format!(
            r#"{{"schema_version":1,"name":"Invalid","sets":[{{"kilograms":{invalid},"portions":[{{"exercise_id":1}}]}}]}}"#
        );
        let path = db.path().join("invalid.json");
        std::fs::write(&path, raw).unwrap();
        let out = run(
            &db,
            &["routine", "update", "1", "--file", path.to_str().unwrap()],
        );
        assert!(!out.status.success(), "accepted {invalid}");
        assert_eq!(show(&db, "1"), before);
    }
    let mut mixed = before.clone();
    mixed["sets"][0]["portions"]
        .as_array_mut()
        .unwrap()
        .push(json!({"exercise_id":3}));
    let path = write(&db, &mixed);
    bad(
        &db,
        &["routine", "update", "1", "--file", &path],
        "negative kilograms",
    );
    mixed["sets"][0]["kilograms"] = json!(0);
    let path = write(&db, &mixed);
    ok(&db, &["routine", "update", "1", "--file", &path]);
    assert_eq!(show(&db, "1")["sets"][0]["kilograms"], 0);
}
#[test]
fn numeric_fields_reject_objects_instead_of_coercing_them_to_numbers() {
    let db = TempDir::new().unwrap();
    exercise(&db, "Squat", "repetitions", "external");
    let path = write(
        &db,
        &json!({"schema_version":1,"name":"Original","sets":[{"portions":[{"exercise_id":1,"repetitions":5}]}]}),
    );
    ok(&db, &["routine", "create", "--file", &path]);
    let before = show(&db, "1");
    for field in ["kilograms", "rpe", "repetitions", "seconds", "metres"] {
        let mut doc = before.clone();
        let target = if matches!(field, "kilograms" | "rpe") {
            &mut doc["sets"][0]
        } else {
            &mut doc["sets"][0]["portions"][0]
        };
        target[field] = json!({"$serde_json::private::Number":"5"});
        let path = write(&db, &doc);
        bad(
            &db,
            &["routine", "update", "1", "--file", &path],
            "JSON number",
        );
        assert_eq!(show(&db, "1"), before);
    }
}
