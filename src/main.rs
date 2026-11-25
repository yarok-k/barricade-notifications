mod lib;
mod ui;
mod widgets;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use gtk::prelude::*;
use gtk::{gdk, glib, Application, ApplicationWindow, CssProvider, StyleContext};

use crate::lib::notification_server::{self, Event, Notification};
use lib::parcer::{modified_time, Parser};
use ui::{ui_calendar::UiCalendar,ui_notification::UiNotification};




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

#[derive(Clone, Default)]
pub struct NotificationStore {
    pub events: Rc<RefCell<Vec<Event>>>,
    pub items: Rc<RefCell<HashMap<u32, Notification>>>,
    pub popup_open_count: Rc<Cell<u64>>,
}

impl NotificationStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&self, ev: Event) {
        match &ev {
            Event::Notify(n) => {
                self.items.borrow_mut().insert(n.id, n.clone());
            }
            Event::Close(id) => {
                self.items.borrow_mut().remove(id);
            }
        }
        self.events.borrow_mut().push(ev);
    }

    pub fn notify_popup_opened(&self) {
        let next = self.popup_open_count.get().wrapping_add(1);
        self.popup_open_count.set(next);
    }
}
fn main() -> Result<(), anyhow::Error> {
    let note = NotificationStore::new();
    let popup_window: Rc<RefCell<Option<ApplicationWindow>>> = Rc::new(RefCell::new(None));
    let monitors_initialized = Rc::new(RefCell::new(false));

    let app = Application::builder()
        .application_id("ru.bst.Barricade.notifications")
        .flags(gtk::gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    let note_startup = note.clone();
    app.connect_startup(move |app| {
        // Не даём приложению завершиться при закрытии попапа или переподключении мониторов
        app.hold();

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

    let note_cmd = note.clone();
    let popup_cmd = popup_window.clone();
    let monitors_init_cmd = monitors_initialized.clone();
    app.connect_command_line(move |app, cmd| {
        // При первом запуске поднимаем окна всплываек для мониторов
        if !*monitors_init_cmd.borrow() {
            *monitors_init_cmd.borrow_mut() = true;
            init_notification_windows(app, note_cmd.clone());
        }

        let switch = match lib::flags::parse(cmd.arguments()) {
            Ok(s) => s,
            Err(e) => {
                if e.use_stderr() {
                    eprintln!("{e}");
                    return 1;
                }
                println!("{e}");
                return 0;
            }
        };

        if switch {
            toggle_calendar_popup(app, &popup_cmd, note_cmd.clone());
        }
        0
    });

    let exit_code = app.run();
    if exit_code.value() != 0 {}
    Ok(())
}

fn init_notification_windows(app: &Application, note: NotificationStore) {
    let display = gdk::Display::default().expect("Could not get default display");

    for i in 0..display.n_monitors() {
        if let Some(monitor) = display.monitor(i) {
            create_notification_window(app, &monitor, note.clone());
        }
    }

    let app_clone = app.clone();
    let note_hotplug = note.clone();
    display.connect_monitor_added(move |_, monitor| {
        create_notification_window(&app_clone, monitor, note_hotplug.clone());
    });
}

fn create_notification_window(
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

    let _ui = UiNotification::new(&win, app, note);
    win.show_all();
}

fn toggle_calendar_popup(
    app: &Application,
    popup_slot: &Rc<RefCell<Option<ApplicationWindow>>>,
    note: NotificationStore,
) {
    if let Some(win) = popup_slot.borrow().clone() {
        win.close();
        return;
    }

    let win = ApplicationWindow::builder()
        .application(app)
        .decorated(false)
        .build();

    win.set_app_paintable(true);
    if let Some(visual) = gdk::Screen::default().and_then(|s| s.rgba_visual()) {
        win.set_visual(Some(&visual));
    }

    gtk_layer_shell::init_for_window(&win);
    gtk_layer_shell::set_layer(&win, gtk_layer_shell::Layer::Overlay);
    gtk_layer_shell::set_anchor(&win, gtk_layer_shell::Edge::Top, true);
    gtk_layer_shell::set_margin(&win, gtk_layer_shell::Edge::Top, 10);
    gtk_layer_shell::set_keyboard_interactivity(&win, true);

    let slot_on_destroy = popup_slot.clone();
    win.connect_destroy(move |_| {
        *slot_on_destroy.borrow_mut() = None;
    });

    if let Err(e) = UiCalendar::new(&win, app, note) {
        eprintln!("failed to create calendar ui: {e:#}");
        return;
    }

    *popup_slot.borrow_mut() = Some(win);
}
