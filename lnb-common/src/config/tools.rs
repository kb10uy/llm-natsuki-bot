use lnb_daily_private::{
    masturbation::MasturbationConfiguration, menstruation::MenstruationConfiguration, schedule::ScheduleConfiguration,
    temperature::TemperatureConfiguration, underwear::UnderwearConfiguration,
};
use serde::Deserialize;

/// [tool]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ConfigTools {
    pub image_generator: Option<ConfigToolsImageGenerator>,
    pub get_illust_url: Option<ConfigToolsGetIllustUrl>,
    pub exchange_rate: Option<ConfigToolsExchangeRate>,
    pub daily_private: Option<ConfigToolsDailyPrivate>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigToolsImageGenerator {
    pub endpoint: String,
    pub token: String,
    pub model: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigToolsGetIllustUrl {
    pub database_filepath: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigToolsExchangeRate {
    pub endpoint: String,
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigToolsDailyPrivate {
    pub daily_rng_salt: String,
    pub day_routine: ConfigToolsDailyPrivateDayRoutine,
    pub schedule: ScheduleConfiguration,
    pub underwear: UnderwearConfiguration,
    pub masturbation: MasturbationConfiguration,
    pub menstruation: MenstruationConfiguration,
    pub temperature: TemperatureConfiguration,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigToolsDailyPrivateDayRoutine {
    pub long_term_days: u64,
    pub morning_start: String,
    pub morning_preparation_minutes: usize,
    pub night_start: String,
    pub bathtime_minutes: usize,
}
