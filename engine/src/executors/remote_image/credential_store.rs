use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;

pub trait CredentialStore: Send + Sync {
    fn get_secret(&self, provider_id: &str, key: &str) -> Option<String>;
}

#[derive(Default)]
pub struct EnvCredentialStore;

impl CredentialStore for EnvCredentialStore {
    fn get_secret(&self, provider_id: &str, key: &str) -> Option<String> {
        match (provider_id, key) {
            ("libtv", "token") => std::env::var("LIBTV_TOKEN").ok(),
            ("libtv", "webid") => std::env::var("LIBTV_WEBID").ok(),
            ("libtv", "project_id") => std::env::var("LIBTV_PROJECT_ID").ok(),
            _ => None,
        }
    }
}

pub struct JsonCredentialStore {
    provider_id: String,
    path: PathBuf,
}

impl JsonCredentialStore {
    pub fn new(provider_id: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            provider_id: provider_id.into(),
            path: path.into(),
        }
    }

    pub fn default_libtv_capture() -> Self {
        let path = std::env::var("LIBTV_CREDS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp/libtv_reverse/scripts/creds.json"));
        Self::new("libtv", path)
    }
}

#[derive(Deserialize)]
struct LibTvCreds {
    token: Option<String>,
    webid: Option<String>,
    project_id: Option<String>,
}

impl CredentialStore for JsonCredentialStore {
    fn get_secret(&self, provider_id: &str, key: &str) -> Option<String> {
        if provider_id != self.provider_id {
            return None;
        }

        let raw = fs::read_to_string(&self.path).ok()?;
        let parsed: LibTvCreds = serde_json::from_str(&raw).ok()?;
        match key {
            "token" => parsed.token,
            "webid" => parsed.webid,
            "project_id" => parsed.project_id,
            _ => None,
        }
    }
}

pub struct ChainedCredentialStore {
    stores: Vec<Arc<dyn CredentialStore>>,
}

impl ChainedCredentialStore {
    pub fn new(stores: Vec<Arc<dyn CredentialStore>>) -> Self {
        Self { stores }
    }
}

impl CredentialStore for ChainedCredentialStore {
    fn get_secret(&self, provider_id: &str, key: &str) -> Option<String> {
        self.stores
            .iter()
            .find_map(|store| store.get_secret(provider_id, key))
    }
}
