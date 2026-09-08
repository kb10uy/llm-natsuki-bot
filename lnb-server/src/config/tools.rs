use std::collections::HashMap;

use lnb_core::error::FunctionError;
use lnb_daily_private::{
    masturbation::MasturbationConfiguration, menstruation::MenstruationConfiguration, schedule::ScheduleConfiguration,
    temperature::TemperatureConfiguration, underwear::UnderwearConfiguration,
};
use serde::Deserialize;
use thiserror::Error as ThisError;

/// [tool]
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigTools {
    pub self_info: ConfigToolsSelfInfo,
    pub local_info: ConfigToolsLocalInfo,
    pub shiyu_provider: ConfigToolsShiyuProvider,
    pub image_generator: Option<ConfigToolsImageGenerator>,
    pub math_renderer: Option<ConfigToolsMathRenderer>,
    pub get_illust_url: Option<ConfigToolsGetIllustUrl>,
    pub exchange_rate: Option<ConfigToolsExchangeRate>,
    pub daily_private: Option<ConfigToolsDailyPrivate>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigToolsPrompt {
    pub description: String,

    #[serde(default = "Default::default")]
    pub parameters: HashMap<String, String>,
}

impl ConfigToolsPrompt {
    pub fn parameter(&self, name: &str) -> Result<&str, FunctionError> {
        self.parameters
            .get(name)
            .map(|d| d.as_str())
            .ok_or_else(|| FunctionError::by_serialization(UndefinedPromptParameter(name.to_string())))
    }
}

#[derive(Debug, ThisError)]
#[error("prompt parameter {0} is not defined")]
pub struct UndefinedPromptParameter(pub String);

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigToolsSelfInfo {
    pub prompt: ConfigToolsPrompt,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigToolsLocalInfo {
    pub prompt: ConfigToolsPrompt,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigToolsShiyuProvider {
    pub prompt: ConfigToolsPrompt,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigToolsImageGenerator {
    pub endpoint: String,
    pub token: String,
    pub model: String,
    pub prompt: ConfigToolsPrompt,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigToolsMathRenderer {
    pub endpoint: String,
    pub scale: f64,
    pub prompt: ConfigToolsPrompt,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigToolsGetIllustUrl {
    pub database_filepath: String,
    pub prompt: ConfigToolsPrompt,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigToolsExchangeRate {
    pub endpoint: String,
    pub token: String,
    pub prompt: ConfigToolsPrompt,
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
    pub prompt: ConfigToolsPrompt,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigToolsDailyPrivateDayRoutine {
    pub long_term_days: u64,
    pub morning_start: String,
    pub morning_preparation_minutes: usize,
    pub daytime_minutes: usize,
    pub bathtime_minutes: usize,
}
