pub mod complex;
pub mod simple;

use crate::model::{conversation::ConversationAttachment, schema::DescribedSchema};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionDescriptor {
    pub name: String,
    pub description: String,
    pub parameters: DescribedSchema,
}

#[derive(Debug, Clone, Default)]
pub struct FunctionResponse {
    pub result: Value,
    pub attachments: Vec<ConversationAttachment>,
}
