use serde::Deserialize;
use serde_json::Number;
use std::collections::HashSet;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// Read the original JSON token so serde_json's private Number object cannot
// masquerade as a user-supplied numeric value under arbitrary_precision.
pub(crate) fn optional_number<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Number>, D::Error> {
    let raw = Option::<Box<serde_json::value::RawValue>>::deserialize(deserializer)?;
    raw.map(|raw| {
        let token = raw.get();
        if !matches!(token.as_bytes().first(), Some(b'-' | b'0'..=b'9')) {
            return Err(serde::de::Error::custom(
                "invalid type: expected a JSON number or null",
            ));
        }
        serde_json::from_str(token).map_err(serde::de::Error::custom)
    })
    .transpose()
}
pub(crate) fn text(number: &Option<Number>) -> Option<String> {
    number.as_ref().map(ToString::to_string)
}
pub(crate) fn number(value: Option<String>) -> rusqlite::Result<Option<Number>> {
    value
        .map(|value| {
            value.parse().map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()
}
pub(crate) fn validate_id(
    id: Option<i64>,
    original: &HashSet<i64>,
    seen: &mut HashSet<i64>,
    kind: &str,
) -> Result<()> {
    if let Some(id) = id {
        if !original.contains(&id) {
            return Err(
                format!("unknown or foreign {kind} ID {id}; new entries must omit IDs").into(),
            );
        }
        if !seen.insert(id) {
            return Err(format!("duplicate {kind} ID {id}").into());
        }
    }
    Ok(())
}
// Parse decimal spelling directly: no binary floating point and no rounding.
// Quantities have at most twelve integer and six fractional significant places.
pub(crate) fn decimal_units(number: &Number) -> Result<i64> {
    let spelling = number.to_string();
    let (mantissa, exponent) = match spelling.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (
            mantissa,
            exponent
                .parse::<i32>()
                .map_err(|_| "decimal exponent exceeds precision bounds")?,
        ),
        None => (spelling.as_str(), 0),
    };
    let negative = mantissa.starts_with('-');
    let mantissa = mantissa.trim_start_matches('-');
    let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{integer}{fraction}");
    let significant = digits.trim_start_matches('0').trim_end_matches('0');
    if significant.is_empty() {
        return Ok(0);
    }
    let trailing = digits.len() - digits.trim_end_matches('0').len();
    let shift = i64::from(exponent) - fraction.len() as i64 + trailing as i64 + 6;
    if !(0..=18).contains(&shift) || significant.len() as i64 + shift > 18 {
        return Err("decimal exceeds precision: at most 12 integer places and 6 fractional places, without rounding".into());
    }
    let units: i64 = significant.parse::<i64>()? * 10_i64.pow(shift as u32);
    Ok(if negative { -units } else { units })
}
