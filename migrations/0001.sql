CREATE TABLE equipment (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE);
CREATE TABLE muscles (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE);
CREATE TABLE exercises (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL CHECK(length(name)>0),
    name_key TEXT NOT NULL UNIQUE,
    measurement TEXT NOT NULL CHECK(measurement IN ('repetitions','duration','distance')),
    load_convention TEXT NOT NULL CHECK(load_convention IN ('external','added-bodyweight')),
    primary_muscle INTEGER REFERENCES muscles(id)
);
CREATE TABLE exercise_equipment (
    exercise_id INTEGER NOT NULL REFERENCES exercises(id),
    equipment_id INTEGER NOT NULL REFERENCES equipment(id),
    PRIMARY KEY(exercise_id,equipment_id)
);
CREATE TABLE exercise_secondary_muscles (
    exercise_id INTEGER NOT NULL REFERENCES exercises(id),
    muscle_id INTEGER NOT NULL REFERENCES muscles(id),
    PRIMARY KEY(exercise_id,muscle_id)
);
