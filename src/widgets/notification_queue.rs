use std::cell::Cell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use gtk::pango;
use gtk::prelude::*;
use gtk::{CssProvider, StyleContext};

use crate::lib::notifications_client::NotificationClient;
use crate::lib::widget::WidgetTrait;

pub struct PopupNotify {
    pub id: u32,
    pub app_name: String,
    pub text: String,
    pub container: gtk::Box,
    pub closed: Rc<Cell<bool>>,
}

impl PopupNotify {
    /// Создаёт карточку уведомления в попапе с крестиком в правом верхнем углу.
    /// При клике на крестик вызывает `on_close(id)` (например, `client.close(id)`).
    pub fn new<F>(id: u32, app_name: String, text: String, on_close: F) -> Result<Self>
    where
        F: Fn(u32) + 'static,
    {
        let card = gtk::Box::new(gtk::Orientation::Vertical, 6);
        card.style_context().add_class("barricade_popup_notify");

        // Верхняя строка: имя приложения слева + крестик закрытия справа
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let app_label = gtk::Label::builder()
            .label(&app_name)
            .xalign(0.0)
            .hexpand(true)
            .build();
        app_label.style_context().add_class("barricade_popup_notify_app");

        let close_btn = gtk::Button::with_label("×");
        close_btn.set_relief(gtk::ReliefStyle::None);
        close_btn.set_focus_on_click(false);
        close_btn.style_context().add_class("barricade_popup_notify_close");

        header.pack_start(&app_label, true, true, 0);
        header.pack_end(&close_btn, false, false, 0);

        // Текст уведомления с ограничением ширины и многоточием
        let body_label = gtk::Label::builder()
            .label(&text)
            .xalign(0.0)
            .wrap(true)
            .build();
        body_label.set_max_width_chars(30);
        body_label.set_ellipsize(pango::EllipsizeMode::End);
        body_label.style_context().add_class("barricade_popup_notify_body");

        card.pack_start(&header, false, false, 0);
        card.pack_start(&body_label, false, false, 0);

        let closed = Rc::new(Cell::new(false));
        let card_for_close = card.clone();
        let closed_flag = closed.clone();
        let on_close = Rc::new(on_close);

        close_btn.connect_clicked(move |_| {
            if closed_flag.get() {
                return;
            }
            closed_flag.set(true);
            on_close(id);

            card_for_close.hide();
            if let Some(parent) = card_for_close.parent() {
                if let Ok(box_parent) = parent.downcast::<gtk::Box>() {
                    box_parent.remove(&card_for_close);
                }
            }
        });

        card.show_all();

        Ok(Self {
            id,
            app_name,
            text,
            container: card,
            closed,
        })
    }

    pub fn is_closed(&self) -> bool {
        self.closed.get()
    }
}

pub struct NotificationManager {
    pub container: gtk::Box,
    items: Vec<PopupNotify>,
    seen_ids: HashSet<u32>,
    client: Arc<NotificationClient>,
}

impl NotificationManager {
    /// Добавляет уведомление в список попапа и привязывает закрытие через D-Bus (`close_notification`)
    pub fn push_notification(
        &mut self,
        id: u32,
        app_name: String,
        summary: String,
        body: String,
    ) -> Result<()> {
        if self.seen_ids.contains(&id) || self.items.iter().any(|n| n.id == id) {
            return Ok(());
        }

        let full_text = if body.is_empty() {
            summary
        } else if summary.is_empty() {
            body
        } else {
            format!("{summary}: {body}")
        };

        let client = Arc::clone(&self.client);
        let note = PopupNotify::new(id, app_name, full_text, move |notif_id| {
            let client = Arc::clone(&client);
            std::thread::spawn(move || {
                if let Err(err) = client.close(notif_id) {
                    eprintln!("Ошибка закрытия уведомления {notif_id}: {err}");
                }
            });
        })?;

        self.seen_ids.insert(id);
        self.container.pack_start(&note.container, false, false, 0);
        self.container.show_all();
        self.items.push(note);
        Ok(())
    }

    /// Убирает из списка карточки, которые уже закрыты (например, по внешнему событию Close)
    pub fn remove_by_id(&mut self, id: u32) {
        if let Some(pos) = self.items.iter().position(|n| n.id == id) {
            let removed = self.items.remove(pos);
            removed.closed.set(true);
            removed.container.hide();
            if let Some(parent) = removed.container.parent() {
                if let Ok(box_parent) = parent.downcast::<gtk::Box>() {
                    box_parent.remove(&removed.container);
                }
            }
        }
    }
}

impl WidgetTrait for NotificationManager {
    fn new() -> Result<Self> {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let client = Arc::new(NotificationClient::connect()?);

        let mut manager = Self {
            container,
            items: Vec::new(),
            seen_ids: HashSet::new(),
            client,
        };
        manager.update()?;
        Ok(manager)
    }

    fn update(&mut self) -> Result<()> {
        self.items.retain(|n| !n.is_closed());
        Ok(())
    }
}

/// Опциональная базовая стилизация крестика и карточки (можно положить в ваш CSS-файл)
pub fn install_default_popup_notify_css() {
    let provider = CssProvider::new();
    let css = b"
        .barricade_popup_notify {
            padding: 8px 10px;
            border-radius: 8px;
        }
        .barricade_popup_notify_app {
            font-weight: bold;
        }
        .barricade_popup_notify_close {
            padding: 0px 4px;
            min-width: 20px;
            min-height: 20px;
        }
    ";
    if let Ok(()) = provider.load_from_data(css) {
        if let Some(screen) = gtk::gdk::Screen::default() {
            StyleContext::add_provider_for_screen(
                &screen,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }
}
