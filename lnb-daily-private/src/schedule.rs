use std::{
    fmt::{Formatter, Result as FmtResult},
    ops::RangeInclusive,
};

use serde::{
    Deserialize, Serialize,
    de::{Error as _, SeqAccess, Visitor},
    ser::SerializeTuple,
};
use time::Date;

const WEEK_NUMBER_RANGE: RangeInclusive<u8> = 1..=53;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WeekRange(u8, u8);

impl WeekRange {
    pub fn contains(&self, date: Date) -> bool {
        (self.0..=self.1).contains(&date.iso_week())
    }
}

impl<'de> Deserialize<'de> for WeekRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(WeekRangeVisitor)
    }
}

impl Serialize for WeekRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple_serializer = serializer.serialize_tuple(2)?;
        tuple_serializer.serialize_element(&self.0)?;
        tuple_serializer.serialize_element(&self.1)?;
        tuple_serializer.end()
    }
}

struct WeekRangeVisitor;

impl<'de> Visitor<'de> for WeekRangeVisitor {
    type Value = WeekRange;

    fn expecting(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_str("2 element tuple")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<WeekRange, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let start = seq.next_element::<u8>()?;
        let end = seq.next_element::<u8>()?;
        let should_none = seq.next_element::<()>()?;

        match (start, end, should_none) {
            (Some(start_n), Some(end_n), None) => {
                if !WEEK_NUMBER_RANGE.contains(&start_n) {
                    Err(A::Error::custom(format!("start week number out of range: {start_n}")))
                } else if !WEEK_NUMBER_RANGE.contains(&end_n) {
                    Err(A::Error::custom(format!("end week number out of range: {end_n}")))
                } else {
                    Ok(WeekRange(start_n, end_n))
                }
            }
            (_, _, _) => Err(A::Error::custom("should be 2 integer elements")),
        }
    }
}
