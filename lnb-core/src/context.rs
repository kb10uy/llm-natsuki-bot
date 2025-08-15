use crate::interface::{text::TextProvider, time::DateTimeProvider};

use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Context {
    pub system_role: Arc<dyn TextProvider<Data = ()>>,
    pub sensitive_marker: Arc<str>,
    pub datetime_provider: Arc<dyn DateTimeProvider>,
}
