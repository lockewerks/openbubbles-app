use anyhow::{bail, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthState {
    NotConfigured,
    AuthenticatingAccount,
    DevicePairingRequired,
    Ready,
    Errored(String),
}

impl AuthState {
    pub fn label(&self) -> String {
        match self {
            AuthState::NotConfigured => "not_configured".into(),
            AuthState::AuthenticatingAccount => "authenticating".into(),
            AuthState::DevicePairingRequired => "device_pairing".into(),
            AuthState::Ready => "ready".into(),
            AuthState::Errored(e) => format!("error:{e}"),
        }
    }
}

pub async fn authenticate_apple_id(_apple_id: &str, _password: &str) -> Result<()> {
    bail!("authenticate_apple_id: not yet wired — port from rustpush apple-private-apis / icloud-auth; parked entry point in rust/src/api/api.rs::do_first_time_init");
}

pub async fn submit_2fa_code(_code: &str) -> Result<()> {
    bail!("submit_2fa_code: not yet wired — see rust/src/api/api.rs::get_2fa_code flow");
}

pub async fn request_device_pairing_code() -> Result<String> {
    bail!("request_device_pairing_code: not yet wired — hw.openbubbles.app /code endpoint; parked in lib/services/rustpush/rustpush_service.dart::gen_code");
}

pub async fn complete_device_pairing(_code: &str) -> Result<()> {
    bail!("complete_device_pairing: not yet wired — parked in lib/services/rustpush/rustpush_service.dart::do_hw_activation");
}

pub async fn send_imessage(
    _chat_guid: &str,
    _participant_addresses: &[String],
    _text: &str,
) -> Result<String> {
    bail!("send_imessage: not yet wired — port from parked rust/src/api/api.rs::send");
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
