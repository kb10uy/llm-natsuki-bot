use std::{collections::HashMap, sync::Arc};

use lnb_core::{
    error::FunctionError,
    interface::{
        Context,
        function::{ArcFunction, FunctionDescriptor, FunctionResponse},
    },
    model::{conversation::IncompleteConversation, message::MessageToolCalling},
};

#[derive(Clone)]
pub struct FunctionStore {
    functions: Arc<HashMap<String, (ArcFunction, FunctionDescriptor)>>,
}

impl FunctionStore {
    pub fn new(functions: impl IntoIterator<Item = ArcFunction>) -> FunctionStore {
        let functions = functions
            .into_iter()
            .map(|f| {
                let descriptor = f.get_descriptor();
                (descriptor.name.clone(), (f, descriptor))
            })
            .collect();
        FunctionStore {
            functions: Arc::new(functions),
        }
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &FunctionDescriptor> {
        self.functions.iter().map(|(_, (_, d))| d)
    }

    pub async fn find_call(
        &self,
        tool_calling: MessageToolCalling,
        context: &Context,
        incomplete: &IncompleteConversation,
    ) -> Option<Result<FunctionResponse, FunctionError>> {
        if let Some((function, _)) = self.functions.get(&tool_calling.name) {
            let result = function.call(context, incomplete, tool_calling).await;
            Some(result)
        } else {
            None
        }
    }
}
