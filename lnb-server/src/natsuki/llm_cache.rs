use crate::llm::create_llm;

use std::{collections::HashMap, future::Future, sync::Arc};

use crate::config::llm::{ConfigLlm, ConfigLlmModel};
use lnb_core::{error::LlmError, interface::llm::ArcLlm, model::conversation::ConversationModel};
use thiserror::Error as ThisError;
use tokio::sync::OnceCell;
use tracing::{debug, warn};

#[derive(Clone)]
pub struct LlmCache {
    default_model: String,
    models: Arc<HashMap<String, ModelEntry>>,
}

struct ModelEntry {
    config: ConfigLlmModel,
    llm: OnceCell<ArcLlm>,
}

impl LlmCache {
    pub fn new(config: &ConfigLlm) -> LlmCache {
        let models = config
            .models
            .iter()
            .map(|(key, model_config)| {
                (
                    key.clone(),
                    ModelEntry {
                        config: model_config.clone(),
                        llm: OnceCell::new(),
                    },
                )
            })
            .collect();

        LlmCache {
            default_model: config.default.clone(),
            models: Arc::new(models),
        }
    }

    pub async fn get(&self, model: &ConversationModel) -> Result<ArcLlm, LlmCacheError> {
        let key = model.specified_or(&self.default_model);
        self.get_or_init(key, create_llm).await
    }

    async fn get_or_init<F, Fut>(&self, key: &str, initialize: F) -> Result<ArcLlm, LlmCacheError>
    where
        F: FnOnce(ConfigLlmModel) -> Fut,
        Fut: Future<Output = Result<ArcLlm, LlmError>>,
    {
        let entry = self
            .models
            .get(key)
            .ok_or_else(|| LlmCacheError::Undefined(key.to_string()))?;
        let config = entry.config.clone();

        match entry.llm.get_or_try_init(|| initialize(config)).await {
            Ok(llm) => {
                debug!("initialized and cached LLM {key}");
                Ok(llm.clone())
            }
            Err(e) => {
                warn!("failed to initialize LLM {key}: {e}");
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::config::llm::ConfigLlmBackend;
    use futures::{FutureExt, future::BoxFuture};
    use lnb_core::{
        interface::{
            function::FunctionDescriptor,
            llm::{Llm, LlmUpdate},
        },
        model::conversation::IncompleteConversation,
    };
    use serde_json::Value;
    use tokio::time::{Duration, sleep};

    struct DummyLlm;

    impl Llm for DummyLlm {
        fn send_conversation<'a>(
            &'a self,
            _conversation: &'a IncompleteConversation,
            _function_descriptors: &'a [&'a FunctionDescriptor],
        ) -> BoxFuture<'a, Result<LlmUpdate, LlmError>> {
            async { unreachable!() }.boxed()
        }
    }

    fn cache() -> LlmCache {
        LlmCache::new(&ConfigLlm {
            default: "test".to_string(),
            models: HashMap::from([(
                "test".to_string(),
                ConfigLlmModel {
                    backend: ConfigLlmBackend::Openai,
                    config: Value::Null,
                },
            )]),
        })
    }

    #[tokio::test]
    async fn concurrent_gets_share_initialization() {
        let cache = cache();
        let initialization_count = Arc::new(AtomicUsize::new(0));

        let tasks = (0..8).map(|_| {
            let cache = cache.clone();
            let initialization_count = initialization_count.clone();
            tokio::spawn(async move {
                cache
                    .get_or_init("test", move |_| async move {
                        initialization_count.fetch_add(1, Ordering::SeqCst);
                        sleep(Duration::from_millis(10)).await;
                        Ok(Arc::new(DummyLlm) as ArcLlm)
                    })
                    .await
            })
        });

        let results = futures::future::join_all(tasks).await;
        let llms: Vec<_> = results
            .into_iter()
            .map(|result| {
                result
                    .expect("task should complete")
                    .expect("initialization should succeed")
            })
            .collect();

        assert_eq!(initialization_count.load(Ordering::SeqCst), 1);
        assert!(llms.windows(2).all(|pair| Arc::ptr_eq(&pair[0], &pair[1])));
    }

    #[tokio::test]
    async fn failed_initialization_can_be_retried() {
        let cache = cache();
        let first = cache
            .get_or_init("test", |_| async {
                Err(LlmError::ExpectationMismatch("temporary failure".to_string()))
            })
            .await;
        assert!(matches!(first, Err(LlmCacheError::Failed(name)) if name == "test"));

        let second = cache
            .get_or_init("test", |_| async { Ok(Arc::new(DummyLlm) as ArcLlm) })
            .await;
        assert!(second.is_ok());
    }
}
