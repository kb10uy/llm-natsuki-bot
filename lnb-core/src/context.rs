use crate::interface::{text::TextProvider, time::DateTimeProvider};

use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone)]
pub struct Context {
    pub system_role: Arc<dyn TextProvider<Data = HashMap<String, String>>>,
    pub sensitive_marker: Arc<str>,
    pub datetime_provider: Arc<dyn DateTimeProvider>,
}
