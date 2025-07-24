use std::sync::Arc;

use lnb_core::interface::time::DateTimeProvider;

#[derive(Debug, Clone)]
pub struct NatsukiContext {
    pub system_role: Arc<str>,
    pub sensitive_marker: Arc<str>,
    pub datetime_provider: Arc<dyn DateTimeProvider>,
}
