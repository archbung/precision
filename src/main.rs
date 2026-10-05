mod aggregate_values;
mod exercise_types;
mod exercises;
mod routines;
mod workouts;
use clap::{Args, Parser, Subcommand, ValueEnum};
use exercise_types::{LoadConvention, Measurement};
use exercises::{Exercise, Store};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about = "Define exercises, prescribe routines, and record workouts",
    after_help = "Default database: $HOME/.precision/precision.sqlite3. Use --db PATH to override."
)]
struct Cli {
    #[arg(long, global = true)]
    db: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Workout {
        #[command(subcommand)]
        command: WorkoutCommand,
    },
    Routine {
        #[command(subcommand)]
        command: RoutineCommand,
    },
    Exercise {
        #[command(subcommand)]
        command: ExerciseCommand,
    },
    Catalog {
        #[arg(value_enum)]
        kind: Catalog,
    },
}
#[derive(Subcommand)]
#[command(
    about = "Reusable ordered prescriptions",
    after_help = "Create/update use --file PATH with schema_version: 1, name, optional notes, and nonempty sets. Sets contain ordered portions referencing exercise_id. See README.md for JSON examples and decimal precision. Rest and supersets are not yet supported."
)]
enum RoutineCommand {
    Create {
        #[arg(long)]
        file: PathBuf,
    },
    List {
        #[arg(long)]
        json: bool,
    },
    Show {
        id: i64,
        #[arg(long)]
        json: bool,
    },
    Update {
        id: i64,
        #[arg(long)]
        file: PathBuf,
    },
    Delete {
        id: i64,
    },
}
#[derive(Subcommand)]
#[command(
    about = "Standalone resumable workout drafts",
    after_help = "Update replaces metadata and actual sets using --file PATH; show --json exports an editable schema_version: 1 document. Completed workouts are read-only. See README.md for schema. Rest and groups are not yet supported."
)]
enum WorkoutCommand {
    Start {
        #[arg(long)]
        date: String,
        #[arg(long)]
        start: Option<String>,
    },
    List {
        #[arg(long)]
        drafts: bool,
        #[arg(long)]
        json: bool,
    },
    Show {
        id: i64,
        #[arg(long)]
        json: bool,
    },
    Update {
        id: i64,
        #[arg(long)]
        file: PathBuf,
    },
    Finish {
        id: i64,
        #[arg(long)]
        end: Option<String>,
    },
    Discard {
        id: i64,
    },
}
#[derive(Clone, Copy, ValueEnum)]
enum Catalog {
    Equipment,
    Muscles,
}
#[derive(Subcommand)]
enum ExerciseCommand {
    Create {
        #[arg(long)]
        name: String,
        #[arg(long, value_enum)]
        measurement: Measurement,
        #[arg(long, value_enum)]
        load_convention: LoadConvention,
        #[arg(long)]
        equipment: Vec<i64>,
        #[command(flatten)]
        muscles: Muscles,
    },
    List {
        #[arg(long)]
        json: bool,
    },
    Show {
        id: i64,
        #[arg(long)]
        json: bool,
    },
    Update {
        id: i64,
        #[arg(long)]
        name: Option<String>,
        #[command(flatten)]
        muscles: Muscles,
        #[arg(long, conflicts_with = "primary_muscle")]
        clear_primary_muscle: bool,
        #[arg(long, conflicts_with = "secondary_muscle")]
        clear_secondary_muscles: bool,
    },
}
#[derive(Args, Default)]
struct Muscles {
    #[arg(long)]
    primary_muscle: Option<i64>,
    #[arg(long)]
    secondary_muscle: Vec<i64>,
}
impl ValueEnum for Measurement {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Repetitions, Self::Duration, Self::Distance]
    }
    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.as_str()))
    }
}
impl ValueEnum for LoadConvention {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::External, Self::AddedBodyweight]
    }
    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.as_str()))
    }
}
fn print_exercise(
    store: &Store,
    exercise: &Exercise,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        println!("{}", serde_json::to_string_pretty(exercise)?);
    } else {
        let equipment = store.catalog("equipment")?;
        let muscles = store.catalog("muscles")?;
        let label = |catalog: &[(i64, String)], id: i64| {
            catalog
                .iter()
                .find(|(key, _)| *key == id)
                .map_or_else(|| id.to_string(), |(_, name)| format!("{id}: {name}"))
        };
        println!(
            "{}: {}\n  Measurement: {}\n  Load convention: {}\n  Equipment IDs: {:?}\n  Primary muscle ID: {}\n  Secondary muscle IDs: {:?}",
            exercise.id,
            exercise.name,
            exercise.measurement,
            exercise.load_convention,
            exercise
                .equipment
                .iter()
                .map(|id| label(&equipment, *id))
                .collect::<Vec<_>>(),
            exercise
                .primary_muscle
                .map_or("unspecified".into(), |id| label(&muscles, id)),
            exercise
                .secondary_muscles
                .iter()
                .map(|id| label(&muscles, *id))
                .collect::<Vec<_>>()
        );
    }
    Ok(())
}
fn print_routine(
    store: &Store,
    routine: &routines::Routine,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        println!("{}", serde_json::to_string_pretty(routine)?);
    } else {
        println!("{}: {}", routine.id.unwrap(), routine.name);
        if let Some(notes) = &routine.notes {
            println!("  Routine notes: {notes}");
        }
        let quantity = |value: &Option<serde_json::Number>| {
            value
                .as_ref()
                .map_or("unspecified".into(), ToString::to_string)
        };
        for (position, set) in routine.sets.iter().enumerate() {
            println!(
                "  Set {} (ID {}, {}): minimum kilograms {}, maximum RPE {}",
                position + 1,
                set.id.unwrap(),
                set.kind,
                quantity(&set.kilograms),
                quantity(&set.rpe)
            );
            if let Some(description) = &set.load_description {
                println!("    Load description: {description}");
            }
            if let Some(notes) = &set.notes {
                println!("    Set notes: {notes}");
            }
            for (position, portion) in set.portions.iter().enumerate() {
                let exercise = store.show(portion.exercise_id)?;
                let (label, target) = match exercise.measurement {
                    Measurement::Repetitions => (
                        "minimum successful repetitions (each side for unilateral activity)",
                        &portion.repetitions,
                    ),
                    Measurement::Duration => ("minimum seconds", &portion.seconds),
                    Measurement::Distance => ("minimum metres", &portion.metres),
                };
                println!(
                    "    Portion {} (ID {}), exercise {}: {} — {label}: {}",
                    position + 1,
                    portion.id.unwrap(),
                    exercise.id,
                    exercise.name,
                    quantity(target)
                );
                if let Some(notes) = &portion.notes {
                    println!("      Portion notes: {notes}");
                }
            }
        }
    }
    Ok(())
}
fn print_workout(
    store: &Store,
    workout: &workouts::Workout,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        println!("{}", serde_json::to_string_pretty(workout)?);
        return Ok(());
    }
    println!(
        "{}: {} workout ({})\n  Start: {}\n  End: {}",
        workout.id.unwrap(),
        workout.state,
        workout.date,
        workout.start.as_deref().unwrap_or("unknown"),
        workout.end.as_deref().unwrap_or("unknown")
    );
    if let Some(notes) = &workout.notes {
        println!("  Session notes: {notes}");
    }
    for (index, set) in workout.sets.iter().enumerate() {
        println!(
            "  Set {} (ID {}, {}): kilograms {}, RPE {}",
            index + 1,
            set.id.unwrap(),
            set.kind,
            set.kilograms
                .as_ref()
                .map_or("unknown".into(), ToString::to_string),
            set.rpe
                .as_ref()
                .map_or("unknown".into(), ToString::to_string)
        );
        if let Some(description) = &set.load_description {
            println!("    Load description: {description}");
        }
        if let Some(notes) = &set.notes {
            println!("    Set notes: {notes}");
        }
        if let (Some(white), Some(red)) = (set.white_flags, set.red_flags) {
            println!(
                "    Judging: {white} white, {red} red{}",
                if red > white {
                    " (failed judgment)"
                } else {
                    ""
                }
            );
        }
        for (index, portion) in set.portions.iter().enumerate() {
            let exercise = store.show(portion.exercise_id)?;
            let (label, value) = match exercise.measurement {
                Measurement::Repetitions => (
                    "attempted repetitions (minimum of sides for unilateral activity)",
                    &portion.repetitions,
                ),
                Measurement::Duration => ("seconds", &portion.seconds),
                Measurement::Distance => ("metres", &portion.metres),
            };
            println!(
                "    Portion {} (ID {}), exercise {}: {} — {label}: {}",
                index + 1,
                portion.id.unwrap(),
                exercise.id,
                exercise.name,
                value.as_ref().unwrap()
            );
            if let Some(notes) = &portion.notes {
                println!("      Portion notes: {notes}");
            }
        }
    }
    Ok(())
}
fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let path = match cli.db {
        Some(path) => path,
        None => {
            PathBuf::from(std::env::var_os("HOME").ok_or("HOME is unavailable; supply --db PATH")?)
                .join(".precision/precision.sqlite3")
        }
    };
    let mut store = Store::open(&path)?;
    match cli.command {
        Command::Workout { command } => match command {
            WorkoutCommand::Start { date, start } => {
                let workout = store.start_workout(date, start)?;
                print_workout(&store, &workout, false)?;
            }
            WorkoutCommand::List { drafts, json } => {
                let workouts = store.workouts(drafts)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&workouts)?);
                } else if workouts.is_empty() {
                    println!("No workouts.");
                } else {
                    for workout in workouts {
                        print_workout(&store, &workout, false)?;
                    }
                }
            }
            WorkoutCommand::Show { id, json } => print_workout(&store, &store.workout(id)?, json)?,
            WorkoutCommand::Update { id, file } => {
                let document = serde_json::from_slice(&std::fs::read(file)?)?;
                let workout = store.save_workout(id, document)?;
                print_workout(&store, &workout, false)?;
            }
            WorkoutCommand::Finish { id, end } => {
                let workout = store.finish_workout(id, end)?;
                print_workout(&store, &workout, false)?;
            }
            WorkoutCommand::Discard { id } => {
                store.discard_workout(id)?;
                println!("Discarded workout {id}.");
            }
        },
        Command::Routine { command } => match command {
            RoutineCommand::Create { file } => {
                let document = serde_json::from_slice(&std::fs::read(file)?)?;
                let routine = store.save_routine(None, document)?;
                print_routine(&store, &routine, false)?;
            }
            RoutineCommand::Update { id, file } => {
                let document = serde_json::from_slice(&std::fs::read(file)?)?;
                let routine = store.save_routine(Some(id), document)?;
                print_routine(&store, &routine, false)?;
            }
            RoutineCommand::Show { id, json } => print_routine(&store, &store.routine(id)?, json)?,
            RoutineCommand::List { json } => {
                let routines = store.routines()?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&routines)?);
                } else if routines.is_empty() {
                    println!("No routines. Create one with routine create --file PATH.");
                } else {
                    for routine in routines {
                        print_routine(&store, &routine, false)?;
                    }
                }
            }
            RoutineCommand::Delete { id } => {
                store.delete_routine(id)?;
                println!("Deleted routine {id}.");
            }
        },
        Command::Catalog { kind } => {
            let table = match kind {
                Catalog::Equipment => "equipment",
                Catalog::Muscles => "muscles",
            };
            for (id, name) in store.catalog(table)? {
                println!("{id}: {name}");
            }
        }
        Command::Exercise { command } => match command {
            ExerciseCommand::Create {
                name,
                measurement,
                load_convention,
                equipment,
                muscles,
            } => {
                let exercise = Exercise {
                    schema_version: 1,
                    id: 0,
                    name,
                    measurement,
                    load_convention,
                    equipment,
                    primary_muscle: muscles.primary_muscle,
                    secondary_muscles: muscles.secondary_muscle,
                };
                let exercise = store.create(exercise)?;
                print_exercise(&store, &exercise, false)?;
            }
            ExerciseCommand::List { json } => {
                let exercises = store.list()?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&exercises)?);
                } else if exercises.is_empty() {
                    println!("No exercises. Create one with exercise create --help.");
                } else {
                    for exercise in exercises {
                        print_exercise(&store, &exercise, false)?;
                    }
                }
            }
            ExerciseCommand::Show { id, json } => print_exercise(&store, &store.show(id)?, json)?,
            ExerciseCommand::Update {
                id,
                name,
                muscles,
                clear_primary_muscle,
                clear_secondary_muscles,
            } => {
                if name.is_none()
                    && muscles.primary_muscle.is_none()
                    && muscles.secondary_muscle.is_empty()
                    && !clear_primary_muscle
                    && !clear_secondary_muscles
                {
                    return Err("supply --name, muscle options, or --clear-primary-muscle/--clear-secondary-muscles".into());
                }
                let exercise = store.update(
                    id,
                    name,
                    muscles.primary_muscle,
                    if clear_secondary_muscles {
                        Some(vec![])
                    } else if muscles.secondary_muscle.is_empty() {
                        None
                    } else {
                        Some(muscles.secondary_muscle)
                    },
                    clear_primary_muscle,
                )?;
                print_exercise(&store, &exercise, false)?;
            }
        },
    }
    Ok(())
}
fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
