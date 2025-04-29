mod daily_private;
mod exchange_rate;
mod get_illust_url;
mod image_generator;
mod local_info;
mod self_info;

pub use daily_private::DailyPrivate;
pub use exchange_rate::ExchangeRate;
pub use get_illust_url::GetIllustUrl;
pub use image_generator::ImageGenerator;
use lnb_rate_limiter::RateLimiter;
pub use local_info::LocalInfo;
pub use self_info::SelfInfo;

use std::fmt::Debug;

use lnb_core::{error::FunctionError, interface::function::Function};
use serde::de::DeserializeOwned;

pub trait ConfigurableFunction: Function
where
    Self: Sized,
{
    /// Name used for logging.
    const NAME: &'static str;

    /// Configuration type.
    type Configuration: Debug + DeserializeOwned;

    /// Configures new instance.
    async fn configure(config: &Self::Configuration, rate_limits: Option<RateLimiter>) -> Result<Self, FunctionError>;
}
