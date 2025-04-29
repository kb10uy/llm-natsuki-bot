use crate::function::ConfigurableSimpleFunction;

use std::collections::HashMap;

use futures::{FutureExt, TryFutureExt, future::BoxFuture};
use lnb_common::{config::tools::ConfigToolsExchangeRate, rate_limits::RateLimitsCategory};
use lnb_core::{
    APP_USER_AGENT, RFC3339_NUMOFFSET,
    error::FunctionError,
    interface::function::{FunctionDescriptor, FunctionResponse, simple::SimpleFunction},
    model::schema::DescribedSchema,
};
use reqwest::{Client, ClientBuilder};
use serde::Deserialize;
use serde_json::{Value, json};
use time::{OffsetDateTime, format_description::BorrowedFormatItem, macros::format_description};

// "Sun, 30 Mar 2025 00:00:01 +0000" とかが返ってくるので
const API_RESPONSE_DATETIME: &[BorrowedFormatItem<'static>] = format_description!(
    "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] [offset_hour sign:mandatory][offset_minute]"
);

#[derive(Debug)]
pub struct ExchangeRate {
    client: Client,
    token_endpoint: String,
}

impl ConfigurableSimpleFunction for ExchangeRate {
    const NAME: &'static str = stringify!(ExchangeRate);

    type Configuration = ConfigToolsExchangeRate;

    async fn configure(
        config: &ConfigToolsExchangeRate,
        _: Option<&RateLimitsCategory>,
    ) -> Result<ExchangeRate, FunctionError> {
        let client = ClientBuilder::new()
            .user_agent(APP_USER_AGENT)
            .build()
            .map_err(FunctionError::by_external)?;
        let token_endpoint = format!("{}/v6/{}", config.endpoint, config.token);
        Ok(ExchangeRate { client, token_endpoint })
    }
}

impl SimpleFunction for ExchangeRate {
    fn get_descriptor(&self) -> FunctionDescriptor {
        // TODO: 多分 [(String, String)] を受け取ってこっちで group_by した方が確実
        FunctionDescriptor {
            name: "exchange_rate".to_string(),
            description: "為替相場を取得します。同じ計算元の通貨から複数の計算先を一度に取得できます。".to_string(),
            parameters: DescribedSchema::object(
                "parameters",
                "引数",
                vec![
                    DescribedSchema::string("base_code", "為替の計算元になる ISO 4217 通貨コード。"),
                    DescribedSchema::array(
                        "target_codes",
                        "為替の計算先になる ISO 4217 通貨コードのリスト。",
                        DescribedSchema::string("code", "通貨コード"),
                    ),
                ],
            ),
        }
    }

    fn call<'a>(&'a self, _id: &str, params: Value) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>> {
        let parameters = match serde_json::from_value(params).map_err(FunctionError::by_serialization) {
            Ok(p) => p,
            Err(err) => return async { Err(FunctionError::Serialization(err.into())) }.boxed(),
        };
        async move { self.get_exchange_rate(parameters).await }.boxed()
    }
}

impl ExchangeRate {
    async fn get_exchange_rate(&self, parameters: RequestParameters) -> Result<FunctionResponse, FunctionError> {
        let response = self
            .client
            .get(format!("{}/latest/{}", self.token_endpoint, parameters.base_code))
            .send()
            .map_err(FunctionError::by_external)
            .await?;
        let api_response: ExchangeRateApiResponse = response.json().map_err(FunctionError::by_serialization).await?;

        let updated_at = OffsetDateTime::parse(&api_response.time_last_update_utc, API_RESPONSE_DATETIME)
            .map_err(FunctionError::by_serialization)?
            .format(RFC3339_NUMOFFSET)
            .map_err(FunctionError::by_serialization)?;
        let target_rates: HashMap<_, _> = parameters
            .target_codes
            .into_iter()
            .flat_map(|c| api_response.conversion_rates.get(&c).map(|&r| (c, r)))
            .collect();

        Ok(FunctionResponse {
            result: json!({
                "updated_at": updated_at,
                "base_code": api_response.base_code,
                "target_rates": target_rates,
            }),
            ..Default::default()
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RequestParameters {
    base_code: String,
    target_codes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExchangeRateApiResponse {
    time_last_update_utc: String,
    base_code: String,
    conversion_rates: HashMap<String, f64>,
}
