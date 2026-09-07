use crate::datetime::LogicalMoment;

use serde::{Deserialize, Serialize};
use time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayRoutine {
    /// 昼パートの長さ。
    daytime_duration: Duration,

    /// 朝着替えの時間。
    morning_preparation: Duration,

    /// 入浴時間。
    bathtime_duration: Duration,
}

impl DayRoutine {
    pub fn new(daytime_duration: Duration, morning_preparation: Duration, bathtime_duration: Duration) -> DayRoutine {
        DayRoutine {
            daytime_duration,
            morning_preparation,
            bathtime_duration,
        }
    }

    /// 時刻から行動状態を求める。乱数を使わないので観測フェーズで呼べる。
    pub fn observe(&self, moment: &LogicalMoment) -> DayStep {
        if moment.day_elapsed < self.daytime_duration {
            if moment.day_elapsed < self.morning_preparation {
                DayStep::Morning
            } else {
                DayStep::Daytime
            }
        } else {
            let night_elapsed = moment.day_elapsed - self.daytime_duration;
            if night_elapsed < self.bathtime_duration {
                DayStep::Bathtime
            } else {
                DayStep::Night
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DayStep {
    /// 起きた後着替えるまで(昼パート)
    Morning,

    /// 日中(昼パート)
    Daytime,

    /// 風呂(夜パート)
    Bathtime,

    /// 寝支度完了(夜パート)
    Night,

    /// 寝てる(夜パート)
    Asleep,

    /// 中途覚醒(夜パート)
    MidAwake,
}
