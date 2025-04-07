mod daily_private;
mod exchange_rate;
mod get_illust_url;
mod image_generator;
mod local_info;
mod self_info;

pub use exchange_rate::ExchangeRate;
pub use get_illust_url::GetIllustUrl;
pub use image_generator::ImageGenerator;
pub use local_info::LocalInfo;
pub use self_info::SelfInfo;

use lnb_core::error::FunctionError;

pub trait ConfigurableFunction
where
    Self: Sized,
{
    const NAME: &'static str;
    type Configuration;
    async fn create(config: &Self::Configuration) -> Result<Self, FunctionError>;
}
