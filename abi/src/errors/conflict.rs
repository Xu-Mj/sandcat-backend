use chrono::{DateTime, Utc};
use regex::Regex;
use std::{collections::HashMap, str::FromStr};
/// s.parse()
/// 1. invoke ConflictReservationInfo::from_str(s)
/// 2. invoke ConfilictReservation::from_str(s)
/// 3. invoke ParsedInfo::from_str(s)?.try_into() in ConflictReservation::from_str(s); do parse the string
/// 4. invoke ConfilictReservation::try_from(parsed_info)
/// 5. invoke ReservationWindow::try_from(hash_map)
/// 6. parse hash map to ReservationWindow
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictReservationInfo {
    Parsed(ConflictReservation),
    UnParsed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictReservation {
    pub new: ReservationWindow,
    pub old: ReservationWindow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationWindow {
    pub resource_id: String,
    // for user readable
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl FromStr for ConflictReservation {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // parse
        ParsedInfo::from_str(s)?.try_into()
    }
}

impl FromStr for ConflictReservationInfo {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(conflict) = s.parse() {
            Ok(ConflictReservationInfo::Parsed(conflict))
        } else {
            Ok(ConflictReservationInfo::UnParsed(s.to_string()))
        }
    }
}

#[derive(Debug)]
struct ParsedInfo {
    pub new: HashMap<String, String>,
    pub old: HashMap<String, String>,
}

// parse ParsedInfo to ConflictReservation
impl TryFrom<ParsedInfo> for ConflictReservation {
    type Error = ();

    fn try_from(value: ParsedInfo) -> Result<Self, Self::Error> {
        Ok(Self {
            new: value.new.try_into()?,
            old: value.old.try_into()?,
        })
    }
}

impl TryFrom<HashMap<String, String>> for ReservationWindow {
    type Error = ();

    fn try_from(value: HashMap<String, String>) -> Result<Self, Self::Error> {
        // parse time
        let time = value.get("timespan").ok_or(())?.replace('"', "");
        let mut split = time.splitn(2, ',');
        let start = parse_str_to_time(split.next().ok_or(())?)?;
        let end = parse_str_to_time(split.next().ok_or(())?)?;

        Ok(Self {
            resource_id: value.get("resource_id").ok_or(())?.to_string(),
            start,
            end,
        })
    }
}
impl FromStr for ParsedInfo {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // "键(resource_id, timespan)=(ocean-view-room-713, [\"2022-12-25 22:00:00+00\",\"2022-12-30 19:00:00+00\"))与已存在的键(resource_id, timespan)=(ocean-view-room-713, [\"2022-12-25 22:00:00+00\",\"2022-12-28 19:00:00+00\"))冲突"
        // parse:
        // k1:resource_id, v1: ocean-view-room-713,
        // k2:timespan, v2:["2022-12-25 22:00:00+00","2022-12-30 19:00:00+00"]
        let re = Regex::new(r#"\((?P<k1>[a-zA-Z0-9_-]+)\s*,\s*(?P<k2>[a-zA-Z0-9_-]+)\)=\((?P<v1>[a-zA-Z0-9_-]+)\s*,\s*\[(?P<v2>[^\)\]]+)"#).unwrap();
        // use vec to store the result
        let mut maps = vec![];
        for cap in re.captures_iter(s) {
            let mut map = HashMap::new();
            map.insert(cap["k1"].to_string(), cap["v1"].to_string());
            map.insert(cap["k2"].to_string(), cap["v2"].to_string());

            maps.push(Some(map));
        }
        if maps.len() != 2 {
            return Err(());
        }
        Ok(ParsedInfo {
            new: maps[0].take().unwrap(),
            old: maps[1].take().unwrap(),
        })
    }
}

fn parse_str_to_time(s: &str) -> Result<DateTime<Utc>, ()> {
    Ok(
        chrono::DateTime::parse_from_str(s.trim(), "%Y-%m-%d %H:%M:%S %#z")
            .map_err(|_| ())?
            .with_timezone(&Utc),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_STR: &str = "键(resource_id, timespan)=(ocean-view-room-713, [\"2022-12-25 22:00:00+00\",\"2022-12-30 19:00:00+00\"))与已存在的键(resource_id, timespan)=(ocean-view-room-713, [\"2022-12-25 22:00:00+00\",\"2022-12-28 19:00:00+00\"))冲突";

    #[test]
    fn parse_str_to_time_should_work() {
        parse_str_to_time("2022-12-25 22:00:00+00").unwrap();
    }
    #[test]
    fn str_to_parsed_info_should_work() {
        let conflict: ParsedInfo = TEST_STR.parse().unwrap();
        assert_eq!(conflict.new["resource_id"], "ocean-view-room-713");
        assert_eq!(conflict.old["resource_id"], "ocean-view-room-713");
        assert_eq!(
            conflict.new["timespan"],
            "\"2022-12-25 22:00:00+00\",\"2022-12-30 19:00:00+00\""
        );
        assert_eq!(
            conflict.old["timespan"],
            "\"2022-12-25 22:00:00+00\",\"2022-12-28 19:00:00+00\""
        );
    }

    #[test]
    fn hash_map_to_reservation_window_should_work() {
        let mut map = HashMap::new();
        map.insert("resource_id".to_string(), "ocean-view-room-713".to_string());
        map.insert(
            "timespan".to_string(),
            " \"2022-12-25 22:00:00+00\",\"2022-12-30 19:00:00+00\"".to_string(),
        );
        let window: ReservationWindow = map.try_into().unwrap();
        assert_eq!(window.resource_id, "ocean-view-room-713");
        assert_eq!(window.start.to_rfc3339(), "2022-12-25T22:00:00+00:00");
        assert_eq!(window.end.to_rfc3339(), "2022-12-30T19:00:00+00:00");
    }
    #[test]
    fn parse_to_conflict_reservation_info_should_work() {
        let conflict: ConflictReservationInfo = TEST_STR.parse().unwrap();
        if let ConflictReservationInfo::Parsed(conflict) = conflict {
            assert_eq!(conflict.new.resource_id, "ocean-view-room-713");
            assert_eq!(conflict.old.resource_id, "ocean-view-room-713");
            assert_eq!(conflict.new.start.to_rfc3339(), "2022-12-25T22:00:00+00:00");
            assert_eq!(conflict.old.start.to_rfc3339(), "2022-12-25T22:00:00+00:00");
            assert_eq!(conflict.new.end.to_rfc3339(), "2022-12-30T19:00:00+00:00");
            assert_eq!(conflict.old.end.to_rfc3339(), "2022-12-28T19:00:00+00:00");
        }
    }
}
