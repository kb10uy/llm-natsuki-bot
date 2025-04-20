use std::{collections::HashMap, sync::Arc};

use lnb_core::{
    error::FunctionError,
    interface::{
        Context,
        function::{FunctionDescriptor, FunctionResponse, complex::ArcComplexFunction, simple::ArcSimpleFunction},
    },
    model::{
        conversation::{IncompleteConversation, UserRole},
        message::MessageToolCalling,
    },
};

#[derive(Clone)]
pub struct FunctionStore {
    simple_functions: Arc<HashMap<String, (ArcSimpleFunction, FunctionDescriptor)>>,
    complex_functions: Arc<HashMap<String, (ArcComplexFunction, FunctionDescriptor)>>,
}

impl FunctionStore {
    pub fn new(
        simple_functions: impl IntoIterator<Item = ArcSimpleFunction>,
        complex_functions: impl IntoIterator<Item = ArcComplexFunction>,
    ) -> FunctionStore {
        let simple_functions = simple_functions
            .into_iter()
            .map(|f| {
                let descriptor = f.get_descriptor();
                (descriptor.name.clone(), (f, descriptor))
            })
            .collect();
        let complex_functions = complex_functions
            .into_iter()
            .map(|f| {
                let descriptor = f.get_descriptor();
                (descriptor.name.clone(), (f, descriptor))
            })
            .collect();
        FunctionStore {
            simple_functions: Arc::new(simple_functions),
            complex_functions: Arc::new(complex_functions),
        }
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &FunctionDescriptor> {
        let simple_descriptors = self.simple_functions.iter().map(|(_, (_, d))| d);
        let complex_descriptors = self.complex_functions.iter().map(|(_, (_, d))| d);
        simple_descriptors.chain(complex_descriptors)
    }

    pub async fn find_call(
        &self,
        tool_calling: MessageToolCalling,
        context: &Context,
        incomplete: &IncompleteConversation,
        role: &UserRole,
    ) -> Option<Result<FunctionResponse, FunctionError>> {
        if let Some((simple_function, _)) = self.simple_functions.get(&tool_calling.name) {
            let result = simple_function.call(&tool_calling.id, tool_calling.arguments).await;
            Some(result)
        } else if let Some((complex_function, _)) = self.complex_functions.get(&tool_calling.name) {
            let result = complex_function.call(context, incomplete, role, tool_calling).await;
            Some(result)
        } else {
            None
        }
    }
}
