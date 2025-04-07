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
pub use local_info::LocalInfo;
pub use self_info::SelfInfo;

use lnb_core::{error::FunctionError, interface::function::simple::SimpleFunction};
use serde::de::DeserializeOwned;

pub trait ConfigurableSimpleFunction: SimpleFunction
where
    Self: Sized,
{
    /// Name used for logging.
    const NAME: &'static str;

    /// Configuration type.
    type Configuration: DeserializeOwned;

    /// Configures new instance.
    async fn configure(config: Self::Configuration) -> Result<Self, FunctionError>;
}
