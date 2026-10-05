ALTER TABLE workouts ADD COLUMN source_routine_id INTEGER REFERENCES routines(id) ON DELETE SET NULL;
ALTER TABLE workouts ADD COLUMN original_source_id INTEGER;
ALTER TABLE workouts ADD COLUMN original_source_name TEXT;
ALTER TABLE workouts ADD COLUMN intention_notes TEXT;
CREATE TABLE intention_sets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workout_id INTEGER NOT NULL REFERENCES workouts(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK(position >= 0),
    type TEXT NOT NULL CHECK(type IN ('warmup','main')),
    kilograms TEXT,
    rpe_half INTEGER CHECK(rpe_half BETWEEN 2 AND 20),
    load_description TEXT,
    notes TEXT,
    UNIQUE(workout_id, position)
);
CREATE TABLE intention_portions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    set_id INTEGER NOT NULL REFERENCES intention_sets(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK(position >= 0),
    exercise_id INTEGER NOT NULL REFERENCES exercises(id),
    repetitions TEXT,
    seconds TEXT,
    metres TEXT,
    notes TEXT,
    UNIQUE(set_id, position)
);
