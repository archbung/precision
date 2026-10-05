mod exercises;
use clap::{Args, Parser, Subcommand, ValueEnum};
use exercises::{Exercise, Store};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about = "Define and inspect exercise activities",
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
    Exercise {
        #[command(subcommand)]
        command: ExerciseCommand,
    },
    Catalog {
        #[arg(value_enum)]
        kind: Catalog,
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
#[derive(Clone, Copy, ValueEnum)]
enum Measurement {
    Repetitions,
    Duration,
    Distance,
}
#[derive(Clone, Copy, ValueEnum)]
enum LoadConvention {
    External,
    AddedBodyweight,
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
                    measurement: measurement.to_possible_value().unwrap().get_name().into(),
                    load_convention: load_convention
                        .to_possible_value()
                        .unwrap()
                        .get_name()
                        .into(),
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
