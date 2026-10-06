use precision::exercises::Store;
use serde_json::json;

#[test]
fn saved_activity_requires_a_current_revision_and_rejects_stale_replacements() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("revision.db");
    let mut first = Store::open(&path).unwrap();
    let draft = first
        .start_workout("2026-10-06".into(), None, None)
        .unwrap();
    let mut document = serde_json::to_value(&draft).unwrap();
    document.as_object_mut().unwrap().remove("revision");
    let missing = first
        .save_workout(1, serde_json::from_value(document.clone()).unwrap())
        .err()
        .unwrap()
        .to_string();
    assert!(missing.contains("revision"), "{missing}");
    document["revision"] = json!(0);
    document["notes"] = json!("first save");
    first
        .save_workout(1, serde_json::from_value(document.clone()).unwrap())
        .unwrap();
    let mut second = Store::open(&path).unwrap();
    document["notes"] = json!("stale overwrite");
    let error = second
        .save_workout(1, serde_json::from_value(document).unwrap())
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("stale"), "{error}");
    assert_eq!(
        first.workout(1).unwrap().notes.as_deref(),
        Some("first save")
    );
}

#[test]
fn intention_changes_invalidate_actual_editors_without_losing_saved_activity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("intention.db");
    let mut store = Store::open(&path).unwrap();
    let stale = store
        .start_workout("2026-10-06".into(), None, None)
        .unwrap();
    let intention =
        serde_json::from_value(json!({"schema_version":1,"notes":"new intention","sets":[]}))
            .unwrap();
    store.save_intention(1, 0, intention).unwrap();
    assert!(store.save_workout(1, stale).is_err());
    assert!(store.discard_workout(1, 0).is_err());
    assert!(store.finish_workout(1, 0, None).is_err());
    let current = store.workout(1).unwrap();
    assert_eq!(current.intention.notes.as_deref(), Some("new intention"));
    assert_eq!(current.revision, Some(1));
    let stale_intention = serde_json::from_value(json!({"schema_version":1,"sets":[]})).unwrap();
    assert!(store.save_intention(1, 0, stale_intention).is_err());
    store.discard_workout(1, 1).unwrap();
    assert!(store.workout(1).is_err());
}

#[test]
fn cli_exports_revisions_and_missing_invalid_or_stale_contract_values_fail_with_guidance() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cli.db");
    let run = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_precision"))
            .arg("--db")
            .arg(&path)
            .args(args)
            .output()
            .unwrap()
    };
    assert!(
        run(&["workout", "start", "--date", "2026-10-06"])
            .status
            .success()
    );
    let export = || {
        serde_json::from_slice::<serde_json::Value>(
            &run(&["workout", "show", "1", "--json", "--actual-only"]).stdout,
        )
        .unwrap()
    };
    let initial = export();
    assert_eq!(initial["revision"], 0);
    let file = dir.path().join("actual.json");
    for revision in [
        None,
        Some(json!(null)),
        Some(json!(-1)),
        Some(json!(0.5)),
        Some(json!("0")),
        Some(json!(1)),
    ] {
        let mut document = initial.clone();
        if let Some(revision) = revision {
            document["revision"] = revision;
        } else {
            document.as_object_mut().unwrap().remove("revision");
        }
        std::fs::write(&file, serde_json::to_vec(&document).unwrap()).unwrap();
        let out = run(&["workout", "update", "1", "--file", file.to_str().unwrap()]);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("revision"));
        assert_eq!(export(), initial);
    }
    std::fs::write(&file, serde_json::to_vec(&initial).unwrap()).unwrap();
    assert!(
        run(&["workout", "update", "1", "--file", file.to_str().unwrap()])
            .status
            .success()
    );
    assert_eq!(export()["revision"], 1);
    let out = run(&["workout", "update", "1", "--file", file.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("workout show"));
    for command in ["finish", "discard"] {
        assert!(!run(&["workout", command, "1"]).status.success());
        assert!(
            !run(&["workout", command, "1", "--revision", "0"])
                .status
                .success()
        );
    }
    assert!(
        run(&["workout", "discard", "1", "--revision", "1"])
            .status
            .success()
    );
}
