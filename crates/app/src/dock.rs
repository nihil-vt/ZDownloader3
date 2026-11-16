//! The JD2 bottom dock: two toggleable group boxes above the status strip — "Download
//! Overview" (the list at a glance) and "File Properties" (the selected download, JD2's
//! properties panel in a dialog's skin). Both follow the theme's light-dialog colours, as
//! Options and the Scheduler do.

use egui::{Pos2, Rect, Ui, pos2, vec2};

use crate::model::Item;
use crate::winui::{self, LIGHT_TEXT, MUTED};

/// What the dock's buttons send back; the app applies it (a missing panel must not be drawn).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    HideOverview,
    HideProperties,
}

/// The dock's height when shown; the panes above it lose this much.
pub const DOCK_H: f32 = 148.0;

/// The list at a glance (the JD2 Overview panel's information, computed from `items`).
#[derive(Default)]
struct Overview {
    total: usize,
    complete: usize,
    failed: usize,
    stopped: usize,
    running: usize,
    bytes_total: u64,
    bytes_done: u64,
    speed: f64,
    eta: Option<String>,
}

fn overview(items: &[Item]) -> Overview {
    let mut o = Overview {
        total: items.len(),
        ..Default::default()
    };
    for i in items {
        match i.status {
            crate::model::Status::Complete => o.complete += 1,
            crate::model::Status::Error => o.failed += 1,
            _ => {
                if i.live.is_some() {
                    o.running += 1;
                } else {
                    o.stopped += 1;
                }
            }
        }
        o.bytes_total = o.bytes_total.saturating_add(i.size.unwrap_or(0));
        o.bytes_done = o.bytes_done.saturating_add(i.downloaded);
        o.speed += i.rate();
    }
    let remaining = o.bytes_total.saturating_sub(o.bytes_done);
    if o.speed > 1.0 && remaining > 0 {
        let secs = remaining as f64 / o.speed;
        o.eta = Some(eta_text(secs));
    }
    o
}

fn eta_text(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "?".to_owned();
    }
    let s = secs.round() as u64;
    if s < 60 {
        return format!("{s} s");
    }
    let m = s / 60;
    if m < 60 {
        return format!("{m} min {} s", s % 60);
    }
    let h = m / 60;
    format!("{h} h {} min", m % 60)
}

/// What the dock shows; none of its drawing can fail (a zero-height dock is just not called).
pub fn dock(
    ui: &mut Ui,
    o: Pos2,
    w: f32,
    top: f32,
    bottom: f32,
    items: &[Item],
    selected: Option<&Item>,
    selected_count: usize,
    show_overview: bool,
    show_properties: bool,
) -> Option<Action> {
    let p = ui.painter().clone();
    let mut action = None;
    let pad = 6.0;
    let right = o.x + w - pad;
    let left = o.x + pad;
    // Split the room between the two panels (JD2's Overview is roomier).
    let (ov_left, ov_right, prop_left) = match (show_overview, show_properties) {
        (true, true) => {
            let mid = left + (right - left) * 0.44;
            (left, mid - 3.0, mid + 3.0)
        }
        (true, false) => (left, right, 0.0),
        (false, true) => (0.0, 0.0, left),
        (false, false) => return None,
    };
    if show_overview {
        let rc = Rect::from_min_max(pos2(ov_left, top), pos2(ov_right, bottom));
        panel(&p, rc, "Download Overview", Action::HideOverview, &mut action, ui);
        let ov = overview(items);
        // The grid: two label+value columns, in English point offsets.
        let rows: [(&str, String); 10] = [
            ("Downloads", crate::i18n::integer(ov.total as u64)),
            ("Running", crate::i18n::integer(ov.running as u64)),
            ("Complete", crate::i18n::integer(ov.complete as u64)),
            ("Failed", crate::i18n::integer(ov.failed as u64)),
            ("Stopped", crate::i18n::integer(ov.stopped as u64)),
            ("Total size", crate::dialogs::dialog_size(ov.bytes_total)),
            (
                "Bytes loaded",
                crate::dialogs::dialog_size(ov.bytes_done),
            ),
            (
                "Remaining",
                crate::dialogs::dialog_size(ov.bytes_total.saturating_sub(ov.bytes_done)),
            ),
            ("Speed", crate::model::rate(ov.speed)),
            ("ETA", ov.eta.clone().unwrap_or_else(|| "—".to_owned())),
        ];
        let col_w = (rc.width() - 8.0) / 2.0;
        for (i, (label, value)) in rows.iter().enumerate() {
            let col = i % 2;
            let row = i / 2;
            let x = rc.left() + 10.0 + col as f32 * col_w;
            let y = rc.top() + 26.0 + row as f32 * 21.0;
            winui::text(&p, x, y, label, MUTED());
            let wlabel = winui::text_width(&p, label) + 6.0;
            winui::text(&p, x + wlabel, y, value, LIGHT_TEXT());
        }
    }
    if show_properties {
        let rc = Rect::from_min_max(pos2(prop_left, top), pos2(right, bottom));
        panel(&p, rc, "File Properties", Action::HideProperties, &mut action, ui);
        let rows: Vec<(String, String)> = match selected {
            Some(item) => properties(item),
            None if selected_count > 1 => vec![(
                "Selected".to_owned(),
                crate::i18n::fmt(
                    crate::i18n::t("{n} downloads"),
                    &[("n", &crate::i18n::integer(selected_count as u64))],
                ),
            )],
            None => vec![("Selection".to_owned(), "—".to_owned())],
        };
        let x = rc.left() + 10.0;
        let mut y = rc.top() + 26.0;
        for (label, value) in rows.into_iter().take(5) {
            winui::text(&p, x, y, &label, MUTED());
            let wlabel = winui::text_width(&p, &label) + 6.0;
            winui::fit(
                ui,
                &p,
                x + wlabel,
                y,
                &value,
                LIGHT_TEXT(),
                rc.right() - 10.0,
                ("dock-prop", label.clone()),
            );
            y += 21.0;
        }
    }
    action
}

/// The panel frame + its caption in the top line + a close button at the right.
fn panel(
    p: &egui::Painter,
    rc: Rect,
    caption: &str,
    action: Action,
    out: &mut Option<Action>,
    ui: &mut Ui,
) {
    let caption_w = winui::text_width(p, caption) + 12.0;
    winui::group_box(
        p,
        winui::rect(rc),
        Some((rc.left() + 9.0, rc.left() + 9.0 + caption_w)),
    );
    winui::text(p, rc.left() + 15.0, rc.top() + 4.0, caption, LIGHT_TEXT());
    let x = rc.right() - 22.0;
    let close = ui.interact(
        Rect::from_min_size(pos2(x, rc.top() + 2.0), vec2(18.0, 15.0)),
        egui::Id::unique(("dock-close", caption)),
        egui::Sense::click(),
    );
    if close.clicked() {
        *out = Some(action);
    }
    let ink = if close.hovered() {
        LIGHT_TEXT()
    } else {
        MUTED()
    };
    winui::text(p, x + 5.0, rc.top() + 13.0, "×", ink);
}

fn properties(item: &Item) -> Vec<(String, String)> {
    let host = item
        .url
        .host_str()
        .map(|h| h.strip_prefix("www.").unwrap_or(h).to_owned())
        .unwrap_or_else(|| "—".to_owned());
    let size = match item.size {
        Some(s) => format!(
            "{} of {}",
            crate::dialogs::dialog_size(item.downloaded),
            crate::dialogs::dialog_size(s)
        ),
        None => crate::dialogs::dialog_size(item.downloaded),
    };
    let checksum = item
        .checksum
        .as_ref()
        .map(|h| format!("{:?} — {} bytes of digest", h.algo, h.value.len()))
        .unwrap_or_else(|| "—".to_owned());
    let package = item
        .package
        .as_ref()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "—".to_owned());
    vec![
        ("File name".to_owned(), item.file_name.clone()),
        ("From".to_owned(), host),
        ("Save to".to_owned(), item.dest.to_string_lossy().into_owned()),
        ("Size".to_owned(), size),
        ("Status".to_owned(), item.status_text()),
        ("Category".to_owned(), item.category.key().into_owned()),
        ("Added".to_owned(), item.added.clone().unwrap_or_else(|| "—".to_owned())),
        ("Checksum".to_owned(), checksum),
        ("Package".to_owned(), package),
    ]
}


// ---------------------------------------------------------------------------
// The link grabber's own dock (JD2's linkgrabber overview + package properties).

/// The link grabber's dock: applies the same two toggles (View → Download overview / File
/// properties) the downloads dock uses, showing the grabber's aggregates instead.
#[allow(clippy::too_many_arguments)]
pub fn grabber_dock(
    ui: &mut Ui,
    o: Pos2,
    w: f32,
    top: f32,
    bottom: f32,
    summary: &crate::grabber::GrabberSummary,
    current: Option<&crate::grabber::PackageSummary>,
    show_overview: bool,
    show_properties: bool,
) -> Option<Action> {
    let p = ui.painter().clone();
    let mut action = None;
    let (ov_left, ov_right, prop_left) = match (show_overview, show_properties) {
        (true, true) => {
            let mid = o.x + (w) * 0.44 + 6.0;
            (o.x + 6.0, mid - 3.0, mid + 3.0)
        }
        (true, false) => (o.x + 6.0, o.x + w - 7.0, 0.0),
        (false, true) => (0.0, 0.0, o.x + 6.0),
        (false, false) => return None,
    };
    if show_overview {
        let rc = Rect::from_min_max(pos2(ov_left, top), pos2(ov_right, bottom));
        panel(&p, rc, "Linkgrabber Overview", Action::HideOverview, &mut action, ui);
        let s = summary;
        let size = s.size.map(crate::dialogs::dialog_size).unwrap_or_else(|| "—".to_owned());
        let rows: [(&str, String); 8] = [
            ("Packages", crate::i18n::integer(s.packages as u64)),
            ("Links", crate::i18n::integer(s.links as u64)),
            ("Online", crate::i18n::integer(s.online as u64)),
            ("Failed", crate::i18n::integer(s.failed as u64)),
            ("Queued", crate::i18n::integer(s.queued as u64)),
            ("Ticked", crate::i18n::integer(s.ticked as u64)),
            ("Total size", size),
            ("Package", current.map(|c| c.name.clone()).unwrap_or_else(|| "—".to_owned())),
        ];
        let col_w = (rc.width() - 8.0) / 2.0;
        for (i, (label, value)) in rows.iter().enumerate() {
            let col = i % 2;
            let row = i / 2;
            let x = rc.left() + 10.0 + col as f32 * col_w;
            let y = rc.top() + 26.0 + row as f32 * 21.0;
            winui::text(&p, x, y, label, MUTED());
            let wlabel = winui::text_width(&p, label) + 6.0;
            winui::fit(
                ui,
                &p,
                x + wlabel,
                y,
                value,
                LIGHT_TEXT(),
                rc.right() - 10.0,
                ("grabberdock-ov", i),
            );
        }
    }
    if show_properties {
        let rc = Rect::from_min_max(pos2(prop_left, top), pos2(o.x + w - 7.0, bottom));
        panel(&p, rc, "Package Properties", Action::HideProperties, &mut action, ui);
        let rows: Vec<(String, String)> = match current {
            Some(c) => vec![
                ("Package".to_owned(), c.name.clone()),
                ("Save to".to_owned(), if c.folder.is_empty() { "—".to_owned() } else { c.folder.clone() }),
                ("Password".to_owned(), if c.password_set { "•••".to_owned() } else { "—".to_owned() }),
                ("Extract when done".to_owned(), if c.extract { "Yes".to_owned() } else { "No".to_owned() }),
                ("Delete archives".to_owned(), if c.delete_archives { "Yes".to_owned() } else { "No".to_owned() }),
                ("Links".to_owned(), format!("{} ({} ticked)", c.links, c.ticked)),
            ],
            None => vec![("Selection".to_owned(), "—".to_owned())],
        };
        let x = rc.left() + 10.0;
        let mut y = rc.top() + 26.0;
        for (label, value) in rows {
            winui::text(&p, x, y, &label, MUTED());
            let wlabel = winui::text_width(&p, &label) + 6.0;
            winui::fit(
                ui,
                &p,
                x + wlabel,
                y,
                &value,
                LIGHT_TEXT(),
                rc.right() - 10.0,
                ("grabberdock-prop", label),
            );
            y += 21.0;
        }
    }
    action
}
