mod support;
use serde_json::{Value, json};
use std::process::{Command, Output};
use tempfile::TempDir;
fn run(db: &TempDir, args: &[&str]) -> Output {
    support::empty_database(&db.path().join("db"));
    Command::new(env!("CARGO_BIN_EXE_precision"))
        .arg("--db")
        .arg(db.path().join("db"))
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
fn show(db: &TempDir, kind: &str, id: &str) -> Value {
    serde_json::from_str(&ok(db, &[kind, "show", id, "--json"])).unwrap()
}
fn save(db: &TempDir, args: &[&str], doc: &Value) -> Output {
    let path = db.path().join("input.json");
    std::fs::write(&path, serde_json::to_vec(doc).unwrap()).unwrap();
    let mut args = args.to_vec();
    args.extend(["--file", path.to_str().unwrap()]);
    run(db, &args)
}
fn saved(db: &TempDir, args: &[&str], doc: &Value) {
    let out = save(db, args, doc);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
fn setup() -> TempDir {
    let db = TempDir::new().unwrap();
    ok(
        &db,
        &[
            "exercise",
            "create",
            "--name",
            "Squat",
            "--measurement",
            "repetitions",
            "--load-convention",
            "external",
        ],
    );
    db
}
#[test]
fn prescribed_rest_and_noncontiguous_supersets_survive_copy_and_deletion() {
    let db = setup();
    saved(
        &db,
        &["routine", "create"],
        &json!({"schema_version":1,"name":"Source","rest":[0,null,1.25],"sets":[
        {"portions":[{"exercise_id":1}]},{"portions":[{"exercise_id":1}]},
        {"portions":[{"exercise_id":1},{"exercise_id":1}]},{"portions":[{"exercise_id":1}]}]}),
    );
    let mut source = show(&db, "routine", "1");
    source["supersets"] = json!([{"set_ids":[source["sets"][0]["id"],source["sets"][2]["id"],source["sets"][3]["id"]]}]);
    saved(&db, &["routine", "update", "1"], &source);
    let source = show(&db, "routine", "1");
    ok(
        &db,
        &["workout", "start", "--date", "2026-10-05", "--routine", "1"],
    );
    let copy = show(&db, "workout", "1");
    assert_eq!(copy["intention"]["rest"], json!([0, null, 1.25]));
    assert_eq!(
        copy["intention"]["supersets"][0]["set_ids"],
        json!([
            copy["intention"]["sets"][0]["id"],
            copy["intention"]["sets"][2]["id"],
            copy["intention"]["sets"][3]["id"]
        ])
    );
    let mut edited = source.clone();
    edited["rest"] = json!([10, 20, 30]);
    edited["supersets"] = json!([]);
    saved(&db, &["routine", "update", "1"], &edited);
    assert_eq!(show(&db, "workout", "1"), copy);
    ok(&db, &["routine", "delete", "1"]);
    assert_eq!(show(&db, "workout", "1")["intention"], copy["intention"]);
}

#[test]
fn actual_and_intended_organization_are_independent_and_invalid_updates_roll_back() {
    let db = setup();
    ok(&db, &["workout", "start", "--date", "2026-10-05"]);
    saved(
        &db,
        &["workout", "intention", "update", "1"],
        &json!({"schema_version":1,"rest":[0,null,2.5],"sets":[{"portions":[{"exercise_id":1}]},{"portions":[{"exercise_id":1}]},{"portions":[{"exercise_id":1}]},{"portions":[{"exercise_id":1}]}]}),
    );
    saved(
        &db,
        &["workout", "update", "1"],
        &json!({"schema_version":1,"date":"2026-10-05","rest":[null,0,0.000001],"sets":[{"portions":[{"exercise_id":1,"repetitions":0}]},{"portions":[{"exercise_id":1,"repetitions":2}]},{"portions":[{"exercise_id":1,"repetitions":3},{"exercise_id":1,"repetitions":4}]},{"portions":[{"exercise_id":1,"repetitions":5}]}]}),
    );
    let current = show(&db, "workout", "1");
    let mut intended = current["intention"].clone();
    intended["supersets"] =
        json!([{"set_ids":[intended["sets"][0]["id"],intended["sets"][2]["id"]]}]);
    saved(&db, &["workout", "intention", "update", "1"], &intended);
    let mut actual: Value = serde_json::from_str(&ok(
        &db,
        &["workout", "show", "1", "--json", "--actual-only"],
    ))
    .unwrap();
    let a = actual["sets"][0]["id"].clone();
    let c = actual["sets"][2]["id"].clone();
    let d = actual["sets"][3]["id"].clone();
    actual["supersets"] = json!([{"set_ids":[a,c,d]}]);
    saved(&db, &["workout", "update", "1"], &actual);
    let before = show(&db, "workout", "1");
    assert_eq!(before["rest"], json!([null, 0, 0.000001]));
    assert_eq!(before["intention"]["rest"], json!([0, null, 2.5]));
    actual = serde_json::from_str(&ok(
        &db,
        &["workout", "show", "1", "--json", "--actual-only"],
    ))
    .unwrap();
    for (field, value) in [
        ("rest", json!(null)),
        ("rest", json!([])),
        ("rest", json!([0, 0, 0, 0])),
        ("rest", json!([-1, 0, null])),
        ("rest", json!(["1", 0, null])),
        ("rest", json!([0.0000001, 0, null])),
        ("rest", json!([{"$serde_json::private::Number":"1"},0,null])),
        ("supersets", json!([{"set_ids":[a]}])),
        ("supersets", json!([{"set_ids":[a,a]}])),
        ("supersets", json!([{"set_ids":[a,99999]}])),
        ("supersets", json!([{"set_ids":[a,c]},{"set_ids":[c,d]}])),
        ("supersets", json!([{"id":99999,"set_ids":[a,c]}])),
        ("supersets", json!([{"set_ids":[a,c],"supersets":[]}])),
    ] {
        let mut invalid = actual.clone();
        invalid["notes"] = json!("rollback");
        invalid[field] = value;
        assert!(
            !save(&db, &["workout", "update", "1"], &invalid)
                .status
                .success(),
            "{invalid}"
        );
        assert_eq!(show(&db, "workout", "1"), before);
    }
    let mut reordered = actual.clone();
    reordered["sets"].as_array_mut().unwrap().reverse();
    reordered.as_object_mut().unwrap().remove("rest");
    assert!(
        !save(&db, &["workout", "update", "1"], &reordered)
            .status
            .success()
    );
    assert_eq!(show(&db, "workout", "1"), before);
    reordered["rest"] = json!([3, 2, 1]);
    saved(&db, &["workout", "update", "1"], &reordered);
    let after = show(&db, "workout", "1");
    assert_eq!(after["intention"], before["intention"]);
    assert_eq!(after["rest"], json!([3, 2, 1]));
    assert_eq!(after["supersets"], before["supersets"]);
    let human = ok(&db, &["workout", "show", "1"]);
    assert!(human.contains("Minimum rest after set 1: 0 seconds"));
    assert!(human.contains("Observed rest after set 1: 3 seconds"));
    assert!(human.contains("Actual superset ID"));
    ok(&db, &["workout", "finish", "1"]);
    assert_eq!(show(&db, "workout", "1")["rest"], json!([3, 2, 1]));
}

#[test]
fn prescribed_validation_is_owner_local_and_reorder_requires_replacement_rest() {
    let db = setup();
    for _ in 0..2 {
        saved(
            &db,
            &["routine", "create"],
            &json!({"schema_version":1,"name":"Routine","rest":[null,0],"sets":[{"portions":[{"exercise_id":1}]},{"portions":[{"exercise_id":1}]},{"portions":[{"exercise_id":1}]}]}),
        );
    }
    let foreign = show(&db, "routine", "2");
    let mut source = show(&db, "routine", "1");
    source["supersets"] = json!([{"set_ids":[source["sets"][0]["id"],source["sets"][2]["id"]]}]);
    saved(&db, &["routine", "update", "1"], &source);
    for _ in 0..2 {
        ok(
            &db,
            &["workout", "start", "--date", "2026-10-05", "--routine", "1"],
        );
    }
    let before = show(&db, "routine", "1");
    let intention = show(&db, "workout", "1")["intention"].clone();
    let foreign_intention = show(&db, "workout", "2")["intention"].clone();
    for (document, args, foreign_sets, foreign_group) in [
        (
            before.clone(),
            vec!["routine", "update", "1"],
            foreign["sets"].clone(),
            json!(99999),
        ),
        (
            intention.clone(),
            vec!["workout", "intention", "update", "1"],
            foreign_intention["sets"].clone(),
            foreign_intention["supersets"][0]["id"].clone(),
        ),
    ] {
        let a = document["sets"][0]["id"].clone();
        let c = document["sets"][2]["id"].clone();
        for (field, value) in [
            ("rest", json!([1])),
            ("rest", json!([null, -0.5])),
            ("supersets", json!([{"set_ids":[a,foreign_sets[0]["id"]]}])),
            ("supersets", json!([{"id":foreign_group,"set_ids":[a,c]}])),
            ("supersets", json!([{"set_ids":[a,c]},{"set_ids":[c,a]}])),
        ] {
            let mut invalid = document.clone();
            invalid[field] = value;
            assert!(!save(&db, &args, &invalid).status.success());
            assert_eq!(show(&db, "routine", "1"), before);
            assert_eq!(show(&db, "workout", "1")["intention"], intention);
        }
        let mut reordered = document.clone();
        reordered["sets"].as_array_mut().unwrap().reverse();
        reordered.as_object_mut().unwrap().remove("rest");
        assert!(!save(&db, &args, &reordered).status.success());
        reordered["rest"] = json!([1.5, 0]);
        saved(&db, &args, &reordered);
        // Restore for the next aggregate's isolation checks.
        saved(&db, &args, &document);
    }
    let human = ok(&db, &["routine", "show", "1"]);
    assert!(human.contains("Minimum rest after set 1: unspecified seconds"));
    assert!(human.contains("Prescribed superset ID"));
}

#[test]
fn zero_and_one_set_sequences_have_no_transitions() {
    let db = setup();
    ok(&db, &["workout", "start", "--date", "2026-10-05"]);
    assert_eq!(show(&db, "workout", "1")["rest"], json!([]));
    assert_eq!(show(&db, "workout", "1")["intention"]["rest"], json!([]));
    let one = json!({"schema_version":1,"date":"2026-10-05","rest":[],"sets":[{"portions":[{"exercise_id":1,"repetitions":0}]}]});
    saved(&db, &["workout", "update", "1"], &one);
    let mut invalid = one.clone();
    invalid["rest"] = json!([0]);
    assert!(
        !save(&db, &["workout", "update", "1"], &invalid)
            .status
            .success()
    );
    ok(&db, &["workout", "finish", "1"]);
    assert_eq!(show(&db, "workout", "1")["rest"], json!([]));
}

#[test]
fn version_four_data_migrates_to_unknown_rest_without_changing_activity() {
    let db = TempDir::new().unwrap();
    // A pre-feature database is input to the CLI migration boundary.
    let connection = rusqlite::Connection::open(db.path().join("db")).unwrap();
    for sql in [
        include_str!("../migrations/0001.sql"),
        include_str!("../migrations/0002.sql"),
        include_str!("../migrations/0003.sql"),
        include_str!("../migrations/0004.sql"),
    ] {
        connection.execute_batch(sql).unwrap();
    }
    connection.execute_batch("INSERT INTO exercises(id,name,name_key,measurement,load_convention) VALUES (1,'Squat','squat','repetitions','external');
        INSERT INTO routines(id,name) VALUES(1,'Old routine');
        INSERT INTO workouts(id,state,date) VALUES(1,'draft','2026-10-05');
        INSERT INTO prescribed_sets(id,routine_id,position,type) VALUES(1,1,0,'main'),(2,1,1,'warmup'),(3,1,2,'main');
        INSERT INTO prescribed_portions(set_id,position,exercise_id,repetitions) VALUES(1,0,1,'5'),(2,0,1,'0'),(3,0,1,NULL);
        INSERT INTO intention_sets(id,workout_id,position,type) VALUES(1,1,0,'main'),(2,1,1,'main');
        INSERT INTO intention_portions(set_id,position,exercise_id,repetitions) VALUES(1,0,1,'3'),(2,0,1,NULL);
        INSERT INTO performed_sets(id,workout_id,position,type) VALUES(1,1,0,'main'),(2,1,1,'main');
        INSERT INTO performed_portions(set_id,position,exercise_id,repetitions) VALUES(1,0,1,'0'),(2,0,1,'4');
        PRAGMA user_version=4;").unwrap();
    drop(connection);
    let routine = show(&db, "routine", "1");
    assert_eq!(routine["rest"], json!([null, null]));
    assert_eq!(routine["sets"][1]["type"], "warmup");
    assert_eq!(routine["sets"][0]["portions"][0]["repetitions"], 5);
    let workout = show(&db, "workout", "1");
    assert_eq!(workout["rest"], json!([null]));
    assert_eq!(workout["intention"]["rest"], json!([null]));
    assert_eq!(workout["supersets"], json!([]));
    assert_eq!(workout["intention"]["supersets"], json!([]));
    assert_eq!(workout["sets"][0]["portions"][0]["repetitions"], 0);
    assert_eq!(show(&db, "workout", "1"), workout);
}
