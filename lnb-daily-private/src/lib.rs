pub mod day_routine;
pub mod logical_date;
pub mod masturbation;
pub mod menstruation;
pub mod schedule;
pub mod temperature;
pub mod underwear;

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum DailyPrivateError {
    #[error("long term days cannot be divided")]
    LongTermMismatch,
}
