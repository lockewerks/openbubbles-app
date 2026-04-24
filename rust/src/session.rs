use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use rustpush::{
    APSConnection, APSConnectionResource, APSState, AppleAccount, ArcAnisetteClient,
    DefaultAnisetteProvider, IDSNGMIdentity, LoginState, OSConfig, RelayConfig, default_provider,
};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex as AsyncMutex;

pub struct AuthSession {
    pub data_dir: PathBuf,
    pub config: Arc<RelayConfig>,
    pub identity: Arc<IDSNGMIdentity>,
    pub conn: APSConnection,
    pub anisette: ArcAnisetteClient<DefaultAnisetteProvider>,
    pub account: Arc<AsyncMutex<AppleAccount<DefaultAnisetteProvider>>>,
}

fn aps_state_path(data_dir: &Path) -> PathBuf { data_dir.join("aps_state.json") }
fn identity_path(data_dir: &Path) -> PathBuf { data_dir.join("identity.json") }
fn anisette_dir(data_dir: &Path) -> PathBuf { data_dir.join("anisette") }

fn load_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    if !path.exists() { return Ok(None); }
    let s = std::fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&s)?))
}

fn save_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub async fn create_session(data_dir: &Path, config: &RelayConfig) -> Result<AuthSession> {
    std::fs::create_dir_all(data_dir)?;
    std::fs::create_dir_all(anisette_dir(data_dir))?;

    let config_arc: Arc<RelayConfig> = Arc::new(config.clone());
    let os_config: Arc<dyn OSConfig> = config_arc.clone();

    let saved_aps: Option<APSState> = load_json(&aps_state_path(data_dir)).unwrap_or(None);

    let (conn, err) = APSConnectionResource::new(os_config.clone(), saved_aps).await;
    if let Some(e) = err {
        return Err(anyhow!("APS connection failed: {e:?}"));
    }

    let current_aps = conn.state.read().await.clone();
    save_json(&aps_state_path(data_dir), &current_aps)?;

    let identity = if let Ok(Some(id)) = load_json::<IDSNGMIdentity>(&identity_path(data_dir)) {
        id
    } else {
        let fresh = IDSNGMIdentity::new().map_err(|e| anyhow!("new identity: {e:?}"))?;
        save_json(&identity_path(data_dir), &fresh)?;
        fresh
    };

    let login_config = os_config.get_gsa_config(&current_aps, false);
    let anisette = default_provider(login_config.clone(), anisette_dir(data_dir));

    let account = AppleAccount::new_with_anisette(login_config, anisette.clone())
        .map_err(|e| anyhow!("new apple account: {e:?}"))?;

    Ok(AuthSession {
        data_dir: data_dir.to_path_buf(),
        config: config_arc,
        identity: Arc::new(identity),
        conn,
        anisette,
        account: Arc::new(AsyncMutex::new(account)),
    })
}

pub fn hash_password(password: &str) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(password.as_bytes());
    h.finalize().to_vec()
}

pub fn login_state_label(state: &LoginState) -> &'static str {
    match state {
        LoginState::LoggedIn => "logged_in",
        LoginState::NeedsDevice2FA => "needs_device_2fa",
        LoginState::Needs2FAVerification => "needs_2fa",
        LoginState::NeedsSMS2FA => "needs_sms_2fa",
        LoginState::NeedsSMS2FAVerification(_) => "needs_sms_2fa_verify",
        LoginState::NeedsExtraStep(_) => "needs_extra_step",
        LoginState::NeedsLogin => "needs_login",
    }
}
