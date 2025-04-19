mod claude;
mod openai;

use self::openai::{ChatCompletionBackend, ResponsesBackend};
use crate::config::{AppConfigLlm, AppConfigLlmBackend, AppConfigLlmModel, AppConfigLlmOpenaiApi};

use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use lnb_core::{
    error::LlmError,
    interface::llm::ArcLlm,
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

pub async fn create_llm(config: AppConfigLlmModel) -> Result<ArcLlm, LlmError> {
    match config.backend {
        AppConfigLlmBackend::Openai => match config.openai.api {
            AppConfigLlmOpenaiApi::ChatCompletion => Ok(Arc::new(ChatCompletionBackend::new(&config.openai).await?)),
            AppConfigLlmOpenaiApi::Resnposes => Ok(Arc::new(ResponsesBackend::new(&config.openai).await?)),
        },
    }
}

fn convert_json_schema(schema: &DescribedSchema) -> Value {
    let type_value = |s| if schema.optional { json!([s, "null"]) } else { json!(s) };
    match &schema.field_type {
        DescribedSchemaType::Integer => json!({
            "type": type_value("integer"),
            "description": schema.description,
        }),
        DescribedSchemaType::Float => json!({
            "type": type_value("float"),
            "description": schema.description,
        }),
        DescribedSchemaType::Boolean => json!({
            "type": type_value("boolean"),
            "description": schema.description,
        }),
        DescribedSchemaType::String => json!({
            "type": type_value("string"),
            "description": schema.description,
        }),
        DescribedSchemaType::Array(item_type) => json!({
            "type": type_value("array"),
            "description": schema.description,
            "items": convert_json_schema(item_type),
        }),
        DescribedSchemaType::Object(fields) => {
            let properties: HashMap<_, _> = fields
                .iter()
                .map(|f| (f.name.clone(), convert_json_schema(f)))
                .collect();
            let keys: Vec<_> = properties.keys().cloned().collect();
            json!({
                "type": type_value("object"),
                "properties": properties,
                "required": keys,
                "additionalProperties": false,
            })
        }
    }
}
