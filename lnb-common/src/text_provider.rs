use std::collections::HashMap;

use lnb_core::interface::text::TextProvider;
use upon::{Engine, Result as UponResult, Template};

#[derive(Debug, Clone)]
pub struct FixedTextProvider {
    source: String,
}

impl FixedTextProvider {
    pub fn new(source: impl Into<String>) -> FixedTextProvider {
        FixedTextProvider { source: source.into() }
    }
}

impl TextProvider for FixedTextProvider {
    type Data = ();

    fn generate(&self, _: ()) -> String {
        self.source.clone()
    }
}

impl From<String> for FixedTextProvider {
    fn from(value: String) -> Self {
        FixedTextProvider::new(value)
    }
}

#[derive(Debug)]
pub struct InterpolatableTextProvider {
    engine: Engine<'static>,
    template: Template<'static>,
}

impl InterpolatableTextProvider {
    pub fn new(source: impl Into<String>) -> UponResult<InterpolatableTextProvider> {
        let engine = Engine::new();
        let template = engine.compile(source.into())?;
        Ok(InterpolatableTextProvider { engine, template })
    }
}

impl TextProvider for InterpolatableTextProvider {
    type Data = HashMap<String, String>;

    fn generate(&self, data: Self::Data) -> String {
        self.template.render(&self.engine, data).to_string().unwrap_or_default()
    }
}
