use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::Mutex;

use crate::events::{Event, EventBus};
use crate::integration::AuthState;
use crate::storage::Storage;

pub struct AppState {
    pub data_dir: PathBuf,
    pub storage: Arc<Mutex<Storage>>,
    pub events: EventBus,
    pub runtime_handle: tokio::runtime::Handle,
    pub auth: Arc<Mutex<AuthState>>,
}

impl AppState {
    pub fn open(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)
            .with_context(|| format!("creating data dir at {}", data_dir.display()))?;
        let storage = Storage::open(&data_dir.join("whatbubbles.db"))?;
        storage.seed_demo_data()?;
        let auth_state = storage
            .kv_get("auth.state")?
            .map(AuthState::from_label)
            .unwrap_or(AuthState::NotConfigured);
        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            storage: Arc::new(Mutex::new(storage)),
            events: EventBus::new(),
            runtime_handle: crate::RUNTIME.handle().clone(),
            auth: Arc::new(Mutex::new(auth_state)),
        })
    }

    pub fn set_auth(&self, state: AuthState) {
        let label = state.label();
        *self.auth.lock() = state;
        if let Err(e) = self.storage.lock().kv_set("auth.state", &label) {
            self.events.send(Event::Warn(format!("failed to persist auth state: {e}")));
        }
        self.events.send(Event::AuthStateChanged { state: label });
    }

    pub fn auth_label(&self) -> String {
        self.auth.lock().label()
    }
}

impl AuthState {
    fn from_label(label: String) -> Self {
        match label.as_str() {
            "not_configured" => AuthState::NotConfigured,
            "authenticating" => AuthState::AuthenticatingAccount,
            "device_pairing" => AuthState::DevicePairingRequired,
            "ready" => AuthState::Ready,
            other if other.starts_with("error:") => {
                AuthState::Errored(other.trim_start_matches("error:").to_string())
            }
            _ => AuthState::NotConfigured,
        }
    }
}
