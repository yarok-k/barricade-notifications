use std::collections::{HashMap, HashSet};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use crate::lib::notification_server::Event;
use crate::widgets::notify::Notify as NotifyWidget;
use crate::NotificationStore;

pub struct UiNotification {}

impl UiNotification {
    pub fn new(
        window: &gtk::ApplicationWindow,
        _app: &gtk::Application,
        notification_store: NotificationStore,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        window.set_size_request(300, 1);
        window.set_accept_focus(false);
        window.set_skip_taskbar_hint(true);
        window.set_skip_pager_hint(true);
        window.set_resizable(true);

        let main_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        window.add(&main_container);

        let tiles_container = gtk::Box::new(gtk::Orientation::Vertical, 6);
        main_container.pack_start(&tiles_container, false, false, 0);

        let mut spawned_ids: HashSet<u32> = HashSet::new();
        let mut active_tiles: HashMap<u32, NotifyWidget> = HashMap::new();

        let win = window.clone();
        glib::timeout_add_local(Duration::from_millis(500), move || {
            active_tiles.retain(|_, tile| !tile.is_dismissed());

            let events = notification_store.events.borrow();
            for event in events.iter() {
                match event {
                    Event::Notify(n) => {
                        if spawned_ids.contains(&n.id) {
                            continue;
                        }

                        let icon_name: Option<String> = None;

                        match NotifyWidget::new(
                            n.app_name.clone(),
                            n.summary.clone(),
                            n.body.clone(),
                            icon_name,
                            n.id,
                        ) {
                            Ok(tile) => {
                                tiles_container.pack_start(&tile.container, false, false, 0);
                                tiles_container.show_all();
                                win.show_all();
                                spawned_ids.insert(n.id);
                                active_tiles.insert(n.id, tile);
                            }
                            Err(e) => eprintln!("failed to create notify widget: {e:#}"),
                        }
                    }
                    Event::Close(id) => {
                        if let Some(tile) = active_tiles.remove(id) {
                            win.show_all();
                            tile.dismissed.set(true);
                            tile.container.hide();
                            tiles_container.remove(&tile.container);
                        }
                    }
                }
            }

            glib::ControlFlow::Continue
        });

        Ok(Self {})
    }
}
