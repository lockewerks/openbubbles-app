use crossbeam_channel::{unbounded, Receiver, Sender};

#[derive(Clone, Debug)]
pub enum Event {
    Info(String),
    Warn(String),
    Error(String),
    ChatCreated { chat_guid: String },
    ChatUpdated { chat_guid: String },
    MessageArrived { chat_guid: String, message_guid: String },
    MessageSent { chat_guid: String, message_guid: String },
    MessageFailed { chat_guid: String, tentative_guid: String, reason: String },
    AuthStateChanged { state: String },
    TypingStatusChanged { chat_guid: String, is_typing: bool },
}

#[derive(Clone)]
pub struct EventBus {
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        Self { tx, rx }
    }

    pub fn send(&self, ev: Event) {
        let _ = self.tx.send(ev);
    }

    pub fn drain(&self) -> Vec<Event> {
        let mut out = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            out.push(ev);
        }
        out
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
