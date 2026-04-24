use anyhow::{anyhow, bail, Result};
use rustpush::LoginState;

use crate::session::{self, hash_password, AuthSession};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthState {
    NeedsHardwarePairing,
    NeedsCredentials,
    AuthenticatingAccount,
    NeedsTwoFactor,
    NeedsSmsTwoFactor,
    NeedsDeviceTwoFactor,
    NeedsExtraStep(String),
    Ready,
    Errored(String),
}

impl AuthState {
    pub fn label(&self) -> String {
        match self {
            AuthState::NeedsHardwarePairing => "needs_pairing".into(),
            AuthState::NeedsCredentials => "needs_credentials".into(),
            AuthState::AuthenticatingAccount => "authenticating".into(),
            AuthState::NeedsTwoFactor => "needs_2fa".into(),
            AuthState::NeedsSmsTwoFactor => "needs_sms_2fa".into(),
            AuthState::NeedsDeviceTwoFactor => "needs_device_2fa".into(),
            AuthState::NeedsExtraStep(s) => format!("needs_extra:{s}"),
            AuthState::Ready => "ready".into(),
            AuthState::Errored(e) => format!("error:{e}"),
        }
    }

    pub fn from_login_state(ls: &LoginState) -> Self {
        match ls {
            LoginState::LoggedIn => AuthState::Ready,
            LoginState::Needs2FAVerification => AuthState::NeedsTwoFactor,
            LoginState::NeedsSMS2FA => AuthState::NeedsSmsTwoFactor,
            LoginState::NeedsSMS2FAVerification(_) => AuthState::NeedsSmsTwoFactor,
            LoginState::NeedsDevice2FA => AuthState::NeedsDeviceTwoFactor,
            LoginState::NeedsExtraStep(s) => AuthState::NeedsExtraStep(s.clone()),
            LoginState::NeedsLogin => AuthState::NeedsCredentials,
        }
    }
}

pub async fn authenticate_apple_id(
    session: &AuthSession,
    apple_id: &str,
    password: &str,
) -> Result<LoginState> {
    if apple_id.is_empty() || password.is_empty() {
        bail!("apple_id and password are required");
    }
    let hashed = hash_password(password);
    let mut account = session.account.lock().await;
    let state = account
        .login_email_pass(apple_id, &hashed)
        .await
        .map_err(|e| anyhow!("login_email_pass: {e:?}"))?;
    log::info!("login_email_pass → {}", session::login_state_label(&state));
    Ok(state)
}

pub async fn submit_2fa_code(session: &AuthSession, code: &str) -> Result<LoginState> {
    if code.is_empty() {
        bail!("2fa code is required");
    }
    let mut account = session.account.lock().await;
    let state = account
        .verify_2fa(code.to_string())
        .await
        .map_err(|e| anyhow!("verify_2fa: {e:?}"))?;
    log::info!("verify_2fa → {}", session::login_state_label(&state));
    Ok(state)
}

pub async fn send_imessage(
    _chat_guid: &str,
    _participant_addresses: &[String],
    _text: &str,
) -> Result<String> {
    bail!("send_imessage: not yet wired — needs IDS registration (do_login) and IMClient; next commit");
}

pub async fn send_imessage_with_attachments(
    _chat_guid: &str,
    _participant_addresses: &[String],
    _text: &str,
    _attachment_paths: &[String],
) -> Result<String> {
    bail!("send_imessage_with_attachments: not yet wired — depends on MMCS upload path");
}

pub async fn send_tapback(_message_guid: &str, _reaction: &str) -> Result<()> {
    bail!("send_tapback: not yet wired — tapback path in api.rs");
}

pub async fn send_edit(_message_guid: &str, _new_text: &str) -> Result<()> {
    bail!("send_edit: not yet wired — edit path in api.rs");
}

pub async fn send_unsend(_message_guid: &str) -> Result<()> {
    bail!("send_unsend: not yet wired — unsend path in api.rs");
}

pub async fn send_typing(_chat_guid: &str, _typing: bool) -> Result<()> {
    bail!("send_typing: not yet wired — typing indicator path in api.rs");
}

pub async fn start_facetime(_address: &str) -> Result<String> {
    bail!("start_facetime: not yet wired — FaceTime path in api.rs");
}

pub async fn poll_findmy_friends() -> Result<Vec<(String, f64, f64)>> {
    bail!("poll_findmy_friends: not yet wired — FindMy path in api.rs");
}

pub async fn list_shared_albums() -> Result<Vec<String>> {
    bail!("list_shared_albums: not yet wired — shared albums path in api.rs");
}
