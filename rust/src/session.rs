use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use rustpush::{
    APSConnection, APSConnectionResource, APSState, AppleAccount, ArcAnisetteClient,
    DefaultAnisetteProvider, IDSNGMIdentity, IDSUser, LoginDelegate, LoginState, OSConfig,
    RelayConfig, authenticate_apple, default_provider, login_apple_delegates,
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
    pub ids_user: Arc<Mutex<Option<IDSUser>>>,
}

fn aps_state_path(data_dir: &Path) -> PathBuf { data_dir.join("aps_state.json") }
fn identity_path(data_dir: &Path) -> PathBuf { data_dir.join("identity.json") }
fn ids_user_path(data_dir: &Path) -> PathBuf { data_dir.join("ids_user.json") }
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

    let ids_user: Option<IDSUser> = load_json(&ids_user_path(data_dir)).unwrap_or(None);

    Ok(AuthSession {
        data_dir: data_dir.to_path_buf(),
        config: config_arc,
        identity: Arc::new(identity),
        conn,
        anisette,
        account: Arc::new(AsyncMutex::new(account)),
        ids_user: Arc::new(Mutex::new(ids_user)),
    })
}

pub async fn finalize_login_and_register_ids(session: &AuthSession) -> Result<IDSUser> {
    {
        let mut account = session.account.lock().await;
        account
            .update_postdata("WhatBubbles", None, &["icloud", "imessage", "facetime"])
            .await
            .map_err(|e| anyhow!("update_postdata: {e:?}"))?;
        if account.get_pet().is_none() {
            return Err(anyhow!("no PET after login — auth incomplete"));
        }
    }

    let os_config: Arc<dyn OSConfig> = session.config.clone();
    let delegates = {
        let account = session.account.lock().await;
        login_apple_delegates(
            &*account,
            None,
            os_config.as_ref(),
            &[LoginDelegate::IDS, LoginDelegate::MobileMe],
        )
        .await
        .map_err(|e| anyhow!("login_apple_delegates: {e:?}"))?
    };

    let ids_delegate = delegates
        .ids
        .ok_or_else(|| anyhow!("IDS delegate missing in login response"))?;

    let user = authenticate_apple(ids_delegate, os_config.as_ref())
        .await
        .map_err(|e| anyhow!("authenticate_apple: {e:?}"))?;

    save_json(&ids_user_path(&session.data_dir), &user)?;
    *session.ids_user.lock() = Some(user.clone());
    Ok(user)
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
