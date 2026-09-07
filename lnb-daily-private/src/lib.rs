//! bot のプライベートな事情を日付から決定論的に生成する。
//!
//! 乱数の消費列が日付より細かい要素で変化しないことを型で保証するために、
//! 各設定は計画フェーズと観測フェーズの 2 段に分かれている。
//!
//! - `plan` は [`rng::RngSource`] を受け取るが、入力は [`datetime::LogicalDay`]
//!   もしくは [`datetime::LongTermCycle`] までの粒度に限られる。
//! - `observe` は [`datetime::LogicalMoment`] 由来の時刻情報を受け取るが、
//!   乱数生成器を受け取らない。
//!
//! さらに `plan` は [`rng::RngDomain`] ごとに独立した乱数列を導出するので、
//! モジュール間で乱数の消費が干渉しない。
//!
//! 全体の合成は [`plan::DailyPrivateConfiguration`] が行う。

pub mod datetime;
pub mod day_routine;
pub mod masturbation;
pub mod menstruation;
pub mod plan;
pub mod rng;
pub mod schedule;
pub mod temperature;
pub mod underwear;

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum DailyPrivateError {
    #[error("long term days cannot be divided")]
    LongTermMismatch,
}
