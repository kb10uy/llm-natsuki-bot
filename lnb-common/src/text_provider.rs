use lnb_core::interface::text::TextProvider;

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
