use base64::{Engine, prelude::BASE64_STANDARD};
use lnb_core::interface::Context;
use sha2::{Digest, Sha256};

pub const SYSTEM_IDENTITY: &str = "*system*";

pub trait ContextExt {
    fn hashed_identity(&self) -> String;
}

impl ContextExt for Context {
    fn hashed_identity(&self) -> String {
        let identity = self.identity().unwrap_or(SYSTEM_IDENTITY);
        BASE64_STANDARD.encode(Sha256::digest(identity))
    }
}
