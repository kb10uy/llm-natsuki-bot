mod exchange_rate;
mod get_illust_url;
mod image_generator;
mod local_info;
mod self_info;

pub use self::exchange_rate::ExchangeRate;
pub use self::get_illust_url::GetIllustUrl;
pub use self::image_generator::ImageGenerator;
pub use self::local_info::LocalInfo;
pub use self::self_info::SelfInfo;

use lnb_core::error::FunctionError;

pub trait ConfigurableFunction
where
    Self: Sized,
{
    const NAME: &'static str;
    type Configuration;
    async fn create(config: &Self::Configuration) -> Result<Self, FunctionError>;
}
