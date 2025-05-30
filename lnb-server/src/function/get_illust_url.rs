use crate::function::ConfigurableFunction;

use futures::{FutureExt, TryFutureExt, future::BoxFuture};
use lnb_common::config::tools::ConfigToolsGetIllustUrl;
use lnb_core::{
    error::FunctionError,
    interface::{
        Context,
        function::{Function, FunctionDescriptor, FunctionResponse},
    },
    model::{conversation::IncompleteConversation, message::MessageToolCalling, schema::DescribedSchema},
};
use lnb_rate_limiter::RateLimiter;
use rand::{rng, seq::IndexedRandom};
use serde::Serialize;
use serde_json::json;
use sqlx::{SqlitePool, prelude::FromRow};

#[derive(Debug)]
pub struct GetIllustUrl {
    pool: SqlitePool,
}

impl ConfigurableFunction for GetIllustUrl {
    const NAME: &'static str = stringify!(GetIllustUrl);

    type Configuration = ConfigToolsGetIllustUrl;

    async fn configure(
        config: &ConfigToolsGetIllustUrl,
        _: Option<RateLimiter>,
    ) -> Result<GetIllustUrl, FunctionError> {
        let pool = SqlitePool::connect(&config.database_filepath)
            .map_err(FunctionError::by_external)
            .await?;
        Ok(GetIllustUrl { pool })
    }
}

impl Function for GetIllustUrl {
    fn get_descriptor(&self) -> FunctionDescriptor {
        FunctionDescriptor {
            name: "get_illust_url".to_string(),
            description: r#"
                この bot 自身をキャラクターとして描写したイラストの URL を取得する。
                自画像・自撮りを要求された場合もこれを利用する。
            "#
            .to_string(),
            parameters: DescribedSchema::object(
                "parameters",
                "引数",
                vec![DescribedSchema::integer("count", "要求したいイラストの URL の数")],
            ),
        }
    }

    fn call<'a>(
        &'a self,
        _context: &'a Context,
        _incomplete: &'a IncompleteConversation,
        tool_calling: MessageToolCalling,
    ) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>> {
        let count = tool_calling.arguments["count"].as_u64().unwrap_or(1) as usize;
        async move { self.get_illust_infos(count).await }.boxed()
    }
}

impl GetIllustUrl {
    async fn get_illust_infos(&self, count: usize) -> Result<FunctionResponse, FunctionError> {
        let all_illusts: Vec<SqliteRowIllust> = sqlx::query_as(r#"SELECT url, creator_name, comment FROM illusts;"#)
            .fetch_all(&self.pool)
            .map_err(FunctionError::by_external)
            .await?;

        let limited_count = count.min(4).min(all_illusts.len());
        let selected_illusts: Vec<_> = all_illusts.choose_multiple(&mut rng(), limited_count).collect();

        Ok(FunctionResponse {
            result: json!({
                "illusts": selected_illusts
            }),
            ..Default::default()
        })
    }
}

#[derive(Debug, Serialize, FromRow)]
pub struct SqliteRowIllust {
    pub url: String,
    pub creator_name: String,
    pub comment: String,
}
