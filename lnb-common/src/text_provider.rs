use lnb_core::interface::text::TextProvider;

#[derive(Debug, Clone)]
pub struct FixedTextProvider {
    source: String,
}

impl TextProvider for FixedTextProvider {
    type Data = ();

    fn generate(&self, _: ()) -> String {
        self.source.clone()
    }
}
