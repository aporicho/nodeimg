use std::sync::Arc;

use crate::executors::remote_image::credential_store::{
    ChainedCredentialStore, EnvCredentialStore, JsonCredentialStore,
};

use super::provider::VideoGenerationProvider;

pub mod libtv;
pub mod mock;

pub fn builtin_providers() -> Vec<Arc<dyn VideoGenerationProvider>> {
    vec![libtv_provider()]
}

pub fn libtv_provider() -> Arc<dyn VideoGenerationProvider> {
    let credential_store = Arc::new(ChainedCredentialStore::new(vec![
        Arc::new(EnvCredentialStore),
        Arc::new(JsonCredentialStore::default_libtv_capture()),
    ]));

    Arc::new(libtv::LibTvVideoProvider::new(credential_store))
}

pub fn mock_provider() -> Arc<dyn VideoGenerationProvider> {
    Arc::new(mock::MockVideoGenerationProvider::new())
}
