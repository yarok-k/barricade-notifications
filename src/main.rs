use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use gtk::prelude::*;
use gtk::{CssProvider, StyleContext, Application, ApplicationWindow, glib, gdk};
use ui::Ui;
use lib::parcer::{modified_time, Parser};
use crate::lib::notification_server;

mod lib;
mod ui;
mod widgets;

struct StyleWatcher {
    provider: CssProvider,
    css_path: Option<PathBuf>,
    config_mtime: Option<SystemTime>,
    css_mtime: Option<SystemTime>,
}

impl StyleWatcher {
    fn new() -> Self {
        let provider = CssProvider::new();
        match gdk::Screen::default() {
            Some(screen) => StyleContext::add_provider_for_screen(
                &screen,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_USER,
            ),
            None => eprintln!("stylesheet disabled: no default screen"),
        }

        let mut watcher = Self {
            provider,
            css_path: None,
            config_mtime: None,
            css_mtime: None,
        };
        watcher.reload_config();
        watcher
    }

    fn reload_config(&mut self) {
        let parser = Parser::new();
        self.config_mtime = modified_time(parser.config_path());

        match parser.parse() {
            Ok(cfg) => self.css_path = cfg.stylesheet,
            Err(e) => eprintln!("config reload failed: {e:#}"),
        }
        self.reload_css();
    }

    fn reload_css(&mut self) {
        self.css_mtime = self.css_path.as_deref().and_then(modified_time);

        let result = match &self.css_path {
            Some(path) => self.provider.load_from_path(&path.to_string_lossy()),
            None => self.provider.load_from_data(b""),
        };
        if let Err(e) = result {
            eprintln!("stylesheet load failed: {e}");
        }
    }

    fn tick(&mut self) {
        let parser = Parser::new();
        if modified_time(parser.config_path()) != self.config_mtime {
            self.reload_config();
        } else if self.css_path.as_deref().and_then(modified_time) != self.css_mtime {
            self.reload_css();
        }
    }
}

#[derive(Clone)]
pub struct NotificationStore {
    pub store: Rc<RefCell<Vec<notification_server::Event>>>,
}

impl NotificationStore {
    pub fn new() -> Self {
        Self {
            store: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn add(&self, ev: notification_server::Event) {
        self.store.borrow_mut().push(ev);
    }
}

fn main() -> Result<(), anyhow::Error> {
    let note = NotificationStore::new();

    let app = Application::builder()
        .application_id("ru.bst.Barricade.notifications")
        .build();

    let note_startup = note.clone();
    app.connect_startup(move |_| {
        let watcher = Rc::new(RefCell::new(StyleWatcher::new()));
        glib::timeout_add_local(Duration::from_millis(500), move || {
            watcher.borrow_mut().tick();
            glib::ControlFlow::Continue
        });

        let note_rx = note_startup.clone();
        match notification_server::start() {
            Ok((conn, rx)) => {
                glib::timeout_add_local(Duration::from_millis(50), move || {
                    let _keep_alive = &conn;
                    while let Ok(ev) = rx.try_recv() {
                        note_rx.add(ev);
                    }
                    glib::ControlFlow::Continue
                });
            }
            Err(e) => eprintln!("notification server failed: {e:#}"),
        }
    });

    let note_activate = note.clone();
    app.connect_activate(move |app| {
        let display = gtk::gdk::Display::default().expect("Could not get default display");

        let monitor_count = display.n_monitors();
        for i in 0..monitor_count {
            if let Some(monitor) = display.monitor(i) {
                create_window_for_monitor(app, &monitor, note_activate.clone());
            }
        }

        let app_clone = app.clone();
        let note_hotplug = note_activate.clone();
        display.connect_monitor_added(move |_, monitor| {
            create_window_for_monitor(&app_clone, monitor, note_hotplug.clone());
        });
    });

    let exit_code = app.run();
    if exit_code.value() != 0 {}
    Ok(())
}

fn create_window_for_monitor(
    app: &Application,
    monitor: &gdk::Monitor,
    note: NotificationStore,
) {
    let win = ApplicationWindow::builder()
        .application(app)
        .decorated(false)
        .build();

    gtk_layer_shell::init_for_window(&win);
    gtk_layer_shell::set_monitor(&win, monitor);
    gtk_layer_shell::set_layer(&win, gtk_layer_shell::Layer::Overlay);
    gtk_layer_shell::set_anchor(&win, gtk_layer_shell::Edge::Top, true);
    let _ui = Ui::new(&win, app, note);

    win.show_all();
}
