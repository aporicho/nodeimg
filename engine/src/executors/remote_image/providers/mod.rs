use std::sync::Arc;

use super::credential_store::{ChainedCredentialStore, EnvCredentialStore, JsonCredentialStore};
use super::provider::ImageGenerationProvider;

pub mod libtv;

pub fn builtin_providers() -> Vec<Arc<dyn ImageGenerationProvider>> {
    let credential_store = Arc::new(ChainedCredentialStore::new(vec![
        Arc::new(EnvCredentialStore),
        Arc::new(JsonCredentialStore::default_libtv_capture()),
    ]));

    vec![Arc::new(libtv::LibTvProvider::new(credential_store))]
}
