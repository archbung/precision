use crate::aggregate_values::{decimal_units, number, optional_number, text, validate_id};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::Number;
use std::collections::HashSet;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct Rest(#[serde(deserialize_with = "optional_number")] pub Option<Number>);

pub fn rest_array<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Vec<Rest>>, D::Error> {
    Vec::<Rest>::deserialize(deserializer).map(Some)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Superset {
    pub id: Option<i64>,
    pub set_ids: Vec<i64>,
}

pub fn validate(
    rest: &mut Option<Vec<Rest>>,
    groups: &[Superset],
    sets: &[Option<i64>],
    old_sets: &[Option<i64>],
    old_groups: &[Superset],
) -> Result<()> {
    if rest.is_none() {
        let retained: HashSet<_> = sets.iter().flatten().copied().collect();
        let old_order: Vec<_> = old_sets
            .iter()
            .flatten()
            .filter(|id| retained.contains(id))
            .copied()
            .collect();
        let new_order: Vec<_> = sets
            .iter()
            .flatten()
            .filter(|id| old_order.contains(id))
            .copied()
            .collect();
        if old_order != new_order {
            return Err("reordering sets requires a complete replacement rest array".into());
        }
        *rest = Some(
            (0..sets.len().saturating_sub(1))
                .map(|_| Rest(None))
                .collect(),
        );
    }
    let rest = rest.as_ref().unwrap();
    if rest.len() != sets.len().saturating_sub(1) {
        return Err("rest requires exactly N-1 transition values (empty for zero/one set)".into());
    }
    for value in rest {
        if value
            .0
            .as_ref()
            .map(decimal_units)
            .transpose()?
            .is_some_and(|n| n < 0)
        {
            return Err("rest seconds must be nonnegative".into());
        }
    }
    let old_ids = old_groups.iter().filter_map(|g| g.id).collect();
    let local: HashSet<_> = sets.iter().flatten().copied().collect();
    let mut ids = HashSet::new();
    let mut members = HashSet::new();
    for group in groups {
        validate_id(group.id, &old_ids, &mut ids, "superset")?;
        if group.set_ids.len() < 2 {
            return Err("superset requires at least two distinct sets".into());
        }
        for id in &group.set_ids {
            if !local.contains(id) {
                return Err(format!("unknown or foreign superset set ID {id}").into());
            }
            if !members.insert(*id) {
                return Err("duplicate or overlapping superset membership".into());
            }
        }
    }
    Ok(())
}

// The prefix is supplied only by the three aggregate persistence paths.
pub fn write(
    connection: &Connection,
    prefix: &str,
    owner: i64,
    rest: &[Rest],
    groups: &mut [Superset],
) -> Result<()> {
    connection.execute(
        &format!("DELETE FROM {prefix}_rest WHERE owner_id=?1"),
        [owner],
    )?;
    connection.execute(
        &format!("DELETE FROM {prefix}_supersets WHERE owner_id=?1"),
        [owner],
    )?;
    for (position, value) in rest.iter().enumerate() {
        connection.execute(
            &format!("INSERT INTO {prefix}_rest(owner_id,position,seconds) VALUES (?1,?2,?3)"),
            params![owner, position as i64, text(&value.0)],
        )?;
    }
    for (position, group) in groups.iter_mut().enumerate() {
        connection.execute(
            &format!("INSERT INTO {prefix}_supersets(id,owner_id,position) VALUES (?1,?2,?3)"),
            params![group.id, owner, position as i64],
        )?;
        group.id = Some(group.id.unwrap_or_else(|| connection.last_insert_rowid()));
        for (position, id) in group.set_ids.iter().enumerate() {
            connection.execute(&format!("INSERT INTO {prefix}_members(group_id,owner_id,set_id,position) VALUES (?1,?2,?3,?4)"), params![group.id,owner,id,position as i64])?;
        }
    }
    Ok(())
}
pub fn read(
    connection: &Connection,
    prefix: &str,
    owner: i64,
) -> Result<(Option<Vec<Rest>>, Vec<Superset>)> {
    let mut statement = connection.prepare(&format!(
        "SELECT seconds FROM {prefix}_rest WHERE owner_id=?1 ORDER BY position"
    ))?;
    let rest = statement
        .query_map([owner], |row| Ok(Rest(number(row.get(0)?)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut statement = connection.prepare(&format!(
        "SELECT id FROM {prefix}_supersets WHERE owner_id=?1 ORDER BY position"
    ))?;
    let ids = statement
        .query_map([owner], |row| row.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut groups = vec![];
    for id in ids {
        let mut statement = connection.prepare(&format!(
            "SELECT set_id FROM {prefix}_members WHERE group_id=?1 ORDER BY position"
        ))?;
        let set_ids = statement
            .query_map([id], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        groups.push(Superset {
            id: Some(id),
            set_ids,
        });
    }
    Ok((Some(rest), groups))
}

pub fn print(rest: &Option<Vec<Rest>>, groups: &[Superset], prescribed: bool) {
    for (position, value) in rest.as_deref().unwrap_or(&[]).iter().enumerate() {
        println!(
            "  {} rest after set {}: {} seconds",
            if prescribed { "Minimum" } else { "Observed" },
            position + 1,
            value.0.as_ref().map_or(
                if prescribed { "unspecified" } else { "unknown" }.into(),
                ToString::to_string
            )
        );
    }
    for group in groups {
        println!(
            "  {} superset ID {}: set IDs {:?}",
            if prescribed { "Prescribed" } else { "Actual" },
            group.id.unwrap(),
            group.set_ids
        );
    }
}
