mod claude;
mod openai;

use self::openai::{ChatCompletionBackend, ResponsesBackend};

use std::{collections::HashMap, sync::LazyLock};

use lnb_core::{
    config::{AppConfigLlm, AppConfigLlmBackend, AppConfigLlmOpenaiApi},
    error::LlmError,
    interface::llm::Llm,
    model::schema::{DescribedSchema, DescribedSchemaType},
};
use serde_json::{Value, json};

// MEMO: proc macro で serde のついでに作った方が面白い
pub static ASSISTANT_RESPONSE_SCHEMA: LazyLock<DescribedSchema> = LazyLock::new(|| {
    DescribedSchema::object(
        "response",
        "response as assistant",
        vec![
            DescribedSchema::string(
                "text",
                "ユーザーへの主要な回答内容。夏稀としてふるまって回答してください。",
            ),
            DescribedSchema::string("language", "`text` フィールドに対応する IETF BCP47 言語タグ。"),
            DescribedSchema::boolean("sensitive", "`text` フィールドが性的な話題を含むかどうか。"),
        ],
    )
});

pub async fn create_llm(config: &AppConfigLlm) -> Result<Box<dyn Llm + 'static>, LlmError> {
    match config.backend {
        AppConfigLlmBackend::Openai => match config.openai.api {
            AppConfigLlmOpenaiApi::ChatCompletion => Ok(Box::new(ChatCompletionBackend::new(&config.openai).await?)),
            AppConfigLlmOpenaiApi::Resnposes => Ok(Box::new(ResponsesBackend::new(&config.openai).await?)),
        },
    }
}

fn convert_json_schema(schema: &DescribedSchema) -> Value {
    match &schema.field_type {
        DescribedSchemaType::Integer => json!({
            "type": "integer",
            "description": schema.description,
        }),
        DescribedSchemaType::Float => json!({
            "type": "float",
            "description": schema.description,
        }),
        DescribedSchemaType::Boolean => json!({
            "type": "boolean",
            "description": schema.description,
        }),
        DescribedSchemaType::String => json!({
            "type": "string",
            "description": schema.description,
        }),
        DescribedSchemaType::Object(fields) => {
            let properties: HashMap<_, _> = fields
                .iter()
                .map(|f| (f.name.clone(), convert_json_schema(f)))
                .collect();
            let keys: Vec<_> = properties.keys().cloned().collect();
            json!({
                "type": "object",
                "properties": properties,
                "required": keys,
                "additionalProperties": false,
            })
        }
    }
}
