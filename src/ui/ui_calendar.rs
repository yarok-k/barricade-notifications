use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use gtk::prelude::*;
use gtk::{gdk, glib};

use crate::lib::notification_server::Event;
use crate::lib::widget::WidgetTrait;
use crate::widgets::calendar::Calendar as CalendarWidget;
use crate::widgets::notification_queue::NotificationManager;
use crate::NotificationStore;

const REVEAL_DURATION_MS: u32 = 250;

#[derive(Clone, Copy, PartialEq)]
enum CloseState {
    Open,
    Closing,
    Done,
}

pub struct UiCalendar {}

impl UiCalendar {
    pub fn new(
        window: &gtk::ApplicationWindow,
        _app: &gtk::Application,
        notification_store: NotificationStore,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        notification_store.notify_popup_opened();
        let main_container = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        main_container
            .style_context()
            .add_class("barricade_applet_background");
        main_container.set_hexpand(true);

        let calendar = CalendarWidget::new()?;
        let manager = Rc::new(RefCell::new(NotificationManager::new()?));

        main_container.pack_start(&calendar.container, false, false, 0);
        main_container.pack_start(&manager.borrow().container, true, true, 0);

        window.set_size_request(200, 200);

        let revealer = gtk::Revealer::new();
        revealer.set_transition_type(gtk::RevealerTransitionType::SlideDown);
        revealer.set_transition_duration(REVEAL_DURATION_MS);
        revealer.set_reveal_child(false);
        revealer.add(&main_container);
        window.add(&revealer);

        // Синхронизация списка уведомлений с NotificationStore
        let cursor = Rc::new(Cell::new(0usize));
        let sync_notifications = {
            let store = notification_store.clone();
            let manager = manager.clone();
            let cursor = cursor.clone();
            move || {
                // Сначала докидываем актуальные активные уведомления из store.items
                // (на случай если попап открыли впервые или переоткрыли)
                {
                    let items = store.items.borrow();
                    let mut mgr = manager.borrow_mut();
                    for (id, n) in items.iter() {
                        if let Err(e) = mgr.push_notification(
                            *id,
                            n.app_name.clone(),
                            n.summary.clone(),
                            n.body.clone(),
                        ) {
                            eprintln!("failed to push popup notification: {e:#}");
                        }
                    }
                }

                // Затем прогоняем новые события (Close / Notify) начиная с курсора
                let events = store.events.borrow();
                let start = cursor.get().min(events.len());
                let mut mgr = manager.borrow_mut();
                for ev in &events[start..] {
                    match ev {
                        Event::Notify(n) => {
                            if let Err(e) = mgr.push_notification(
                                n.id,
                                n.app_name.clone(),
                                n.summary.clone(),
                                n.body.clone(),
                            ) {
                                eprintln!("failed to push popup notification: {e:#}");
                            }
                        }
                        Event::Close(id) => {
                            mgr.remove_by_id(*id);
                        }
                    }
                }
                cursor.set(events.len());

                if let Err(e) = mgr.update() {
                    eprintln!("failed to update notification manager: {e:#}");
                }
            }
        };

        sync_notifications();

        let win_weak = window.downgrade();
        glib::timeout_add_local(Duration::from_millis(300), move || {
            if win_weak.upgrade().is_none() {
                return glib::ControlFlow::Break;
            }
            sync_notifications();
            glib::ControlFlow::Continue
        });

        // Закрытие с анимацией сворачивания
        let state = Rc::new(Cell::new(CloseState::Open));

        window.connect_delete_event({
            let revealer = revealer.clone();
            let state = state.clone();
            move |_, _| match state.get() {
                CloseState::Done => glib::Propagation::Proceed,
                CloseState::Closing => glib::Propagation::Stop,
                CloseState::Open => {
                    if !revealer.is_child_revealed() {
                        return glib::Propagation::Proceed;
                    }
                    state.set(CloseState::Closing);
                    revealer.set_reveal_child(false);
                    glib::Propagation::Stop
                }
            }
        });

        revealer.connect_child_revealed_notify({
            let win = window.clone();
            let state = state.clone();
            move |rev| {
                if state.get() == CloseState::Closing && !rev.is_child_revealed() {
                    state.set(CloseState::Done);
                    win.close();
                }
            }
        });

        // Esc -> закрыть только попап
        window.connect_key_press_event(|win, event| {
            if event.keyval() == gdk::keys::constants::Escape {
                win.close();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });

        // Потеря фокуса -> закрыть только попап
        window.connect_focus_out_event(|win, _| {
            win.close();
            glib::Propagation::Proceed
        });

        window.show_all();

        glib::idle_add_local_once(move || {
            revealer.set_reveal_child(true);
        });

        Ok(Self {})
    }
}
