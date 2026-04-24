use anyhow::{bail, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthState {
    NeedsHardwarePairing,
    NeedsCredentials,
    AuthenticatingAccount,
    NeedsTwoFactor,
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
            AuthState::Ready => "ready".into(),
            AuthState::Errored(e) => format!("error:{e}"),
        }
    }
}

pub async fn authenticate_apple_id(_apple_id: &str, _password: &str) -> Result<()> {
    bail!("authenticate_apple_id: APS connection + anisette setup not yet wired — next session (see rust/src/api/api.rs::try_auth)");
}

pub async fn submit_2fa_code(_code: &str) -> Result<()> {
    bail!("submit_2fa_code: depends on authenticate_apple_id — next session");
}

pub async fn send_imessage(
    _chat_guid: &str,
    _participant_addresses: &[String],
    _text: &str,
) -> Result<String> {
    bail!("send_imessage: not yet wired — port from parked rust/src/api/api.rs::send");
}

pub async fn send_imessage_with_attachments(
    _chat_guid: &str,
    _participant_addresses: &[String],
    _text: &str,
    _attachment_paths: &[String],
) -> Result<String> {
    bail!("send_imessage_with_attachments: not yet wired — depends on MMCS upload path in api.rs + IMClient");
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
