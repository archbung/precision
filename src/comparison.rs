use crate::{exercises::Store, organization::Superset, workouts::Workout};
use serde_json::Number;
use std::collections::HashSet;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// A presentation view of either independent sequence, never a correspondence.
struct Portion<'a> {
    id: i64,
    exercise: i64,
    quantity: &'a Option<Number>,
    label: &'static str,
    notes: Option<&'a str>,
}
struct Set<'a> {
    id: i64,
    kind: &'a str,
    kilograms: &'a Option<Number>,
    rpe: &'a Option<Number>,
    description: Option<&'a str>,
    notes: Option<&'a str>,
    flags: Option<(u8, u8)>,
    portions: Vec<Portion<'a>>,
}
fn presentation_portion<'a>(
    store: &Store,
    id: Option<i64>,
    exercise: i64,
    quantities: (&'a Option<Number>, &'a Option<Number>, &'a Option<Number>),
    notes: Option<&'a str>,
    intended: bool,
) -> Result<Portion<'a>> {
    let (repetitions, seconds, metres) = quantities;
    let mode = store.show(exercise)?.measurement;
    use crate::exercise_types::Measurement::*;
    let (quantity, label) = match (mode, intended) {
        (Repetitions, true) => (
            repetitions,
            "minimum successful repetitions (each side if unilateral)",
        ),
        (Repetitions, false) => (
            repetitions,
            "attempted repetitions (minimum of sides if unilateral)",
        ),
        (Duration, true) => (seconds, "minimum seconds"),
        (Duration, false) => (seconds, "seconds"),
        (Distance, true) => (metres, "minimum metres"),
        (Distance, false) => (metres, "metres"),
    };
    Ok(Portion {
        id: id.unwrap(),
        exercise,
        quantity,
        label,
        notes,
    })
}
fn value(number: &Option<Number>) -> String {
    number
        .as_ref()
        .map_or("unspecified".into(), ToString::to_string)
}
fn note(lines: &mut Vec<String>, label: &str, text: Option<&str>) {
    if let Some(text) = text {
        // Keep embedded newlines within their column.
        lines.extend(text.lines().map(|line| format!("{label}: {line}")));
    }
}
fn entry(
    store: &Store,
    set: &Set<'_>,
    position: usize,
    selected: Option<usize>,
    groups: &[Superset],
    intended: bool,
) -> Result<Vec<String>> {
    let mut lines = vec![format!(
        "Set {} (ID {}, {})",
        position + 1,
        set.id,
        set.kind
    )];
    if let Some(portion) = selected {
        lines.push(format!(
            "Selected portion {} (ID {})",
            portion + 1,
            set.portions[portion].id
        ));
    }
    lines.push(format!(
        "Shared set observation/setup: {}kilograms {}; {}RPE {}",
        if intended { "minimum " } else { "" },
        value(set.kilograms),
        if intended { "maximum " } else { "" },
        value(set.rpe)
    ));
    note(&mut lines, "Load description", set.description);
    note(&mut lines, "Whole-set notes", set.notes);
    if let Some((white, red)) = set.flags {
        lines.push(format!(
            "Judging: {white} white, {red} red{}",
            if red > white {
                " (failed judgment; independent of physical completion)"
            } else {
                " (independent of physical completion)"
            }
        ));
    }
    lines.push(
        groups
            .iter()
            .find(|group| group.set_ids.contains(&set.id))
            .map_or("Superset: none".into(), |group| {
                format!(
                    "{} superset ID {}",
                    if intended { "Prescribed" } else { "Actual" },
                    group.id.unwrap()
                )
            }),
    );
    lines.push("Full composition (ordered portions):".into());
    for (index, portion) in set.portions.iter().enumerate() {
        let exercise = store.show(portion.exercise)?;
        lines.push(format!(
            "  Portion {} (ID {}), exercise {}: {} [{} load]",
            index + 1,
            portion.id,
            exercise.id,
            exercise.name,
            exercise.load_convention
        ));
        // In grouped entries, quantities of other exercises remain in the full sequence.
        if selected.is_none_or(|selected| selected == index) {
            lines.push(format!("  {}: {}", portion.label, value(portion.quantity)));
            note(&mut lines, "  Portion notes", portion.notes);
        }
    }
    Ok(lines)
}
fn list(
    store: &Store,
    sets: &[Set<'_>],
    groups: &[Superset],
    key: &(i64, String),
    intended: bool,
) -> Result<Vec<String>> {
    let mut lines = vec![];
    let mut count = 0;
    let mut portions = 0;
    for (index, set) in sets.iter().enumerate() {
        let matches: Vec<_> = set
            .portions
            .iter()
            .enumerate()
            .filter(|(_, portion)| portion.exercise == key.0 && set.kind == key.1)
            .map(|(index, _)| index)
            .collect();
        if !matches.is_empty() {
            count += 1;
        }
        for portion in matches {
            portions += 1;
            lines.extend(entry(store, set, index, Some(portion), groups, intended)?);
            lines.push(String::new());
        }
    }
    lines.insert(0, format!("{count} sets, {portions} portions"));
    if count == 0 {
        lines.push(format!(
            "No {} activity",
            if intended { "prescribed" } else { "recorded" }
        ));
    }
    Ok(lines)
}
pub fn print(store: &Store, workout: &Workout) -> Result<()> {
    let mut intended = vec![];
    for set in &workout.intention.sets {
        let mut portions = vec![];
        for portion in &set.portions {
            portions.push(presentation_portion(
                store,
                portion.id,
                portion.exercise_id,
                (&portion.repetitions, &portion.seconds, &portion.metres),
                portion.notes.as_deref(),
                true,
            )?);
        }
        intended.push(Set {
            id: set.id.unwrap(),
            kind: &set.kind,
            kilograms: &set.kilograms,
            rpe: &set.rpe,
            description: set.load_description.as_deref(),
            notes: set.notes.as_deref(),
            flags: None,
            portions,
        });
    }
    let mut actual = vec![];
    for set in &workout.sets {
        let mut portions = vec![];
        for portion in &set.portions {
            portions.push(presentation_portion(
                store,
                portion.id,
                portion.exercise_id,
                (&portion.repetitions, &portion.seconds, &portion.metres),
                portion.notes.as_deref(),
                false,
            )?);
        }
        actual.push(Set {
            id: set.id.unwrap(),
            kind: &set.kind,
            kilograms: &set.kilograms,
            rpe: &set.rpe,
            description: set.load_description.as_deref(),
            notes: set.notes.as_deref(),
            flags: set.white_flags.zip(set.red_flags),
            portions,
        });
    }
    println!(
        "Workout {}: {} ({})",
        workout.id.unwrap(),
        workout.state,
        workout.date
    );
    println!(
        "Start: {}; end: {}",
        workout.start.as_deref().unwrap_or("unspecified"),
        workout.end.as_deref().unwrap_or("unspecified")
    );
    println!(
        "Routine context: {} (original ID {}); current source ID {}",
        workout.original_source_name.as_deref().unwrap_or("none"),
        workout
            .original_source_id
            .map_or("none".into(), |id| id.to_string()),
        workout
            .source_routine_id
            .map_or("none".into(), |id| id.to_string())
    );
    let mut notes = vec![];
    note(&mut notes, "Session notes", workout.notes.as_deref());
    note(
        &mut notes,
        "Preserved routine/intention notes",
        workout.intention.notes.as_deref(),
    );
    for line in notes {
        println!("{line}");
    }
    println!(
        "Independent lists: visual rows imply no set pairing. Shared observations repeated for portions describe one set. Unspecified actual values are unknown; unspecified targets give no numeric instruction. Zero is explicit."
    );
    let mut keys = vec![];
    let mut seen = HashSet::new();
    for set in intended.iter().chain(&actual) {
        for portion in &set.portions {
            let key = (portion.exercise, set.kind.to_owned());
            if seen.insert(key.clone()) {
                keys.push(key);
            }
        }
    }
    for key in keys {
        println!(
            "\nExercise {}: {} / {}",
            key.0,
            store.show(key.0)?.name,
            key.1
        );
        let left = list(store, &intended, &workout.intention.supersets, &key, true)?;
        let right = list(store, &actual, &workout.supersets, &key, false)?;
        let width = left
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0)
            .max(10);
        println!("{:<width$} | Actual", "Prescribed");
        for index in 0..left.len().max(right.len()) {
            println!(
                "{:<width$} | {}",
                left.get(index).map_or("", String::as_str),
                right.get(index).map_or("", String::as_str)
            );
        }
    }
    for (title, sets, rest, groups, prescribed) in [
        (
            "Full intended sequence and rest",
            &intended,
            &workout.intention.rest,
            &workout.intention.supersets,
            true,
        ),
        (
            "Full actual sequence and rest",
            &actual,
            &workout.rest,
            &workout.supersets,
            false,
        ),
    ] {
        println!("\n{title}:");
        if sets.is_empty() {
            println!(
                "No {} activity",
                if prescribed { "prescribed" } else { "recorded" }
            );
        }
        for (position, set) in sets.iter().enumerate() {
            for line in entry(store, set, position, None, groups, prescribed)? {
                println!("{line}");
            }
        }
        crate::organization::print(rest, groups, prescribed);
    }
    Ok(())
}
