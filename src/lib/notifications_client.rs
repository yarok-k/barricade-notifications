use std::collections::HashMap;

use anyhow::Result;
use zbus::{proxy, Connection, Variant};

#[proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, Variant<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;

    fn close_notification(&self, id: u32) -> zbus::Result<()>;
}

pub struct NotificationClient {
    conn: Connection,
}

impl NotificationClient {
    pub fn connect() -> Result<Self> {
        let conn = zbus::block_on(Connection::session())?;
        Ok(Self { conn })
    }

    pub fn send(&self, summary: &str, body: &str, urgency: u8, timeout_ms: i32) -> Result<u32> {
        zbus::block_on(async {
            let proxy = NotificationsProxy::new(&self.conn).await?;
            let mut hints: HashMap<&str, Variant<'_>> = HashMap::new();
            hints.insert("urgency", Variant::from(urgency));

            let id = proxy
                .notify(
                    "gtk-client",
                    0,
                    "",
                    summary,
                    body,
                    &[],
                    hints,
                    timeout_ms,
                )
                .await?;
            Ok(id)
        })
    }

    pub fn close(&self, id: u32) -> Result<()> {
        zbus::block_on(async {
            let proxy = NotificationsProxy::new(&self.conn).await?;
            proxy.close_notification(id).await?;
            Ok(())
        })
    }
}
