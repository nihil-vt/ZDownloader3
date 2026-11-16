mod theme;

use gtk4::prelude::*;
use libadwaita::{Application, ApplicationWindow, HeaderBar, StatusPage, TabView};
use std::cell::RefCell;
use std::rc::Rc;
use theme::Theme;

const APP_ID: &str = "io.github.zdownloader3.ZDownloader3";

fn main() -> gtk4::glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let provider: Rc<RefCell<Option<gtk4::CssProvider>>> = Rc::new(RefCell::new(None));
    theme::apply(Theme::AdwaitaDark, &mut provider.borrow_mut());

    // --- header bar: title + theme picker ---
    let header = HeaderBar::new();
    header.set_title_widget(Some(&libadwaita::WindowTitle::new("ZDownloader3", "native download manager")));

    let theme_names: Vec<&str> = Theme::ALL.iter().map(|(n, _)| *n).collect();
    let dropdown = gtk4::DropDown::from_strings(&theme_names);
    dropdown.set_selected(1); // Adwaita Dark default
    dropdown.set_tooltip_text(Some("Theme"));
    {
        let provider = Rc::clone(&provider);
        dropdown.connect_selected_notify(move |dd| {
            let idx = dd.selected() as usize;
            if let Some((_, t)) = Theme::ALL.get(idx) {
                theme::apply(*t, &mut provider.borrow_mut());
            }
        });
    }
    header.pack_end(&dropdown);

    // --- body: Linkgrabber / Downloads tabs (placeholders) ---
    let tabs = TabView::new();
    for title in ["Linkgrabber", "Downloads"] {
        let page = StatusPage::builder()
            .icon_name(if title == "Downloads" { "folder-download-symbolic" } else { "edit-paste-symbolic" })
            .title(title)
            .description("Not implemented yet — engine lands after analysis phase")
            .build();
        let tab = tabs.append(&page);
        tab.set_title(title);
    }

    // --- status bar: aria2 daemon probe ---
    let status = gtk4::Label::builder()
        .label("aria2: checking…")
        .halign(gtk4::Align::Start)
        .margin_start(8)
        .margin_end(8)
        .margin_top(2)
        .margin_bottom(2)
        .css_classes(["dim-label", "caption"])
        .build();
    {
        let status = status.clone();
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        std::thread::spawn(move || {
            let msg = match zd_aria2::Client::localhost(None).get_version() {
                Ok(v) => format!("aria2 {} ({})", v.version, v.enabled_features.join(", ")),
                Err(e) => format!("aria2: not reachable ({e})"),
            };
            let _ = tx.send(msg);
        });
        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            match rx.try_recv() {
                Ok(msg) => {
                    status.set_label(&msg);
                    gtk4::glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => gtk4::glib::ControlFlow::Continue,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => gtk4::glib::ControlFlow::Break,
            }
        });
    }

    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    content.append(&tabs);
    content.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
    content.append(&status);

    let toolbar_view = libadwaita::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&content));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("ZDownloader3")
        .default_width(1000)
        .default_height(640)
        .content(&toolbar_view)
        .build();
    window.present();
}
