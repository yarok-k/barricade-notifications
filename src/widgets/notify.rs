use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use anyhow::Result;
use gtk::glib;
use gtk::prelude::*;

pub struct Notify {
    pub container: gtk::Box,
    pub id: u32,
    pub dismissed: Rc<Cell<bool>>, // плитка закрыта (кликом или таймером), но само уведомление сохраняется
}

impl Notify {
    pub fn new(
        name: String,
        summary: String,
        body: String,
        icon_name: Option<String>,
        id: u32,
    ) -> Result<Self> {
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        container.style_context().add_class("barricade_applet_background");
        println!("note sended");

        // Иконка уведомления (если передана)
        if let Some(ref icon) = icon_name {
            if !icon.is_empty() {
                let image = gtk::Image::from_icon_name(Some(icon), gtk::IconSize::LargeToolbar);
                container.pack_start(&image, false, false, 0);
            }
        }

        // Текстовый блок
        let text_stack = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let note_app_name = gtk::Label::builder().label(&name).xalign(0.0).build();
        let note_summary = gtk::Label::builder()
            .label(&summary)
            .xalign(0.0)
            .wrap(true)
            .build();
        let note_body = gtk::Label::builder()
            .label(&body)
            .xalign(0.0)
            .wrap(true)
            .build();

        text_stack.pack_start(&note_app_name, false, false, 0);
        text_stack.pack_start(&note_summary, false, false, 0);
        text_stack.pack_start(&note_body, false, false, 0);
        container.pack_start(&text_stack, true, true, 0);

        let dismissed = Rc::new(Cell::new(false));

        // Оборачиваем в EventBox, чтобы ловить клик по всей плитке
        let event_box = gtk::EventBox::new();
        event_box.add(&container);

        // Клик по плитке убирает только саму плитку из UI
        let dismissed_on_click = dismissed.clone();
        let event_box_click = event_box.clone();
        event_box.connect_button_press_event(move |_, _| {
            if !dismissed_on_click.get() {
                dismissed_on_click.set(true);
                event_box_click.hide();
                // Если нужно удалить виджет из родительского контейнера сразу:
                if let Some(parent) = event_box_click.parent() {
                    if let Ok(box_parent) = parent.downcast::<gtk::Box>() {
                        box_parent.remove(&event_box_click);
                    }
                }
            }
            glib::Propagation::Stop
        });

        const REMOVE_DELAY: u64 = 10;
        let dismissed_on_timer = dismissed.clone();
        let event_box_timer = event_box.clone();
        glib::timeout_add_local(Duration::from_secs(REMOVE_DELAY), move || {
            if dismissed_on_timer.get() {
                return glib::ControlFlow::Break;
            }
            dismissed_on_timer.set(true);
            event_box_timer.hide();
            if let Some(parent) = event_box_timer.parent() {
                if let Ok(box_parent) = parent.downcast::<gtk::Box>() {
                    box_parent.remove(&event_box_timer);
                }
            }
            glib::ControlFlow::Break
        });

        // Кладём event_box внутрь pub container: gtk::Box, чтобы внешний код
        // мог работать с `notify.container` как и раньше
        let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
        outer.pack_start(&event_box, false, false, 0);

        container.show_all();

        Ok(Self {
            container: outer,
            id,
            dismissed,
        })


    }

    pub fn is_dismissed(&self) -> bool {
        self.dismissed.get()
    }
}
