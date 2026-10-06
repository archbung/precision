mod support;
use serde_json::{Value, json};
use std::process::{Command, Output};
use tempfile::TempDir;
fn run(db: &TempDir, args: &[&str]) -> Output {
    support::empty_database(&db.path().join("db"));
    Command::new(env!("CARGO_BIN_EXE_precision"))
        .arg("--db")
        .arg(db.path().join("db"))
        .args(support::with_workout_revision(&db.path().join("db"), args))
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
fn five_intended_sets_and_four_actual_sets_remain_independent() {
    let db = setup();
    saved(
        &db,
        &["routine", "create"],
        &json!({"schema_version":1,"name":"Five","notes":"routine instruction","sets":(0..5).map(|_| json!({"portions":[{"exercise_id":1,"repetitions":5}]})).collect::<Vec<_>>()}),
    );
    ok(
        &db,
        &["workout", "start", "--date", "2026-10-05", "--routine", "1"],
    );
    saved(
        &db,
        &["workout", "update", "1"],
        &json!({"schema_version":1,"date":"2026-10-05","notes":"session note","sets":(0..4).map(|_| json!({"portions":[{"exercise_id":1,"repetitions":9,"notes":"left 10, right 9; ninth unsuccessful"}]})).collect::<Vec<_>>()}),
    );
    ok(&db, &["workout", "finish", "1"]);
    let before = show(&db, "workout", "1");
    let output = ok(&db, &["workout", "compare", "1"]);
    let grouped = output
        .split("Full intended sequence and rest")
        .next()
        .unwrap();
    assert!(grouped.contains("5 sets, 5 portions"));
    assert!(grouped.contains("4 sets, 4 portions"));
    assert_eq!(grouped.matches("minimum successful repetitions").count(), 5);
    assert_eq!(grouped.matches("attempted repetitions").count(), 4);
    for text in [
        "routine instruction",
        "session note",
        "ninth unsuccessful",
        "Independent lists",
        "Full actual sequence and rest",
    ] {
        assert!(output.contains(text), "{output}");
    }
    assert_eq!(ok(&db, &["workout", "compare", "1"]), output);
    assert_eq!(show(&db, "workout", "1"), before);
    let missing = run(&db, &["workout", "compare", "9999"]);
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
}

#[test]
fn complex_groups_preserve_positions_shared_observations_and_independent_sequences() {
    let db = setup();
    for (name, mode) in [
        ("Hold", "duration"),
        ("Carry", "distance"),
        ("Pull-up", "repetitions"),
    ] {
        ok(
            &db,
            &[
                "exercise",
                "create",
                "--name",
                name,
                "--measurement",
                mode,
                "--load-convention",
                "added-bodyweight",
            ],
        );
    }
    ok(&db, &["workout", "start", "--date", "2026-10-05"]);
    saved(
        &db,
        &["workout", "intention", "update", "1"],
        &json!({"schema_version":1,"notes":"today's intention","rest":[0,null,1.25],"sets":[
            {"type":"warmup","kilograms":0,"portions":[{"exercise_id":1,"repetitions":0}]},
            {"kilograms":12.5,"rpe":8.5,"load_description":"shared bar","notes":"one complex","portions":[{"exercise_id":1,"repetitions":3},{"exercise_id":2,"seconds":null},{"exercise_id":1,"repetitions":null,"notes":"repeat movement"}]},
            {"portions":[{"exercise_id":3,"metres":0}]},
            {"portions":[{"exercise_id":1,"repetitions":5}]}
        ]}),
    );
    saved(
        &db,
        &["workout", "update", "1"],
        &json!({"schema_version":1,"date":"2026-10-05","rest":[null,0],"sets":[
            {"kilograms":0,"rpe":9,"white_flags":1,"red_flags":2,"notes":"completed movement; red for pressout","portions":[{"exercise_id":1,"repetitions":4},{"exercise_id":1,"repetitions":0,"notes":"left 1 right 0; failed attempt"}]},
            {"type":"warmup","kilograms":-20,"load_description":"purple band","portions":[{"exercise_id":4,"repetitions":2}]},
            {"portions":[{"exercise_id":1,"repetitions":6}]}
        ]}),
    );
    let before = show(&db, "workout", "1");
    let mut intention = before["intention"].clone();
    intention["supersets"] =
        json!([{"set_ids":[intention["sets"][0]["id"],intention["sets"][3]["id"]]}]);
    saved(&db, &["workout", "intention", "update", "1"], &intention);
    let mut actual: Value = serde_json::from_str(&ok(
        &db,
        &["workout", "show", "1", "--json", "--actual-only"],
    ))
    .unwrap();
    actual["supersets"] = json!([{"set_ids":[actual["sets"][0]["id"],actual["sets"][2]["id"]]}]);
    saved(&db, &["workout", "update", "1"], &actual);
    let output = ok(&db, &["workout", "compare", "1"]);
    let sections: Vec<_> = output.split("\nExercise ").skip(1).collect();
    let headers: Vec<_> = sections.iter().map(|s| s.lines().next().unwrap()).collect();
    assert_eq!(
        headers,
        vec![
            "1: Squat / warmup",
            "1: Squat / main",
            "2: Hold / main",
            "3: Carry / main",
            "4: Pull-up / warmup"
        ]
    );
    let main = sections[1];
    assert_eq!(main.matches("2 sets, 3 portions").count(), 2);
    for text in [
        "Selected portion 1",
        "Selected portion 3",
        "minimum successful repetitions (each side if unilateral): unspecified",
        "maximum RPE 8.5",
        "minimum kilograms 12.5",
        "Shared set observation/setup",
        "shared bar",
        "one complex",
        "repeat movement",
        "failed judgment; independent of physical completion",
        "completed movement; red for pressout",
        "left 1 right 0; failed attempt",
    ] {
        assert!(main.contains(text), "missing {text}: {main}");
    }
    assert!(sections[0].contains("No recorded activity"));
    assert!(sections[2].contains("minimum seconds: unspecified"));
    assert!(sections[3].contains("minimum metres: 0"));
    assert!(sections[4].contains("No prescribed activity"));
    assert!(sections[4].contains("kilograms -20"));
    assert!(sections[4].contains("purple band"));
    let record = show(&db, "workout", "1");
    for (owner, label) in [(&record["intention"], "Prescribed"), (&record, "Actual")] {
        let group = &owner["supersets"][0];
        assert!(output.contains(&format!(
            "{label} superset ID {}: set IDs [{}, {}]",
            group["id"], group["set_ids"][0], group["set_ids"][1]
        )));
        for (index, set) in owner["sets"].as_array().unwrap().iter().enumerate() {
            assert!(output.contains(&format!("Set {} (ID {},", index + 1, set["id"])));
            for (index, portion) in set["portions"].as_array().unwrap().iter().enumerate() {
                assert!(output.contains(&format!("Portion {} (ID {}),", index + 1, portion["id"])));
            }
        }
    }
    let (intended_sequence, actual_sequence) = output
        .split_once("Full intended sequence and rest")
        .unwrap()
        .1
        .split_once("Full actual sequence and rest")
        .unwrap();
    for text in [
        "Minimum rest after set 1: 0 seconds",
        "Minimum rest after set 2: unspecified seconds",
        "Minimum rest after set 3: 1.25 seconds",
    ] {
        assert!(intended_sequence.contains(text));
    }
    for text in [
        "Observed rest after set 1: unknown seconds",
        "Observed rest after set 2: 0 seconds",
    ] {
        assert!(actual_sequence.contains(text));
    }
    assert_eq!(show(&db, "workout", "1"), record);
}

#[test]
fn empty_drafts_and_missing_intention_remain_inspectable() {
    let db = setup();
    ok(&db, &["workout", "start", "--date", "2026-10-05"]);
    let empty = ok(&db, &["workout", "compare", "1"]);
    assert!(empty.contains("draft"));
    assert!(empty.contains("No prescribed activity"));
    assert!(empty.contains("No recorded activity"));
    saved(
        &db,
        &["workout", "update", "1"],
        &json!({"schema_version":1,"date":"2026-10-05","sets":[{"portions":[{"exercise_id":1,"repetitions":0}]}]}),
    );
    let output = ok(&db, &["workout", "compare", "1"]);
    assert!(output.contains("0 sets, 0 portions"));
    assert!(output.contains("1 sets, 1 portions"));
    assert!(output.contains("attempted repetitions (minimum of sides if unilateral): 0"));
    assert!(output.contains("kilograms unspecified"));
}
