//! The main window's "Menu" toolbar button (#1097): every former menu-bar menu (Tasks, File,
//! Downloads, View, Help) in one panel with a search box at its top. Submenus render inline
//! under their parent; typing filters everything flat. Esc closes; Enter runs the highlighted
//! entry; a click runs any entry. The panel stays inside the main window (no separate window).

use egui::{Pos2, Rect, Ui, pos2, vec2};

use crate::chrome::MenuEntry;
use crate::winui;

const HEADER: f32 = 28.0;
const ITEM: f32 = 24.0;
pub const WIDTH: f32 = 340.0;
const SEARCH_H: f32 = 40.0;

/// The sections, grouped as they used to be (Tasks, File, Downloads, View, Help), with the
/// entries each section held.
pub struct Sections {
    pub groups: Vec<(String, Vec<MenuEntry<crate::app::Cmd>>)>,
    pub hint: fn(&crate::app::Cmd) -> Option<&'static str>,
}

pub enum Chosen {
    Command(crate::app::Cmd),
    Closed,
    Open,
}

/// The open panel's state: the filter, and the highlighted entry's position in the displayed
/// list.
#[derive(Clone, Default)]
pub struct ToolbarMenu {
    pub search: String,
    pub selection: usize,
}

enum Row<'a> {
    Header(usize),
    SubHeader {
        text: String,
    },
    Entry {
        depth: usize,
        index: usize,
        entry: &'a MenuEntry<crate::app::Cmd>,
        enabled: bool,
    },
}

fn flat_rows<'a>(sections: &'a Sections, search: &'a str) -> Vec<Row<'a>> {
    let searching = !search.trim().is_empty();
    let mut out: Vec<Row> = Vec::new();
    if !searching {
        for (gi, (_, entries)) in sections.groups.iter().enumerate() {
            if entries.is_empty() {
                continue;
            }
            out.push(Row::Header(gi));
            let mut index = 0usize;
            walk(&mut out, entries, 0, &mut index);
        }
    } else {
        let q = search.trim().to_lowercase();
        for (_, entries) in &sections.groups {
            for e in entries {
                collect_match(&mut out, e, vec![String::new()], &q, 0);
            }
        }
    }
    out
}

fn walk<'a>(
    rows: &mut Vec<Row<'a>>,
    entries: &'a [MenuEntry<crate::app::Cmd>],
    depth: usize,
    index: &mut usize,
) {
    for e in entries {
        if matches!(e, MenuEntry::Sep) {
            continue;
        }
        match e {
            MenuEntry::Sub(label, _, children) => {
                let name = crate::chrome::plain(crate::i18n::t(label)).0;
                rows.push(Row::SubHeader { text: name });
                for c in children {
                    if !matches!(c, MenuEntry::Sep) {
                        walk(rows, std::slice::from_ref(c), depth + 1, index);
                    }
                }
            }
            _ => {
                let parts = entry_parts(e);
                rows.push(Row::Entry {
                    depth,
                    index: *index,
                    entry: e,
                    enabled: parts.2,
                });
                *index += 1;
            }
        }
    }
}

fn collect_match<'a>(
    rows: &mut Vec<Row<'a>>,
    entry: &'a MenuEntry<crate::app::Cmd>,
    _prefix: Vec<String>,
    q: &str,
    index: usize,
) {
    if let MenuEntry::Sub(_, _, children) = entry {
        for c in children {
            collect_match(rows, c, Vec::new(), q, index);
        }
        return;
    }
    if matches!(entry, MenuEntry::Sep) {
        return;
    }
    let (_, label, _) = entry_parts(entry);
    let t = crate::i18n::t(label);
    let plain = crate::chrome::plain(t).0.to_lowercase();
    if plain.contains(q) {
        let parts = entry_parts(entry);
        rows.push(Row::Entry {
            depth: 0,
            index,
            entry,
            enabled: parts.2,
        });
    }
}

/// (label, translated label, enabled).
fn entry_parts<'a>(e: &'a MenuEntry<crate::app::Cmd>) -> (&'a str, &'a str, bool) {
    match e {
        MenuEntry::Item(l, _, en, _) | MenuEntry::Radio(l, _, en, _) | MenuEntry::Sub(l, en, _) => {
            (l, crate::i18n::t(l), *en)
        }
        MenuEntry::Sep => ("", "", true),
    }
}

pub fn show(ui: &mut Ui, at: Pos2, menu: &mut ToolbarMenu, sections: &Sections) -> (Rect, Chosen) {
    let p = ui.painter().clone();
    let search = menu.search.clone();
    let rows = flat_rows(sections, &search);
    let h = rows
        .iter()
        .map(|r| match r {
            Row::Header(_) => HEADER,
            Row::SubHeader { .. } | Row::Entry { .. } => ITEM,
        })
        .sum::<f32>()
        + SEARCH_H
        + 10.0;
    let rect = Rect::from_min_size(at, vec2(WIDTH, h.min(ui.available_height() + 40.0)));
    // The surface, as every popup's.
    let radius = crate::theme::metrics().flyout_radius;
    p.rect_filled(rect, radius, crate::chrome::POPUP_BG());
    p.rect_stroke(
        rect,
        radius,
        egui::Stroke::new(1.0, crate::chrome::POPUP_BORDER()),
        egui::StrokeKind::Inside,
    );
    // The search box.
    let search_rc = Rect::from_min_size(at + vec2(6.0, 6.0), vec2(WIDTH - 12.0, SEARCH_H - 12.0));
    crate::chrome::popup_surface(&p, search_rc.shrink(1.0));
    let shown = crate::i18n::t("Search...");
    let resp = ui.put(
        search_rc,
        egui::TextEdit::singleline(&mut menu.search)
            .desired_width(search_rc.width())
            .margin(egui::Margin::symmetric(8, 6))
            .font(crate::winui::tahoma())
            .frame(egui::Frame::NONE)
            .hint_text(shown),
    );
    // Focus the box on open so typing lands in it (the box keeps it afterwards).
    if !resp.has_focus() {
        ui.memory_mut(|m| m.request_focus(resp.id));
    }
    // Outside click closes (as Esc would).
    if ui.input(|i| i.pointer.any_pressed())
        && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
        && !rect.contains(pos)
    {
        *menu = ToolbarMenu::default();
        return (rect, Chosen::Closed);
    }
    // Keys.
    for ev in ui.input(|i| i.events.clone()) {
        if let egui::Event::Key {
            key, pressed: true, ..
        } = ev
        {
            match key {
                egui::Key::Escape => return (rect, close_panel(menu)),
                egui::Key::ArrowDown => menu.selection = move_select(&rows, menu.selection, 1),
                egui::Key::ArrowUp => menu.selection = move_select(&rows, menu.selection, -1),
                _ => {}
            }
        }
    }
    let mut chosen: Option<crate::app::Cmd> = None;
    let mut y = at.y + SEARCH_H;
    for row in &rows {
        match row {
            Row::Header(g) => {
                let (name, _) = &sections.groups[*g];
                winui::text(
                    &p,
                    at.x + 8.0,
                    y + 18.0,
                    &name.to_uppercase(),
                    winui::MUTED(),
                );
                y += HEADER;
            }
            Row::SubHeader { text } => {
                winui::text(
                    &p,
                    at.x + 10.0,
                    y + 15.0,
                    &text.to_uppercase(),
                    winui::MUTED(),
                );
                y += ITEM;
            }
            Row::Entry {
                depth,
                index,
                entry,
                enabled,
            } => {
                let is_selected = *index == menu.selection;
                let rc = Rect::from_min_size(pos2(at.x + 4.0, y), vec2(WIDTH - 8.0, ITEM));
                let hovered = ui.rect_contains_pointer(rc);
                if is_selected || hovered {
                    p.rect_filled(
                        rc,
                        crate::theme::metrics().part_radius,
                        crate::chrome::POPUP_HOVER(),
                    );
                }
                let (_, text, ..) = entry_parts(entry);
                winui::fit(
                    ui,
                    &p,
                    at.x + 12.0 + 12.0 * *depth as f32,
                    y + 15.0,
                    text,
                    if *enabled {
                        winui::TEXT()
                    } else {
                        winui::MUTED()
                    },
                    at.x + WIDTH - 12.0,
                    ("supermenu", y as u64),
                );
                if let Some(cmd) = cmd_of(entry) {
                    if let Some(hint) = (sections.hint)(cmd) {
                        let hk = crate::i18n::tc("menu", hint);
                        winui::text_right(&p, at.x + WIDTH - 12.0, y + 15.0, hk, winui::MUTED());
                    }
                }
                let resp = ui.interact(
                    rc,
                    egui::Id::unique(("supermenu-row", y as u64, *index)),
                    egui::Sense::click(),
                );
                if resp.clicked() {
                    if let Some(c) = cmd_of(entry) {
                        chosen = Some(c.clone());
                    }
                }
                y += ITEM;
            }
        }
    }
    if rows.iter().all(|r| matches!(r, Row::Entry { .. })) {
        // Only entries: nothing shown.
        let none = crate::i18n::t("No results");
        winui::text(
            &p,
            at.x + 8.0,
            at.y + SEARCH_H + 20.0,
            &none,
            winui::MUTED(),
        );
    }
    let _ = resp;
    match chosen {
        Some(c) => {
            *menu = ToolbarMenu::default();
            return (rect, Chosen::Command(c));
        }
        None => (rect, Chosen::Open),
    }
}

fn close_panel(menu: &mut ToolbarMenu) -> Chosen {
    // Reset the state and say the panel closed.
    *menu = ToolbarMenu::default();
    Chosen::Closed
}

fn cmd_of<'a>(e: &'a MenuEntry<crate::app::Cmd>) -> Option<&'a crate::app::Cmd> {
    match e {
        MenuEntry::Item(_, c, _, _) | MenuEntry::Radio(_, c, _, _) => Some(c),
        MenuEntry::Sub(_, _, _) | MenuEntry::Sep => None,
    }
}

fn move_select(rows: &[Row], from: usize, dir: i32) -> usize {
    let n = rows.len() as isize;
    let mut i = from as isize + dir as isize;
    while i >= 0 && i < n {
        if matches!(&rows[i as usize], Row::Entry { .. }) {
            return i as usize;
        }
        i += dir as isize;
    }
    from
}
