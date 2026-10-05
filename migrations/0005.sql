CREATE UNIQUE INDEX prescribed_set_owner ON prescribed_sets(id,routine_id);
CREATE TABLE prescribed_rest (
 owner_id INTEGER NOT NULL REFERENCES routines(id) ON DELETE CASCADE,
 position INTEGER NOT NULL CHECK(position >= 0), seconds TEXT,
 PRIMARY KEY(owner_id,position)
);
INSERT INTO prescribed_rest(owner_id,position,seconds)
 SELECT routine_id,position,NULL FROM prescribed_sets WHERE position < (SELECT count(*)-1 FROM prescribed_sets AS s WHERE s.routine_id=prescribed_sets.routine_id);
CREATE TABLE prescribed_supersets (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 owner_id INTEGER NOT NULL REFERENCES routines(id) ON DELETE CASCADE,
 position INTEGER NOT NULL CHECK(position >= 0),
 UNIQUE(owner_id,position), UNIQUE(id,owner_id)
);
CREATE TABLE prescribed_members (
 group_id INTEGER NOT NULL, owner_id INTEGER NOT NULL, set_id INTEGER NOT NULL,
 position INTEGER NOT NULL CHECK(position >= 0),
 FOREIGN KEY(group_id,owner_id) REFERENCES prescribed_supersets(id,owner_id) ON DELETE CASCADE,
 FOREIGN KEY(set_id,owner_id) REFERENCES prescribed_sets(id,routine_id) ON DELETE CASCADE,
 PRIMARY KEY(owner_id,set_id), UNIQUE(group_id,position)
);
CREATE UNIQUE INDEX intention_set_owner ON intention_sets(id,workout_id);
CREATE TABLE intention_rest (
 owner_id INTEGER NOT NULL REFERENCES workouts(id) ON DELETE CASCADE,
 position INTEGER NOT NULL CHECK(position >= 0), seconds TEXT,
 PRIMARY KEY(owner_id,position)
);
INSERT INTO intention_rest(owner_id,position,seconds)
 SELECT workout_id,position,NULL FROM intention_sets WHERE position < (SELECT count(*)-1 FROM intention_sets AS s WHERE s.workout_id=intention_sets.workout_id);
CREATE TABLE intention_supersets (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 owner_id INTEGER NOT NULL REFERENCES workouts(id) ON DELETE CASCADE,
 position INTEGER NOT NULL CHECK(position >= 0),
 UNIQUE(owner_id,position), UNIQUE(id,owner_id)
);
CREATE TABLE intention_members (
 group_id INTEGER NOT NULL, owner_id INTEGER NOT NULL, set_id INTEGER NOT NULL,
 position INTEGER NOT NULL CHECK(position >= 0),
 FOREIGN KEY(group_id,owner_id) REFERENCES intention_supersets(id,owner_id) ON DELETE CASCADE,
 FOREIGN KEY(set_id,owner_id) REFERENCES intention_sets(id,workout_id) ON DELETE CASCADE,
 PRIMARY KEY(owner_id,set_id), UNIQUE(group_id,position)
);
CREATE UNIQUE INDEX performed_set_owner ON performed_sets(id,workout_id);
CREATE TABLE performed_rest (
 owner_id INTEGER NOT NULL REFERENCES workouts(id) ON DELETE CASCADE,
 position INTEGER NOT NULL CHECK(position >= 0), seconds TEXT,
 PRIMARY KEY(owner_id,position)
);
INSERT INTO performed_rest(owner_id,position,seconds)
 SELECT workout_id,position,NULL FROM performed_sets WHERE position < (SELECT count(*)-1 FROM performed_sets AS s WHERE s.workout_id=performed_sets.workout_id);
CREATE TABLE performed_supersets (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 owner_id INTEGER NOT NULL REFERENCES workouts(id) ON DELETE CASCADE,
 position INTEGER NOT NULL CHECK(position >= 0),
 UNIQUE(owner_id,position), UNIQUE(id,owner_id)
);
CREATE TABLE performed_members (
 group_id INTEGER NOT NULL, owner_id INTEGER NOT NULL, set_id INTEGER NOT NULL,
 position INTEGER NOT NULL CHECK(position >= 0),
 FOREIGN KEY(group_id,owner_id) REFERENCES performed_supersets(id,owner_id) ON DELETE CASCADE,
 FOREIGN KEY(set_id,owner_id) REFERENCES performed_sets(id,workout_id) ON DELETE CASCADE,
 PRIMARY KEY(owner_id,set_id), UNIQUE(group_id,position)
);
