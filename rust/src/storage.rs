use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

pub struct Storage {
    conn: Connection,
}

pub struct ChatRow {
    pub guid: String,
    pub title: String,
    pub is_imessage: bool,
    pub is_group: bool,
    pub is_archived: bool,
    pub is_pinned: bool,
    pub pin_order: i32,
    pub unread_count: i32,
    pub last_message_date: i64,
    pub last_message_preview: String,
    pub participant_summary: String,
}

pub struct MessageRow {
    pub guid: String,
    pub chat_guid: String,
    pub handle_id: Option<i64>,
    pub sender_address: String,
    pub sender_display_name: String,
    pub text: String,
    pub subject: String,
    pub is_from_me: bool,
    pub date: i64,
    pub date_read: i64,
    pub date_edited: i64,
    pub is_unsent: bool,
    pub has_attachments: bool,
    pub thread_origin_guid: String,
}

pub struct HandleRow {
    pub id: i64,
    pub address: String,
    pub service: String,
    pub display_name: String,
}

const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS chats (
    guid TEXT PRIMARY KEY,
    title TEXT NOT NULL DEFAULT '',
    is_imessage INTEGER NOT NULL DEFAULT 1,
    is_group INTEGER NOT NULL DEFAULT 0,
    is_archived INTEGER NOT NULL DEFAULT 0,
    is_pinned INTEGER NOT NULL DEFAULT 0,
    pin_order INTEGER NOT NULL DEFAULT 0,
    unread_count INTEGER NOT NULL DEFAULT 0,
    last_message_date INTEGER NOT NULL DEFAULT 0,
    last_message_preview TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS chats_by_last_message ON chats(last_message_date DESC);

CREATE TABLE IF NOT EXISTS handles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    address TEXT NOT NULL,
    service TEXT NOT NULL,
    display_name TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    UNIQUE(address, service)
);

CREATE TABLE IF NOT EXISTS chat_handles (
    chat_guid TEXT NOT NULL,
    handle_id INTEGER NOT NULL,
    PRIMARY KEY (chat_guid, handle_id),
    FOREIGN KEY (chat_guid) REFERENCES chats(guid) ON DELETE CASCADE,
    FOREIGN KEY (handle_id) REFERENCES handles(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS messages (
    guid TEXT PRIMARY KEY,
    chat_guid TEXT NOT NULL,
    handle_id INTEGER,
    text TEXT NOT NULL DEFAULT '',
    subject TEXT NOT NULL DEFAULT '',
    is_from_me INTEGER NOT NULL DEFAULT 0,
    date INTEGER NOT NULL,
    date_read INTEGER NOT NULL DEFAULT 0,
    date_edited INTEGER NOT NULL DEFAULT 0,
    is_unsent INTEGER NOT NULL DEFAULT 0,
    has_attachments INTEGER NOT NULL DEFAULT 0,
    thread_origin_guid TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (chat_guid) REFERENCES chats(guid) ON DELETE CASCADE,
    FOREIGN KEY (handle_id) REFERENCES handles(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS messages_by_chat_date ON messages(chat_guid, date DESC);

CREATE TABLE IF NOT EXISTS reactions (
    message_guid TEXT NOT NULL,
    handle_id INTEGER,
    is_from_me INTEGER NOT NULL DEFAULT 0,
    reaction_type TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    PRIMARY KEY (message_guid, handle_id, reaction_type),
    FOREIGN KEY (message_guid) REFERENCES messages(guid) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS attachments (
    guid TEXT PRIMARY KEY,
    message_guid TEXT NOT NULL,
    filename TEXT NOT NULL DEFAULT '',
    mime_type TEXT NOT NULL DEFAULT '',
    size_bytes INTEGER NOT NULL DEFAULT 0,
    local_path TEXT NOT NULL DEFAULT '',
    transfer_state INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (message_guid) REFERENCES messages(guid) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS kv (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("opening sqlite db at {}", path.display()))?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn kv_get(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM kv WHERE key = ?", params![key], |r| r.get::<_, String>(0))
            .optional()?)
    }

    pub fn kv_set(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO kv(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn upsert_handle(&self, address: &str, service: &str, display_name: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO handles(address, service, display_name, created_at) VALUES(?,?,?,?)\
             ON CONFLICT(address, service) DO UPDATE SET display_name=excluded.display_name",
            params![address, service, display_name, now_epoch()],
        )?;
        let id: i64 = self.conn.query_row(
            "SELECT id FROM handles WHERE address=? AND service=?",
            params![address, service],
            |r| r.get(0),
        )?;
        Ok(id)
    }

    pub fn list_handles(&self) -> Result<Vec<HandleRow>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, address, service, display_name FROM handles ORDER BY display_name, address")?;
        let rows = stmt
            .query_map([], |r| {
                Ok(HandleRow {
                    id: r.get(0)?,
                    address: r.get(1)?,
                    service: r.get(2)?,
                    display_name: r.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn upsert_chat(
        &self,
        guid: &str,
        title: &str,
        is_imessage: bool,
        is_group: bool,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO chats(guid, title, is_imessage, is_group, created_at) VALUES(?,?,?,?,?)\
             ON CONFLICT(guid) DO UPDATE SET title=excluded.title,\
             is_imessage=excluded.is_imessage, is_group=excluded.is_group",
            params![guid, title, is_imessage as i64, is_group as i64, now_epoch()],
        )?;
        Ok(())
    }

    pub fn set_chat_participants(&self, chat_guid: &str, handle_ids: &[i64]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM chat_handles WHERE chat_guid=?", params![chat_guid])?;
        for hid in handle_ids {
            tx.execute(
                "INSERT OR IGNORE INTO chat_handles(chat_guid, handle_id) VALUES(?,?)",
                params![chat_guid, hid],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn set_chat_pin(&self, chat_guid: &str, is_pinned: bool, order: i32) -> Result<()> {
        self.conn.execute(
            "UPDATE chats SET is_pinned=?, pin_order=? WHERE guid=?",
            params![is_pinned as i64, order, chat_guid],
        )?;
        Ok(())
    }

    pub fn set_chat_archived(&self, chat_guid: &str, is_archived: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE chats SET is_archived=? WHERE guid=?",
            params![is_archived as i64, chat_guid],
        )?;
        Ok(())
    }

    pub fn list_chats(&self, include_archived: bool) -> Result<Vec<ChatRow>> {
        let sql = if include_archived {
            "SELECT guid, title, is_imessage, is_group, is_archived, is_pinned, pin_order,\
             unread_count, last_message_date, last_message_preview\
             FROM chats\
             ORDER BY is_pinned DESC, pin_order ASC, last_message_date DESC"
        } else {
            "SELECT guid, title, is_imessage, is_group, is_archived, is_pinned, pin_order,\
             unread_count, last_message_date, last_message_preview\
             FROM chats WHERE is_archived = 0\
             ORDER BY is_pinned DESC, pin_order ASC, last_message_date DESC"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows: Vec<ChatRow> = stmt
            .query_map([], |r| {
                Ok(ChatRow {
                    guid: r.get(0)?,
                    title: r.get(1)?,
                    is_imessage: r.get::<_, i64>(2)? != 0,
                    is_group: r.get::<_, i64>(3)? != 0,
                    is_archived: r.get::<_, i64>(4)? != 0,
                    is_pinned: r.get::<_, i64>(5)? != 0,
                    pin_order: r.get(6)?,
                    unread_count: r.get(7)?,
                    last_message_date: r.get(8)?,
                    last_message_preview: r.get(9)?,
                    participant_summary: String::new(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn participants_summary(&self, chat_guid: &str) -> Result<String> {
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(NULLIF(h.display_name,''), h.address) FROM chat_handles ch\
             JOIN handles h ON h.id = ch.handle_id\
             WHERE ch.chat_guid = ? ORDER BY h.display_name LIMIT 6",
        )?;
        let parts: Vec<String> = stmt
            .query_map(params![chat_guid], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(parts.join(", "))
    }

    pub fn insert_message(&self, m: &MessageRow) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO messages(guid, chat_guid, handle_id, text, subject,\
             is_from_me, date, date_read, date_edited, is_unsent, has_attachments, thread_origin_guid)\
             VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                m.guid,
                m.chat_guid,
                m.handle_id,
                m.text,
                m.subject,
                m.is_from_me as i64,
                m.date,
                m.date_read,
                m.date_edited,
                m.is_unsent as i64,
                m.has_attachments as i64,
                m.thread_origin_guid,
            ],
        )?;
        self.conn.execute(
            "UPDATE chats SET last_message_date = MAX(last_message_date, ?),\
             last_message_preview = ?\
             WHERE guid = ?",
            params![m.date, m.text, m.chat_guid],
        )?;
        if !m.is_from_me {
            self.conn.execute(
                "UPDATE chats SET unread_count = unread_count + 1 WHERE guid = ?",
                params![m.chat_guid],
            )?;
        }
        Ok(())
    }

    pub fn mark_chat_read(&self, chat_guid: &str) -> Result<()> {
        let now = now_epoch();
        self.conn.execute(
            "UPDATE messages SET date_read = ?\
             WHERE chat_guid = ? AND is_from_me = 0 AND date_read = 0",
            params![now, chat_guid],
        )?;
        self.conn.execute(
            "UPDATE chats SET unread_count = 0 WHERE guid = ?",
            params![chat_guid],
        )?;
        Ok(())
    }

    pub fn edit_message(&self, guid: &str, new_text: &str) -> Result<()> {
        let now = now_epoch();
        self.conn.execute(
            "UPDATE messages SET text = ?, date_edited = ? WHERE guid = ?",
            params![new_text, now, guid],
        )?;
        Ok(())
    }

    pub fn unsend_message(&self, guid: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE messages SET is_unsent = 1, text = '' WHERE guid = ?",
            params![guid],
        )?;
        Ok(())
    }

    pub fn add_reaction(
        &self,
        message_guid: &str,
        handle_id: Option<i64>,
        is_from_me: bool,
        reaction: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO reactions(message_guid, handle_id, is_from_me, reaction_type, timestamp)\
             VALUES(?,?,?,?,?)",
            params![message_guid, handle_id, is_from_me as i64, reaction, now_epoch()],
        )?;
        Ok(())
    }

    pub fn messages_for_chat(&self, chat_guid: &str, limit: i64) -> Result<Vec<MessageRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT m.guid, m.chat_guid, m.handle_id,\
             COALESCE(h.address, '') as sender_address,\
             COALESCE(NULLIF(h.display_name,''), h.address, '') as sender_name,\
             m.text, m.subject, m.is_from_me, m.date, m.date_read, m.date_edited,\
             m.is_unsent, m.has_attachments, m.thread_origin_guid\
             FROM messages m LEFT JOIN handles h ON h.id = m.handle_id\
             WHERE m.chat_guid = ? ORDER BY m.date ASC LIMIT ?",
        )?;
        let rows: Vec<MessageRow> = stmt
            .query_map(params![chat_guid, limit], |r| {
                Ok(MessageRow {
                    guid: r.get(0)?,
                    chat_guid: r.get(1)?,
                    handle_id: r.get(2)?,
                    sender_address: r.get(3)?,
                    sender_display_name: r.get(4)?,
                    text: r.get(5)?,
                    subject: r.get(6)?,
                    is_from_me: r.get::<_, i64>(7)? != 0,
                    date: r.get(8)?,
                    date_read: r.get(9)?,
                    date_edited: r.get(10)?,
                    is_unsent: r.get::<_, i64>(11)? != 0,
                    has_attachments: r.get::<_, i64>(12)? != 0,
                    thread_origin_guid: r.get(13)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn seed_demo_data(&self) -> Result<()> {
        if self
            .conn
            .query_row::<i64, _, _>("SELECT COUNT(*) FROM chats", [], |r| r.get(0))
            .unwrap_or(0)
            > 0
        {
            return Ok(());
        }

        let now = now_epoch();
        let h_nyx = self.upsert_handle("+15555550123", "iMessage", "Nyx")?;
        let h_kay = self.upsert_handle("kay@example.com", "iMessage", "Kay")?;
        let h_rho = self.upsert_handle("+15555550456", "SMS", "Rho")?;
        let h_grp_a = self.upsert_handle("+15555550789", "iMessage", "Jules")?;
        let h_grp_b = self.upsert_handle("sam@example.com", "iMessage", "Sam")?;

        self.upsert_chat("chat-nyx", "Nyx", true, false)?;
        self.set_chat_participants("chat-nyx", &[h_nyx])?;
        self.set_chat_pin("chat-nyx", true, 0)?;

        self.upsert_chat("chat-kay", "Kay", true, false)?;
        self.set_chat_participants("chat-kay", &[h_kay])?;

        self.upsert_chat("chat-rho", "Rho", false, false)?;
        self.set_chat_participants("chat-rho", &[h_rho])?;

        self.upsert_chat("chat-group", "Friday roll", true, true)?;
        self.set_chat_participants("chat-group", &[h_grp_a, h_grp_b])?;

        let seed_msgs = [
            ("chat-nyx", Some(h_nyx), "hey you", false, now - 3600),
            ("chat-nyx", None,        "hey! sorry was heads-down", true,  now - 3500),
            ("chat-nyx", Some(h_nyx), "no worries — dinner still on?", false, now - 900),
            ("chat-kay", Some(h_kay), "sending the doc in a sec",     false, now - 7200),
            ("chat-kay", None,        "thanks 🙏", true,  now - 7100),
            ("chat-rho", Some(h_rho), "can you pick up milk",         false, now - 60),
            ("chat-group", Some(h_grp_a), "bar at 7?",                false, now - 300),
            ("chat-group", Some(h_grp_b), "i'm in",                   false, now - 250),
            ("chat-group", None,          "same",                      true,  now - 240),
        ];

        for (i, (cg, hid, txt, me, date)) in seed_msgs.iter().enumerate() {
            self.insert_message(&MessageRow {
                guid: format!("seed-msg-{i}"),
                chat_guid: (*cg).to_string(),
                handle_id: *hid,
                sender_address: String::new(),
                sender_display_name: String::new(),
                text: (*txt).to_string(),
                subject: String::new(),
                is_from_me: *me,
                date: *date,
                date_read: if *me { 0 } else { *date },
                date_edited: 0,
                is_unsent: false,
                has_attachments: false,
                thread_origin_guid: String::new(),
            })?;
        }

        Ok(())
    }
}
