use std::path::PathBuf;

use anyhow::{anyhow, Result};
use uuid::Uuid;

use crate::app::AppState;
use crate::events::Event;
use crate::integration::{self, AuthState};
use crate::storage::{ChatRow, MessageRow};

#[cxx::bridge(namespace = "whatbubbles")]
mod bridge {
    struct ChatSummary {
        guid: String,
        title: String,
        subtitle: String,
        participant_summary: String,
        is_imessage: bool,
        is_group: bool,
        is_pinned: bool,
        is_archived: bool,
        pin_order: i32,
        unread_count: i32,
        last_message_date: i64,
    }

    struct MessageView {
        guid: String,
        chat_guid: String,
        sender_address: String,
        sender_display_name: String,
        text: String,
        subject: String,
        is_from_me: bool,
        date: i64,
        date_read: i64,
        date_edited: i64,
        is_unsent: bool,
        has_attachments: bool,
        thread_origin_guid: String,
    }

    struct HandleInfo {
        id: i64,
        address: String,
        service: String,
        display_name: String,
    }

    struct EventDto {
        kind: String,
        text: String,
        chat_guid: String,
        message_guid: String,
        flag: bool,
    }

    extern "Rust" {
        type AppState;

        fn core_version() -> String;
        fn core_greeting(name: &str) -> String;
        fn default_data_dir() -> String;
        fn init_logger_at(path: &str) -> Result<()>;

        fn init_app(data_dir: &str) -> Result<Box<AppState>>;

        fn data_dir(app: &AppState) -> String;
        fn auth_state(app: &AppState) -> String;
        fn relay_host(app: &AppState) -> String;
        fn set_relay_host(app: &AppState, host: &str) -> Result<()>;
        fn has_os_config(app: &AppState) -> bool;
        fn os_config_summary(app: &AppState) -> String;
        fn list_chats(app: &AppState, include_archived: bool) -> Vec<ChatSummary>;
        fn list_messages(app: &AppState, chat_guid: &str, limit: i64) -> Vec<MessageView>;
        fn list_handles(app: &AppState) -> Vec<HandleInfo>;

        fn send_message_local(app: &AppState, chat_guid: &str, text: &str) -> Result<String>;
        fn mark_chat_read(app: &AppState, chat_guid: &str) -> Result<()>;
        fn pin_chat(app: &AppState, chat_guid: &str, pinned: bool, order: i32) -> Result<()>;
        fn archive_chat(app: &AppState, chat_guid: &str, archived: bool) -> Result<()>;
        fn tapback_local(app: &AppState, message_guid: &str, reaction: &str) -> Result<()>;
        fn edit_message_local(app: &AppState, message_guid: &str, new_text: &str) -> Result<()>;
        fn unsend_message_local(app: &AppState, message_guid: &str) -> Result<()>;

        fn start_apple_id_auth(app: &AppState, apple_id: &str, password: &str) -> Result<()>;
        fn submit_two_factor_code(app: &AppState, code: &str) -> Result<()>;
        fn complete_pairing(app: &AppState, code: &str, beeper_token: &str) -> Result<()>;
        fn clear_pairing(app: &AppState) -> Result<()>;
        fn start_facetime_call(app: &AppState, address: &str) -> Result<String>;

        fn poll_events(app: &AppState) -> Vec<EventDto>;
        fn seed_demo(app: &AppState) -> Result<()>;
    }
}

fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn core_greeting(name: &str) -> String {
    format!("whatbubbles_core says hi, {name}")
}

fn default_data_dir() -> String {
    if let Some(proj) = directories::ProjectDirs::from("", "whatbubbles", "whatbubbles") {
        proj.data_dir().to_string_lossy().into_owned()
    } else if let Ok(home) = std::env::var("USERPROFILE") {
        format!("{home}\\AppData\\Roaming\\whatbubbles\\whatbubbles")
    } else {
        ".".into()
    }
}

fn init_logger_at(path: &str) -> Result<()> {
    let p = PathBuf::from(path);
    std::fs::create_dir_all(&p)?;
    crate::init_logger(&p);
    Ok(())
}

fn init_app(data_dir: &str) -> Result<Box<AppState>> {
    let path = PathBuf::from(data_dir);
    let app = AppState::open(&path)?;
    Ok(Box::new(app))
}

fn data_dir(app: &AppState) -> String {
    app.data_dir.to_string_lossy().into_owned()
}

fn auth_state(app: &AppState) -> String {
    app.auth_label()
}

fn relay_host(app: &AppState) -> String {
    app.relay_host()
}

fn set_relay_host(app: &AppState, host: &str) -> Result<()> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("relay host cannot be empty"));
    }
    app.set_relay_host(trimmed.to_string())
}

fn has_os_config(app: &AppState) -> bool {
    app.has_os_config()
}

fn os_config_summary(app: &AppState) -> String {
    let guard = app.os_config.lock();
    match guard.as_ref() {
        None => "none".to_string(),
        Some(cfg) => format!(
            "host={} code={}… udid={} proto={}",
            cfg.host,
            cfg.code.chars().take(8).collect::<String>(),
            cfg.udid.as_deref().unwrap_or(""),
            cfg.protocol_version
        ),
    }
}

fn list_chats(app: &AppState, include_archived: bool) -> Vec<bridge::ChatSummary> {
    let storage = app.storage.lock();
    let rows = match storage.list_chats(include_archived) {
        Ok(r) => r,
        Err(e) => {
            app.events.send(Event::Error(format!("list_chats: {e}")));
            return vec![];
        }
    };
    rows.into_iter()
        .map(|mut row: ChatRow| {
            let participants = storage.participants_summary(&row.guid).unwrap_or_default();
            row.participant_summary = participants.clone();
            let title = if row.title.is_empty() {
                participants.clone()
            } else {
                row.title.clone()
            };
            bridge::ChatSummary {
                guid: row.guid,
                title,
                subtitle: row.last_message_preview,
                participant_summary: row.participant_summary,
                is_imessage: row.is_imessage,
                is_group: row.is_group,
                is_pinned: row.is_pinned,
                is_archived: row.is_archived,
                pin_order: row.pin_order,
                unread_count: row.unread_count,
                last_message_date: row.last_message_date,
            }
        })
        .collect()
}

fn list_messages(app: &AppState, chat_guid: &str, limit: i64) -> Vec<bridge::MessageView> {
    match app.storage.lock().messages_for_chat(chat_guid, limit.max(1)) {
        Ok(rows) => rows.into_iter().map(message_row_to_view).collect(),
        Err(e) => {
            app.events.send(Event::Error(format!("list_messages: {e}")));
            vec![]
        }
    }
}

fn list_handles(app: &AppState) -> Vec<bridge::HandleInfo> {
    match app.storage.lock().list_handles() {
        Ok(rows) => rows
            .into_iter()
            .map(|h| bridge::HandleInfo {
                id: h.id,
                address: h.address,
                service: h.service,
                display_name: h.display_name,
            })
            .collect(),
        Err(e) => {
            app.events.send(Event::Error(format!("list_handles: {e}")));
            vec![]
        }
    }
}

fn message_row_to_view(m: MessageRow) -> bridge::MessageView {
    bridge::MessageView {
        guid: m.guid,
        chat_guid: m.chat_guid,
        sender_address: m.sender_address,
        sender_display_name: m.sender_display_name,
        text: m.text,
        subject: m.subject,
        is_from_me: m.is_from_me,
        date: m.date,
        date_read: m.date_read,
        date_edited: m.date_edited,
        is_unsent: m.is_unsent,
        has_attachments: m.has_attachments,
        thread_origin_guid: m.thread_origin_guid,
    }
}

fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn send_message_local(app: &AppState, chat_guid: &str, text: &str) -> Result<String> {
    let guid = format!("temp-{}", Uuid::new_v4());
    let row = MessageRow {
        guid: guid.clone(),
        chat_guid: chat_guid.to_string(),
        handle_id: None,
        sender_address: String::new(),
        sender_display_name: String::new(),
        text: text.to_string(),
        subject: String::new(),
        is_from_me: true,
        date: now_epoch(),
        date_read: 0,
        date_edited: 0,
        is_unsent: false,
        has_attachments: false,
        thread_origin_guid: String::new(),
    };
    app.storage.lock().insert_message(&row)?;
    app.events.send(Event::MessageSent {
        chat_guid: chat_guid.to_string(),
        message_guid: guid.clone(),
    });

    let chat_g = chat_guid.to_string();
    let text_s = text.to_string();
    let tentative = guid.clone();
    let bus = app.events.clone();
    app.runtime_handle.spawn(async move {
        if let Err(e) = integration::send_imessage(&chat_g, &[], &text_s).await {
            bus.send(Event::MessageFailed {
                chat_guid: chat_g,
                tentative_guid: tentative,
                reason: e.to_string(),
            });
        }
    });

    Ok(guid)
}

fn mark_chat_read(app: &AppState, chat_guid: &str) -> Result<()> {
    app.storage.lock().mark_chat_read(chat_guid)?;
    app.events.send(Event::ChatUpdated {
        chat_guid: chat_guid.to_string(),
    });
    Ok(())
}

fn pin_chat(app: &AppState, chat_guid: &str, pinned: bool, order: i32) -> Result<()> {
    app.storage.lock().set_chat_pin(chat_guid, pinned, order)?;
    app.events.send(Event::ChatUpdated {
        chat_guid: chat_guid.to_string(),
    });
    Ok(())
}

fn archive_chat(app: &AppState, chat_guid: &str, archived: bool) -> Result<()> {
    app.storage.lock().set_chat_archived(chat_guid, archived)?;
    app.events.send(Event::ChatUpdated {
        chat_guid: chat_guid.to_string(),
    });
    Ok(())
}

fn tapback_local(app: &AppState, message_guid: &str, reaction: &str) -> Result<()> {
    app.storage
        .lock()
        .add_reaction(message_guid, None, true, reaction)?;
    let msg = message_guid.to_string();
    let rx = reaction.to_string();
    let bus = app.events.clone();
    app.runtime_handle.spawn(async move {
        if let Err(e) = integration::send_tapback(&msg, &rx).await {
            bus.send(Event::Warn(format!("tapback stub: {e}")));
        }
    });
    Ok(())
}

fn edit_message_local(app: &AppState, message_guid: &str, new_text: &str) -> Result<()> {
    app.storage.lock().edit_message(message_guid, new_text)?;
    let msg = message_guid.to_string();
    let txt = new_text.to_string();
    let bus = app.events.clone();
    app.runtime_handle.spawn(async move {
        if let Err(e) = integration::send_edit(&msg, &txt).await {
            bus.send(Event::Warn(format!("edit stub: {e}")));
        }
    });
    Ok(())
}

fn unsend_message_local(app: &AppState, message_guid: &str) -> Result<()> {
    app.storage.lock().unsend_message(message_guid)?;
    let msg = message_guid.to_string();
    let bus = app.events.clone();
    app.runtime_handle.spawn(async move {
        if let Err(e) = integration::send_unsend(&msg).await {
            bus.send(Event::Warn(format!("unsend stub: {e}")));
        }
    });
    Ok(())
}

fn start_apple_id_auth(app: &AppState, apple_id: &str, password: &str) -> Result<()> {
    if apple_id.is_empty() || password.is_empty() {
        return Err(anyhow!("apple_id and password are required"));
    }
    app.set_auth(AuthState::AuthenticatingAccount);
    let apple = apple_id.to_string();
    let pw = password.to_string();
    let bus = app.events.clone();
    let auth_slot = app.auth.clone();
    let storage = app.storage.clone();
    app.runtime_handle.spawn(async move {
        match integration::authenticate_apple_id(&apple, &pw).await {
            Ok(()) => {
                *auth_slot.lock() = AuthState::Ready;
                let label = AuthState::Ready.label();
                let _ = storage.lock().kv_set("auth.state", &label);
                bus.send(Event::AuthStateChanged { state: label });
            }
            Err(e) => {
                let msg = e.to_string();
                *auth_slot.lock() = AuthState::Errored(msg.clone());
                let label = AuthState::Errored(msg).label();
                let _ = storage.lock().kv_set("auth.state", &label);
                bus.send(Event::AuthStateChanged { state: label });
            }
        }
    });
    Ok(())
}

fn submit_two_factor_code(app: &AppState, code: &str) -> Result<()> {
    let c = code.to_string();
    let bus = app.events.clone();
    app.runtime_handle.spawn(async move {
        if let Err(e) = integration::submit_2fa_code(&c).await {
            bus.send(Event::Warn(format!("2fa stub: {e}")));
        }
    });
    Ok(())
}

fn complete_pairing(app: &AppState, code: &str, beeper_token: &str) -> Result<()> {
    if code.trim().is_empty() {
        return Err(anyhow!("pairing code is required"));
    }
    let host = app.relay_host();
    let code_s = code.trim().to_string();
    let token = if beeper_token.trim().is_empty() {
        None
    } else {
        Some(beeper_token.trim().to_string())
    };

    let config = crate::RUNTIME
        .block_on(crate::os_config::fetch_relay_config(&host, &code_s, token))?;
    app.persist_os_config(config)?;
    app.set_auth(AuthState::NeedsCredentials);
    app.events
        .send(Event::Info("paired with relay; enter Apple ID next".into()));
    Ok(())
}

fn clear_pairing(app: &AppState) -> Result<()> {
    app.clear_os_config()?;
    app.set_auth(AuthState::NeedsHardwarePairing);
    app.events.send(Event::Info("cleared relay pairing".into()));
    Ok(())
}

fn start_facetime_call(_app: &AppState, address: &str) -> Result<String> {
    let a = address.to_string();
    crate::RUNTIME.block_on(async { integration::start_facetime(&a).await })
}

fn poll_events(app: &AppState) -> Vec<bridge::EventDto> {
    app.events
        .drain()
        .into_iter()
        .map(event_to_dto)
        .collect()
}

fn event_to_dto(ev: Event) -> bridge::EventDto {
    match ev {
        Event::Info(t) => dto("info", t, "", "", false),
        Event::Warn(t) => dto("warn", t, "", "", false),
        Event::Error(t) => dto("error", t, "", "", false),
        Event::ChatCreated { chat_guid } => dto("chat_created", String::new(), &chat_guid, "", false),
        Event::ChatUpdated { chat_guid } => dto("chat_updated", String::new(), &chat_guid, "", false),
        Event::MessageArrived { chat_guid, message_guid } => {
            dto("message_arrived", String::new(), &chat_guid, &message_guid, false)
        }
        Event::MessageSent { chat_guid, message_guid } => {
            dto("message_sent", String::new(), &chat_guid, &message_guid, false)
        }
        Event::MessageFailed { chat_guid, tentative_guid, reason } => {
            dto("message_failed", reason, &chat_guid, &tentative_guid, false)
        }
        Event::AuthStateChanged { state } => dto("auth_state", state, "", "", false),
        Event::TypingStatusChanged { chat_guid, is_typing } => {
            dto("typing", String::new(), &chat_guid, "", is_typing)
        }
    }
}

fn dto(kind: &str, text: String, chat_guid: &str, message_guid: &str, flag: bool) -> bridge::EventDto {
    bridge::EventDto {
        kind: kind.to_string(),
        text,
        chat_guid: chat_guid.to_string(),
        message_guid: message_guid.to_string(),
        flag,
    }
}

fn seed_demo(app: &AppState) -> Result<()> {
    app.storage.lock().seed_demo_data()
}
