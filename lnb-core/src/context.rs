use crate::interface::time::DateTimeProvider;

use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Context {
    pub system_role: Arc<str>,
    pub sensitive_marker: Arc<str>,
    pub datetime_provider: Arc<dyn DateTimeProvider>,
}
