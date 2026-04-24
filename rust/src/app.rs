use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::Mutex;
use rustpush::RelayConfig;

use crate::events::{Event, EventBus};
use crate::integration::AuthState;
use crate::os_config;
use crate::session::AuthSession;
use crate::storage::Storage;

pub struct AppState {
    pub data_dir: PathBuf,
    pub storage: Arc<Mutex<Storage>>,
    pub events: EventBus,
    pub runtime_handle: tokio::runtime::Handle,
    pub auth: Arc<Mutex<AuthState>>,
    pub os_config: Arc<Mutex<Option<RelayConfig>>>,
    pub relay_host: Arc<Mutex<String>>,
    pub session: Arc<Mutex<Option<Arc<AuthSession>>>>,
}

impl AppState {
    pub fn open(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)
            .with_context(|| format!("creating data dir at {}", data_dir.display()))?;
        let storage = Storage::open(&data_dir.join("whatbubbles.db"))?;
        storage.seed_demo_data()?;

        let os_config_loaded = os_config::load(&os_config::config_path(data_dir))
            .unwrap_or(None);

        let relay_host = storage
            .kv_get("relay.host")?
            .unwrap_or_else(|| os_config::DEFAULT_RELAY_HOST.to_string());

        let auth_state = if os_config_loaded.is_some() {
            match storage.kv_get("auth.state")?.as_deref() {
                Some("ready") => AuthState::Ready,
                Some("authenticating") => AuthState::AuthenticatingAccount,
                Some("needs_2fa") => AuthState::NeedsTwoFactor,
                Some("needs_sms_2fa") => AuthState::NeedsSmsTwoFactor,
                Some("needs_device_2fa") => AuthState::NeedsDeviceTwoFactor,
                Some(label) if label.starts_with("needs_extra:") => {
                    AuthState::NeedsExtraStep(label.trim_start_matches("needs_extra:").to_string())
                }
                Some(label) if label.starts_with("error:") => {
                    AuthState::Errored(label.trim_start_matches("error:").to_string())
                }
                _ => AuthState::NeedsCredentials,
            }
        } else {
            AuthState::NeedsHardwarePairing
        };

        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            storage: Arc::new(Mutex::new(storage)),
            events: EventBus::new(),
            runtime_handle: crate::RUNTIME.handle().clone(),
            auth: Arc::new(Mutex::new(auth_state)),
            os_config: Arc::new(Mutex::new(os_config_loaded)),
            relay_host: Arc::new(Mutex::new(relay_host)),
            session: Arc::new(Mutex::new(None)),
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

    pub fn set_relay_host(&self, host: String) -> Result<()> {
        *self.relay_host.lock() = host.clone();
        self.storage.lock().kv_set("relay.host", &host)?;
        Ok(())
    }

    pub fn relay_host(&self) -> String {
        self.relay_host.lock().clone()
    }

    pub fn persist_os_config(&self, cfg: RelayConfig) -> Result<()> {
        let path = os_config::config_path(&self.data_dir);
        os_config::save(&path, &cfg)?;
        *self.os_config.lock() = Some(cfg);
        Ok(())
    }

    pub fn clear_os_config(&self) -> Result<()> {
        os_config::clear(&os_config::config_path(&self.data_dir))?;
        *self.os_config.lock() = None;
        Ok(())
    }

    pub fn has_os_config(&self) -> bool {
        self.os_config.lock().is_some()
    }
}
