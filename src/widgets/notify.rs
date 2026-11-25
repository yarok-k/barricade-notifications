use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use anyhow::Result;
use gtk::glib;
use gtk::prelude::*;

const REVEAL_MS: u32 = 220;
const REMOVE_DELAY_SECS: u64 = 10;

pub struct Notify {
    pub container: gtk::Box,
    pub id: u32,
    pub dismissed: Rc<Cell<bool>>,
}

impl Notify {
    pub fn new(
        name: String,
        summary: String,
        body: String,
        icon_name: Option<String>,
        id: u32,
    ) -> Result<Self> {
        let card = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        card.style_context().add_class("barricade_applet_background");
        println!("note sended");

        if let Some(ref icon) = icon_name {
            if !icon.is_empty() {
                let image = gtk::Image::from_icon_name(Some(icon), gtk::IconSize::LargeToolbar);
                card.pack_start(&image, false, false, 0);
            }
        }

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
        card.pack_start(&text_stack, true, true, 0);

        let event_box = gtk::EventBox::new();
        event_box.add(&card);

        // Анимация появления/скрытия самой плитки
        let revealer = gtk::Revealer::new();
        revealer.set_transition_type(gtk::RevealerTransitionType::SlideDown);
        revealer.set_transition_duration(REVEAL_MS);
        revealer.set_reveal_child(false);
        revealer.add(&event_box);

        let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
        outer.pack_start(&revealer, false, false, 0);

        let dismissed = Rc::new(Cell::new(false));
        let closing = Rc::new(Cell::new(false));

        let start_close = {
            let dismissed = dismissed.clone();
            let closing = closing.clone();
            let revealer = revealer.clone();
            move || {
                if dismissed.get() || closing.get() {
                    return;
                }
                dismissed.set(true);
                closing.set(true);
                revealer.set_reveal_child(false);
            }
        };

        {
            let closing = closing.clone();
            let outer_weak = outer.downgrade();
            revealer.connect_child_revealed_notify(move |rev| {
                if closing.get() && !rev.is_child_revealed() {
                    closing.set(false);
                    if let Some(box_widget) = outer_weak.upgrade() {
                        box_widget.hide();
                        if let Some(parent) = box_widget.parent() {
                            if let Ok(box_parent) = parent.downcast::<gtk::Box>() {
                                box_parent.remove(&box_widget);
                            }
                        }
                    }
                }
            });
        }

        {
            let start_close = start_close.clone();
            event_box.connect_button_press_event(move |_, _| {
                start_close();
                glib::Propagation::Stop
            });
        }

        {
            let start_close = start_close.clone();
            let dismissed_check = dismissed.clone();
            glib::timeout_add_local(Duration::from_secs(REMOVE_DELAY_SECS), move || {
                if dismissed_check.get() {
                    return glib::ControlFlow::Break;
                }
                start_close();
                glib::ControlFlow::Break
            });
        }

        outer.show_all();

        let rev_open = revealer.clone();
        glib::idle_add_local_once(move || {
            rev_open.set_reveal_child(true);
        });

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
