use crate::llm::create_llm;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use lnb_common::config::llm::{ConfigLlm, ConfigLlmModel};
use lnb_core::{interface::llm::ArcLlm, model::conversation::ConversationModel};
use thiserror::Error as ThisError;
use tokio::sync::RwLock;
use tracing::{debug, warn};

#[derive(Clone)]
pub struct LlmCache {
    default_model: String,
    uninitialized_models: Arc<RwLock<HashMap<String, ConfigLlmModel>>>,
    created_models: Arc<RwLock<HashMap<String, ArcLlm>>>,
    failed_models: Arc<RwLock<HashSet<String>>>,
}

impl LlmCache {
    pub fn new(config: &ConfigLlm) -> LlmCache {
        let uninitialized_models = config.models.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        LlmCache {
            default_model: config.default.clone(),
            uninitialized_models: Arc::new(RwLock::new(uninitialized_models)),
            created_models: Arc::new(RwLock::new(HashMap::new())),
            failed_models: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub async fn get(&self, model: &ConversationModel) -> Result<ArcLlm, LlmCacheError> {
        let key = model.specified_or(&self.default_model);

        let created_llm = {
            let created_lock = self.created_models.read().await;
            created_lock.get(key).cloned()
        };
        if let Some(created_llm) = created_llm {
            return Ok(created_llm);
        }

        let failed_llm_name = {
            let failed_lock = self.failed_models.read().await;
            failed_lock.get(key).cloned()
        };
        if let Some(failed_llm_name) = failed_llm_name {
            return Err(LlmCacheError::Failed(failed_llm_name));
        }

        let creating_llm_config = {
            let mut uninitialized_lock = self.uninitialized_models.write().await;
            uninitialized_lock.remove(key)
        };
        let Some(creating_llm_config) = creating_llm_config else {
            return Err(LlmCacheError::Undefined(key.to_string()));
        };

        match create_llm(creating_llm_config).await {
            Ok(llm) => {
                debug!("initialized and cached LLM {key}");
                let mut created_lock = self.created_models.write().await;
                created_lock.insert(key.to_string(), llm.clone());
                Ok(llm)
            }
            Err(e) => {
                warn!("failed to initialize LLM {key}: {e}");
                let mut failed_lock = self.failed_models.write().await;
                failed_lock.insert(key.to_string());
                Err(LlmCacheError::Failed(key.to_string()))
            }
        }
    }
}

#[derive(Debug, ThisError)]
pub enum LlmCacheError {
    #[error("undefined model: {0}")]
    Undefined(String),

    #[error("model {0} reported initialization failure")]
    Failed(String),
}
