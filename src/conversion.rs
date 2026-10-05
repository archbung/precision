use crate::{
    exercises::Store,
    organization::{Rest, Superset},
    routines::{PrescribedPortion, PrescribedSet, Routine},
};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const GUIDANCE: &str = "Recorded repetitions are attempts, not confirmed successes; review them as future minimum successful repetitions (each side for unilateral activity). Primary quantities and signed kilograms are minima; RPE is a maximum and rest is a minimum. Null means no numeric instruction; zero is explicit. Notes do not establish success. RPE and rest targets are unspecified by default.";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub schema_version: u32,
    pub from_workout: i64,
    pub source_routine_id: Option<i64>,
    pub source_revision: Option<i64>,
    pub review_guidance: String,
    pub routine: Routine,
}

impl Store {
    pub fn propose_routine(&self, id: i64) -> Result<Proposal> {
        let tx = self.connection.unchecked_transaction()?;
        let workout = self.workout(id)?;
        if workout.state != "finished" {
            return Err("conversion requires a finished workout".into());
        }
        let ids: HashMap<_, _> = workout
            .sets
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.unwrap(), -(i as i64 + 1)))
            .collect();
        let mut portion_id = 0;
        let sets = workout
            .sets
            .iter()
            .map(|set| PrescribedSet {
                id: Some(ids[&set.id.unwrap()]),
                kind: set.kind.clone(),
                kilograms: set.kilograms.clone(),
                rpe: None,
                load_description: set.load_description.clone(),
                notes: set.notes.clone(),
                portions: set
                    .portions
                    .iter()
                    .map(|portion| {
                        portion_id -= 1;
                        PrescribedPortion {
                            id: Some(portion_id),
                            exercise_id: portion.exercise_id,
                            repetitions: portion.repetitions.clone(),
                            seconds: portion.seconds.clone(),
                            metres: portion.metres.clone(),
                            notes: portion.notes.clone(),
                        }
                    })
                    .collect(),
            })
            .collect();
        let revision = workout
            .source_routine_id
            .map(|id| {
                tx.query_row("SELECT revision FROM routines WHERE id=?1", [id], |r| {
                    r.get::<_, i64>(0)
                })
            })
            .transpose()?;
        tx.commit()?;
        Ok(Proposal {
            schema_version: 1,
            from_workout: id,
            source_routine_id: workout.original_source_id,
            source_revision: revision,
            review_guidance: GUIDANCE.into(),
            routine: Routine {
                schema_version: 1,
                id: None,
                name: format!("Workout {} ({})", id, workout.date),
                notes: workout.notes.clone(),
                sets,
                rest: Some(
                    (0..workout.sets.len().saturating_sub(1))
                        .map(|_| Rest(None))
                        .collect(),
                ),
                supersets: workout
                    .supersets
                    .iter()
                    .enumerate()
                    .map(|(i, g)| Superset {
                        id: Some(-(i as i64 + 1)),
                        set_ids: g.set_ids.iter().map(|id| ids[id]).collect(),
                    })
                    .collect(),
            },
        })
    }
    pub fn save_reviewed_routine(
        &mut self,
        id: i64,
        mut proposal: Proposal,
        replace: bool,
    ) -> Result<Routine> {
        let original_proposal = self.propose_routine(id)?;
        if proposal.schema_version != 1
            || proposal.from_workout != id
            || proposal.source_routine_id != original_proposal.source_routine_id
            || proposal.source_revision.is_some_and(|r| r < 1)
            || (proposal.source_routine_id.is_none() && proposal.source_revision.is_some())
            || (original_proposal.source_revision.is_some() && proposal.source_revision.is_none())
        {
            return Err(
                "mismatched proposal provenance; generate a fresh proposal and review".into(),
            );
        }
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let destination = if replace {
            let source = proposal
                .source_routine_id
                .ok_or("workout has no source routine")?;
            let current: Option<i64> = tx
                .query_row("SELECT revision FROM routines WHERE id=?1", [source], |r| {
                    r.get(0)
                })
                .optional()?;
            if current.is_none() || current != proposal.source_revision {
                return Err("source routine was deleted or revision is stale; generate a fresh proposal and review".into());
            }
            // Confirm the live source link within the same transaction as replacement.
            let live: Option<i64> = tx.query_row(
                "SELECT source_routine_id FROM workouts WHERE id=?1",
                [id],
                |r| r.get(0),
            )?;
            if live != Some(source) {
                return Err(
                    "mismatched source routine; generate a fresh proposal and review".into(),
                );
            }
            Some(source)
        } else {
            None
        };
        crate::routines::validate(&tx, &mut proposal.routine, Some(&original_proposal.routine))?;
        let local_ids: Vec<_> = proposal.routine.sets.iter().map(|s| s.id).collect();
        for set in &mut proposal.routine.sets {
            set.id = None;
            for portion in &mut set.portions {
                portion.id = None;
            }
        }
        for group in &mut proposal.routine.supersets {
            group.id = None;
        }
        crate::routines::persist(&tx, destination, &mut proposal.routine, Some(&local_ids))?;
        tx.commit()?;
        Ok(proposal.routine)
    }
}
