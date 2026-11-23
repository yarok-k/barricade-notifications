use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};

use anyhow::Result;
use zbus::{connection, interface, Connection, Variant};

const NAME: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";

#[derive(Debug, Clone)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub urgency: u8,
    pub timeout_ms: i32,
}

#[derive(Debug)]
pub enum Event {
    Notify(Notification),
    Close(u32),
}

struct Server {
    next_id: u32,
    tx: Sender<Event>,
}

#[interface(name = "org.freedesktop.Notifications")]
impl Server {
    fn get_capabilities(&self) -> Vec<&str> {
        vec!["body"]
    }

    fn get_server_information(&self) -> (&str, &str, &str, &str) {
        ("barricade", "bst", "0.1", "1.2")
    }

    #[allow(clippy::too_many_arguments)]
    fn notify(
        &mut self,
        app_name: String,
        replaces_id: u32,
        _app_icon: String,
        summary: String,
        body: String,
        _actions: Vec<String>,
        hints: HashMap<String, Variant<'_>>,
        expire_timeout: i32,
    ) -> u32 {
        let id = if replaces_id != 0 {
            replaces_id
        } else {
            let id = self.next_id;
            self.next_id += 1;
            id
        };
        let urgency = hints
            .get("urgency")
            .and_then(|v| u8::try_from(v).ok())
            .unwrap_or(1);

        let _ = self.tx.send(Event::Notify(Notification {
            id, app_name, summary, body, urgency, timeout_ms: expire_timeout,
        }));
        id
    }

    fn close_notification(&self, id: u32) {
        let _ = self.tx.send(Event::Close(id));
    }
}

/// Поднимает сервер. Connection нужно держать живым, иначе имя освободится.
pub fn start() -> Result<(Connection, Receiver<Event>)> {
    let (tx, rx) = mpsc::channel();
    let conn = zbus::block_on(async {
        connection::Builder::session()
            .name(NAME) // упадёт с NameTaken, если mako ещё жив
            .serve_at(PATH, Server { next_id: 1, tx })
            .build()
            .await
    })?;
    Ok((conn, rx))
}

/// reason: 1 истекло, 2 закрыто пользователем, 3 закрыто через CloseNotification
pub fn emit_closed(conn: &Connection, id: u32, reason: u32) -> Result<()> {
    zbus::block_on(conn.emit_signal(
        None::<&str>,
        PATH,
        NAME,
        "NotificationClosed",
        &(id, reason),
    ))?;
    Ok(())
}
