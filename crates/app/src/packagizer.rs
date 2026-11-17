//! Packagizer (#37): JDownloader2's linkgrabber rule engine. Rules run in order, in the
//! link grabber, when links are added (and again on demand: "Apply rules"). A rule whose
//! conditions all hold changes its link: renamed, moved to a package, or carried into the
//! package's destination folder — or stops the chain for that link ("stop after this rule").
//!
//! Variables in the payload patterns: `<filename> <name> <ext> <packagename> <index>
//! <simpledate:FORMAT>` — `<name>` is the name without its extension, `<packagename>` the
//! package the link is in (as it is *before* this rule runs), `<index>` its 1-based index
//! within that package, `<simpledate:yyyy-MM-dd>` today's date in the given format.

use egui::{Pos2, Rect, pos2};
use serde::{Deserialize, Serialize};

/// What a matching rule does to its link.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// The link moves into a package with this name (variables allowed); the package is made
    /// when it doesn't exist, as a site's package is.
    #[default]
    MoveToPackage,
    /// The file is renamed (variables allowed); the destination folder doesn't change.
    RenameFile,
    /// The link's package's destination becomes this path (variables allowed; relative paths
    /// join the download folder).
    SetDestination,
    /// The chain stops for this link (nothing else is done to it) — JD2's "stop after this
    /// rule".
    StopAfterRule,
}

/// One rule; the chain runs them in the list's order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rule {
    /// Stable, for the editor and for errors that name the rule.
    pub id: u64,
    pub enabled: bool,
    pub name: String,
    /// What the address must match (regex, left empty for any).
    pub url: String,
    /// What the name must match (regex, left empty for any).
    pub filename: String,
    /// The extensions it may be ("rar 7z zip", comma or space separated; empty for any).
    pub exts: String,
    /// The size bounds in bytes; 0 = unbounded.
    pub min_size: u64,
    pub max_size: u64,
    pub action: Action,
    /// The payload: the package's name, the new file name, or the destination path.
    pub payload: String,
}

impl Default for Rule {
    fn default() -> Self {
        Rule {
            id: 0,
            enabled: true,
            name: String::new(),
            url: String::new(),
            filename: String::new(),
            exts: String::new(),
            min_size: 0,
            max_size: 0,
            action: Action::MoveToPackage,
            payload: String::new(),
        }
    }
}

/// What the chain decided for one link. Applied by the grabber (renaming changes the link,
/// moving changes its package, destination changes the package's folder).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Change {
    pub name: Option<String>,
    pub package: Option<String>,
    pub destination: Option<String>,
    pub stopped: bool,
}

/// A rule's conditions on one link: URL, name, extension family, size. Compiles each regex
/// once per call; a bad regex never matches (the rule is inert, the GUI shows it).
fn matches(rule: &Rule, url: &str, name: &str, size: Option<u64>) -> bool {
    if !rule.url.is_empty()
        && !regex::RegexBuilder::new(&rule.url)
            .case_insensitive(true)
            .build()
            .is_ok_and(|r| r.is_match(url))
    {
        return false;
    }
    if !rule.filename.is_empty()
        && !regex::RegexBuilder::new(&rule.filename)
            .case_insensitive(true)
            .build()
            .is_ok_and(|r| r.is_match(name))
    {
        return false;
    }
    if !rule.exts.is_empty() {
        let exts: Vec<String> = rule
            .exts
            .split([',', ' ', ';', '\t'])
            .map(|e| {
                e.trim()
                    .to_ascii_lowercase()
                    .trim_start_matches('.')
                    .to_owned()
            })
            .filter(|e| !e.is_empty())
            .collect();
        let ext = name
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .unwrap_or_default();
        if !exts.iter().any(|e| e == &ext) {
            return false;
        }
    }
    match size {
        Some(size) => {
            if rule.min_size > 0 && size < rule.min_size {
                return false;
            }
            if rule.max_size > 0 && size > rule.max_size {
                return false;
            }
        }
        None => {
            // An unknown size: a bound either way refuses (nothing to check the bound against).
            if rule.min_size > 0 || rule.max_size > 0 {
                return false;
            }
        }
    }
    true
}

/// The variables' values for one link; `simpledate` in `<simpledate:...>` takes the format in
/// the braces (`yyyy-MM-dd`, only the parts, in English letters).
#[derive(Clone)]
pub struct Vars<'a> {
    pub filename: &'a str,
    pub name: &'a str,
    pub ext: &'a str,
    pub packagename: &'a str,
    pub index: usize,
    pub simpledate: String,
    /// Capture groups from the filename regex (or the URL regex when the filename has none);
    /// `$1`-`$9`/`$10`... and `${name}` substitute from these.
    pub captures: std::collections::HashMap<usize, &'a str>,
    pub captures_named: std::collections::HashMap<String, &'a str>,
}

/// Fill `v`'s capture groups from the filename regex when it has groups (JD2's variables
/// `{1}`..`{n}`, reached here as `$1`..`$n`), else from the URL regex. Empty patterns and
/// regexes that fail to compile give no captures; a rule that matched already compiles.
fn fill_captures<'a>(rule: &Rule, url: &'a str, name: &'a str, v: &mut Vars<'a>) {
    let captures_of = |text: &'a str,
                       pattern: &str|
     -> Option<(regex::Captures<'a>, Vec<Option<String>>)> {
        if pattern.is_empty() {
            return None;
        }
        let re = regex::RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .ok()?;
        let names: Vec<Option<String>> = re.capture_names().map(|n| n.map(str::to_owned)).collect();
        let caps = re.captures(text)?;
        (caps.len() > 1).then_some((caps, names))
    };
    let Some((caps, names)) =
        captures_of(name, &rule.filename).or_else(|| captures_of(url, &rule.url))
    else {
        return;
    };
    for i in 1..caps.len() {
        if let Some(m) = caps.get(i) {
            v.captures.insert(i, m.as_str());
        }
    }
    for (_i, cap) in names.into_iter().enumerate().skip(1) {
        if let Some(nm) = cap
            && !nm.is_empty()
            && let Some(m) = caps.name(&nm)
        {
            v.captures_named.insert(nm, m.as_str());
        }
    }
}

/// The variables' values for one link (`now` as unix seconds).
pub fn vars<'a>(name: &'a str, packagename: &'a str, index: usize, now: i64) -> Vars<'a> {
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i + 1..]),
        _ => (name, ""),
    };
    Vars {
        filename: name,
        name: stem,
        ext,
        packagename,
        index,
        simpledate: chrono_now(now),
        captures: std::collections::HashMap::new(),
        captures_named: std::collections::HashMap::new(),
    }
}

/// `yyyy-MM-dd HH:mm:ss` in UTC, the format subset the patterns take.
fn chrono_now(now: i64) -> String {
    use std::time::SystemTime;
    let _ = SystemTime::now(); // (now comes from the caller)
    // Civil date from unix days (howard hinnant's algorithm).
    let mut z = now / 86_400 + 719_468;
    let era = z.div_euclid(146_097);
    z = z.rem_euclid(146_097);
    let yoe = (z - z / 1460 + z / 36_524 - z / 146_096) / 365;
    let y = yoe + era * 100 * 4;
    let doy = z - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    let secs = now.rem_euclid(86_400);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// Replace `<filename>`, `<name>`, `<ext>`, `<packagename>`, `<index>` and
/// `<simpledate:FORMAT>` (FORMAT: `yyyy MM dd HH mm ss`, e.g. `yyyy-MM-dd`) in `pattern`.
pub fn subst(pattern: &str, v: &Vars) -> String {
    let mut out = String::with_capacity(pattern.len() + 16);
    let bytes = pattern.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            // ${name} or $N (one or more digits): capture-group substitutions
            if bytes.get(i + 1) == Some(&b'{') {
                if let Some(end) = pattern[i + 2..].find('}').map(|e| i + 2 + e) {
                    if let Some(val) = v.captures_named.get(&pattern[i + 2..end]) {
                        out.push_str(val);
                    } else {
                        out.push_str(&pattern[i..=end]);
                    }
                    i = end + 1;
                    continue;
                }
            } else {
                let mut j = i + 1;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j > i + 1
                    && let Ok(n) = pattern[i + 1..j].parse::<usize>()
                {
                    if let Some(val) = v.captures.get(&n) {
                        out.push_str(val);
                    } else {
                        out.push_str(&pattern[i..j]);
                    }
                    i = j;
                    continue;
                }
            }
            out.push('$');
            i += 1;
            continue;
        }
        if bytes[i] != b'<' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        let Some(end) = pattern[i..].find('>').map(|e| i + e) else {
            out.push_str(&pattern[i..]);
            break;
        };
        let token = &pattern[i + 1..end];
        match token {
            "filename" => out.push_str(v.filename),
            "name" => out.push_str(v.name),
            "ext" => out.push_str(v.ext),
            "packagename" => out.push_str(v.packagename),
            "index" => out.push_str(&crate::i18n::integer(v.index as u64)),
            t if t.starts_with("simpledate:") => {
                let fmt = &t["simpledate:".len()..];
                out.push_str(&format_simpledate(fmt, &v.simpledate));
            }
            // Unknown variable: kept as written (the editor shows what it didn't lose).
            _ => {
                out.push('<');
                out.push_str(token);
                out.push('>');
            }
        }
        i = end + 1;
    }
    out
}

fn format_simpledate(fmt: &str, now_ymd_hms: &str) -> String {
    let parts: Vec<&str> = now_ymd_hms.split([' ', '-', ':']).collect();
    if parts.len() != 6 {
        return now_ymd_hms.to_owned();
    }
    let (y, mo, d, h, mi, s) = (parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]);
    fmt.replace("yyyy", y)
        .replace("MM", mo)
        .replace("dd", d)
        .replace("HH", h)
        .replace("mm", mi)
        .replace("ss", s)
}

/// A name a rule may make: no path separators or drive letters; what Windows/macOS/Linux all
/// refuse. Empty → None (an invalid rename never blanks a name).
pub fn sanitize_name(s: &str) -> Option<String> {
    let s = s
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c => c,
        })
        .collect::<String>();
    let s = s.trim().trim_matches('.').to_owned();
    // All dashes/dots/spaces is not a name (a payload of `///` made it through otherwise).
    (!s.is_empty() && s.chars().any(|c| c.is_alphanumeric())).then_some(s)
}

/// The rule chain, applied to one link. `index` is its 1-based index in its package (before
/// this chain runs); `now` for `<simpledate:...>`.
pub fn apply(
    rules: &[Rule],
    url: &str,
    name: &str,
    size: Option<u64>,
    package: &str,
    index: usize,
    now: i64,
) -> Change {
    let mut out = Change::default();
    let mut current_name: String = name.to_owned();
    let mut current_package: String = package.to_owned();
    for rule in rules {
        if !rule.enabled || !matches(rule, url, &current_name, size) {
            continue;
        }
        let mut v = vars(&current_name, &current_package, index, now);
        fill_captures(rule, url, &current_name, &mut v);
        match rule.action {
            Action::MoveToPackage => {
                if let Some(p) = sanitize_name(&subst(&rule.payload, &v)) {
                    out.package = Some(p.clone());
                    current_package = p;
                }
            }
            Action::RenameFile => {
                if let Some(n) = sanitize_name(&subst(&rule.payload, &v)) {
                    out.name = Some(n.clone());
                    current_name = n;
                }
            }
            Action::SetDestination => {
                // The destination pattern is a path: variables allowed, separators kept,
                // only `..` refused (a rule writes nowhere outside the download folder).
                let joined = subst(&rule.payload, &v);
                if !joined.split(['/', '\\']).any(|c| c == "..") {
                    out.destination = Some(joined);
                }
            }
            Action::StopAfterRule => {
                out.stopped = true;
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(action: Action, payload: &str) -> Rule {
        Rule {
            id: 1,
            enabled: true,
            name: "t".into(),
            url: String::new(),
            filename: String::new(),
            exts: String::new(),
            min_size: 0,
            max_size: 0,
            action,
            payload: payload.into(),
        }
    }

    #[test]
    fn variables_substitute() {
        let v = vars("Book.Name.2026.epub", "Reading", 3, 0);
        assert_eq!(subst("<name> (<index>)", &v), "Book.Name.2026 (3)");
        assert_eq!(subst("<filename>", &v), "Book.Name.2026.epub");
        assert_eq!(subst("<ext>", &v), "epub");
        assert_eq!(subst("<packagename>", &v), "Reading");
        assert_eq!(subst("<simpledate:yyyy-MM-dd>", &v), "1970-01-01");
        assert_eq!(subst("<unknown>", &v), "<unknown>");
    }

    #[test]
    fn bad_regex_is_inert_not_a_crash() {
        let mut r = rule(Action::RenameFile, "x");
        r.url = "([unclosed".into();
        let c = apply(&[r], "https://a/b", "f", None, "p", 1, 0);
        assert_eq!(c, Change::default());
    }

    #[test]
    fn stop_after_rule_ends_the_chain() {
        let chain = [
            rule(Action::StopAfterRule, ""),
            rule(Action::MoveToPackage, "nope"),
        ];
        let c = apply(&chain, "https://a", "f", None, "p", 1, 0);
        assert!(c.stopped);
        assert_eq!(c.package, None);
    }

    #[test]
    fn later_rule_sees_earlier_rename() {
        let chain = [
            rule(Action::RenameFile, "<name>.renamed"),
            rule(Action::MoveToPackage, "<filename>"),
        ];
        let c = apply(&chain, "https://a", "book.pdf", None, "p", 1, 0);
        assert_eq!(c.name.as_deref(), Some("book.renamed"));
        assert_eq!(c.package.as_deref(), Some("book.renamed"));
    }

    #[test]
    fn extension_filter_matches() {
        let mut r = rule(Action::RenameFile, "x.pdf");
        r.exts = "pdf, epub".into();
        assert_eq!(
            apply(
                std::slice::from_ref(&r),
                "https://a",
                "a.PDF",
                None,
                "p",
                1,
                0
            )
            .name
            .as_deref(),
            Some("x.pdf")
        );
        assert_eq!(
            apply(
                std::slice::from_ref(&r),
                "https://a",
                "a.epub",
                None,
                "p",
                1,
                0
            )
            .name
            .as_deref(),
            Some("x.pdf")
        );
        assert_eq!(
            apply(
                std::slice::from_ref(&r),
                "https://a",
                "a.mp4",
                None,
                "p",
                1,
                0
            )
            .name,
            None
        );
    }

    #[test]
    fn sizes_bound() {
        let mut r = rule(Action::MoveToPackage, "big");
        r.min_size = 1_000;
        let one = std::slice::from_ref(&r);
        assert_eq!(
            apply(one, "https://a", "f", Some(500), "p", 1, 0).package,
            None
        );
        assert_eq!(
            apply(one, "https://a", "f", Some(2_000), "p", 1, 0)
                .package
                .as_deref(),
            Some("big")
        );
        assert_eq!(apply(one, "https://a", "f", None, "p", 1, 0).package, None);
    }

    #[test]
    fn invalid_rename_is_dropped() {
        let r = rule(Action::RenameFile, "///");
        assert_eq!(apply(&[r], "https://a", "f", None, "p", 1, 0).name, None);
        let r = rule(Action::RenameFile, "");
        assert_eq!(apply(&[r], "https://a", "f", None, "p", 1, 0).name, None);
    }
}

// ---------------------------------------------------------------------------
// The Packagizer window (Options → Packagizer): the rule chain, edited.

use crate::dialogs::{Outcome, window};
use crate::winui::{self, ButtonKind, LIGHT_BG, LIGHT_TEXT, r, text};

const DIALOG_W: f32 = 780.0;
const DIALOG_H: f32 = 480.0;
/// The list column's width.
const LIST_W: f32 = 300.0;
/// The row height in the rules list.
const ROW_H: f32 = 20.0;

/// What OK was pressed with.
pub struct Changes {
    pub rules: Vec<Rule>,
    /// The user pressed "Apply to list" and OK: the app re-runs the chain on the link
    /// grabber's staged links.
    pub apply: bool,
}

pub struct Packagizer {
    pub rules: Vec<Rule>,
    next_id: u64,
    selected: Option<usize>,
    status: Option<&'static str>,
}

impl Packagizer {
    pub fn new(rules: Vec<Rule>) -> Self {
        let next_id = rules.iter().map(|r| r.id).max().unwrap_or(0) + 1;
        let selected = (!rules.is_empty()).then_some(0);
        Packagizer {
            rules,
            next_id,
            selected,
            status: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Outcome<Changes> {
        let out = window(
            ctx,
            "packagizer",
            "Packagizer",
            [DIALOG_W, DIALOG_H],
            false,
            LIGHT_BG(),
            |ui, o| self.body(ui, o),
        );
        match out {
            Outcome::Open => Outcome::Open,
            Outcome::Closed => Outcome::Closed,
            Outcome::Done(apply) => Outcome::Done(Changes {
                rules: self.rules.clone(),
                apply,
            }),
        }
    }

    fn body(&mut self, ui: &mut egui::Ui, o: Pos2) -> Outcome<bool> {
        let p = ui.painter().clone();
        let top = o.y + 12.0;
        text(
            &p,
            o.x + 11.0,
            top + 12.0,
            "&Rule chain — first match wins, in this order:",
            LIGHT_TEXT(),
        );
        // --- the rules list ---
        let list = egui::Rect::from_min_max(
            pos2(o.x + 11.0, top + 20.0),
            pos2(o.x + LIST_W, o.y + DIALOG_H - 58.0),
        );
        let inner = list.shrink(2.0);
        p.rect_filled(winui::rect(inner), 0.0, winui::FIELD());
        let _view = inner.size();
        let sel = self.selected;
        let rules_snapshot: Vec<Rule> = self.rules.clone();
        for (k, rule) in rules_snapshot.iter().enumerate() {
            let y = inner.top() + k as f32 * ROW_H;
            if y + ROW_H > inner.bottom() {
                break;
            }
            let row = Rect::from_min_max(pos2(inner.left(), y), pos2(inner.right(), y + ROW_H));
            let is_sel = sel == Some(k);
            if is_sel {
                p.rect_filled(row, 0.0, winui::SELECTED_INACTIVE());
            }
            let check_x = inner.left() + 6.0;
            let mut on = rule.enabled;
            let changed =
                winui::checkbox_light(ui, pos2(check_x, y + 2.0), &mut on, &rule.name, true)
                    .changed();
            if changed {
                self.rules[k].enabled = on;
            }
        }
        // Click on a row selects it; it is drawn after the checkboxes.
        for k in 0..self.rules.len() {
            let y = inner.top() + k as f32 * ROW_H;
            if y + ROW_H > inner.bottom() {
                break;
            }
            let row =
                Rect::from_min_max(pos2(inner.left() + 26.0, y), pos2(inner.right(), y + ROW_H));
            let resp = ui.interact(
                row,
                egui::Id::unique(("packagizer-row", k)),
                egui::Sense::click(),
            );
            if resp.clicked() {
                self.selected = Some(k);
            }
        }
        // --- list buttons ---
        let btop = inner.bottom() + 4.0;
        let add = winui::button_light(
            ui,
            r(o, 11.0, btop, 96.0, 21.0),
            "Add rule",
            ButtonKind::Normal,
            false,
        );
        if add.clicked() {
            self.rules.push(Rule {
                id: self.next_id,
                ..Default::default()
            });
            self.next_id += 1;
            self.selected = Some(self.rules.len() - 1);
        }
        let dup_i = self
            .selected
            .and_then(|i| self.rules.get(i).map(|r| (i, r.clone())));
        let dup = winui::button_light(
            ui,
            r(o, 111.0, btop, 96.0, 21.0),
            "Duplicate",
            ButtonKind::Normal,
            false,
        );
        if dup.clicked()
            && let Some((at, mut r)) = dup_i
        {
            r.id = self.next_id;
            self.next_id += 1;
            r.name.push_str(" (copy)");
            self.rules.insert(at + 1, r);
            self.selected = Some(at + 1);
        }
        let del = winui::button_light(
            ui,
            r(o, 211.0, btop, 96.0, 21.0),
            "Remove",
            ButtonKind::Normal,
            false,
        );
        if del.clicked()
            && let Some(i) = self.selected
            && i < self.rules.len()
        {
            self.rules.remove(i);
            self.selected = None;
        }
        let up_i = self.selected.and_then(|i| (i > 0).then_some(i));
        let up = winui::button_light(
            ui,
            r(o, 311.0, btop, 44.0, 21.0),
            "↑",
            ButtonKind::Normal,
            false,
        );
        if up.clicked()
            && let Some(i) = up_i
        {
            self.rules.swap(i - 1, i);
            self.selected = Some(i - 1);
        }
        let dn_i = self
            .selected
            .and_then(|i| (i + 1 < self.rules.len()).then_some(i));
        let dn = winui::button_light(
            ui,
            r(o, 359.0, btop, 44.0, 21.0),
            "↓",
            ButtonKind::Normal,
            false,
        );
        if dn.clicked()
            && let Some(i) = dn_i
        {
            self.rules.swap(i, i + 1);
            self.selected = Some(i + 1);
        }

        // --- the editor for the selected rule ---
        if let Some(i) = self.selected
            && i < self.rules.len()
        {
            let ex = o.x + LIST_W + 22.0;
            let w = r(o, ex - o.x, 0.0, DIALOG_W - LIST_W - 33.0, 21.0).width();
            label(&p, ex, top + 30.0, "Name:");
            winui::edit_light(
                ui,
                r(o, ex - o.x, top + 44.0, w, 21.0),
                &mut self.rules[i].name,
                true,
                false,
            );
            label(&p, ex, top + 72.0, "URL (regex, empty = any):");
            winui::edit_light(
                ui,
                r(o, ex - o.x, top + 86.0, w, 21.0),
                &mut self.rules[i].url,
                true,
                false,
            );
            label(&p, ex, top + 114.0, "File name (regex, empty = any):");
            winui::edit_light(
                ui,
                r(o, ex - o.x, top + 128.0, w, 21.0),
                &mut self.rules[i].filename,
                true,
                false,
            );
            label(&p, ex, top + 156.0, "Extensions (rar, 7z…; empty = any):");
            winui::edit_light(
                ui,
                r(o, ex - o.x, top + 170.0, w, 21.0),
                &mut self.rules[i].exts,
                true,
                false,
            );
            label(&p, ex, top + 198.0, "Minimum size, bytes (0 = any):");
            let mut min_s = self.rules[i].min_size.to_string();
            winui::edit_light(
                ui,
                r(o, ex - o.x, top + 212.0, 140.0, 21.0),
                &mut min_s,
                true,
                false,
            );
            self.rules[i].min_size = min_s.trim().parse().unwrap_or(0);
            label(&p, ex, top + 240.0, "Maximum size, bytes (0 = any):");
            let mut max_s = self.rules[i].max_size.to_string();
            winui::edit_light(
                ui,
                r(o, ex - o.x, top + 254.0, 140.0, 21.0),
                &mut max_s,
                true,
                false,
            );
            self.rules[i].max_size = max_s.trim().parse().unwrap_or(0);
            label(&p, ex, top + 282.0, "Action:");
            let actions = [
                "Move to package",
                "Rename file",
                "Set package's destination",
                "Stop after this rule",
            ];
            let cur = match self.rules[i].action {
                Action::MoveToPackage => 0,
                Action::RenameFile => 1,
                Action::SetDestination => 2,
                Action::StopAfterRule => 3,
            };
            if let Some(picked) = winui::combo_light(
                ui,
                r(o, ex - o.x, top + 296.0, w, 21.0),
                &actions,
                cur,
                true,
                "packagizer-action",
            ) {
                self.rules[i].action = match picked {
                    0 => Action::MoveToPackage,
                    1 => Action::RenameFile,
                    2 => Action::SetDestination,
                    _ => Action::StopAfterRule,
                };
            }
            let (payload_label, payload_enabled) = match self.rules[i].action {
                Action::MoveToPackage => ("Package's name:", true),
                Action::RenameFile => ("New file name:", true),
                Action::SetDestination => (
                    "Destination (absolute, or relative to the download folder):",
                    true,
                ),
                Action::StopAfterRule => ("(This rule only stops the chain)", false),
            };
            label(&p, ex, top + 324.0, payload_label);
            let mut payload = self.rules[i].payload.clone();
            if winui::edit_light(
                ui,
                r(o, ex - o.x, top + 338.0, w, 21.0),
                &mut payload,
                payload_enabled,
                false,
            )
            .changed()
            {
                self.rules[i].payload = payload;
            }
            if self.status.is_none() {
                text(
                    &p,
                    ex,
                    top + 368.0,
                    "Variables: <filename> <name> <ext> <packagename> <index> <simpledate:yyyy-MM-dd>",
                    winui::MUTED(),
                );
            }
        }
        // --- bottom row ---
        let bbottom = o.y + DIALOG_H - 30.0;
        let apply = winui::button_light(
            ui,
            r(o, 11.0, bbottom, 150.0, 22.0),
            "&Apply to list",
            ButtonKind::Normal,
            false,
        );
        if apply.clicked() {
            return Outcome::Done(true);
        }
        let buttons = winui::buttons_light_right(
            ui,
            o,
            DIALOG_W - 11.0,
            bbottom,
            &[
                (crate::i18n::mark("OK"), ButtonKind::Default),
                (crate::i18n::mark("Cancel"), ButtonKind::Normal),
            ],
        );
        if buttons[0].clicked() {
            return Outcome::Done(false);
        }
        if buttons[1].clicked() {
            return Outcome::Closed;
        }
        if winui::escape_pressed(ui) {
            return Outcome::Closed;
        }
        if winui::enter_pressed(ui) {
            return Outcome::Done(false);
        }
        Outcome::Open
    }
}

fn label(p: &egui::Painter, x: f32, y: f32, s: &str) {
    text(p, x, y, s, crate::winui::MUTED());
}

#[cfg(test)]
mod capture_tests {
    use super::*;

    fn rule(action: Action, payload: &str) -> Rule {
        Rule {
            id: 1,
            enabled: true,
            name: "t".into(),
            url: String::new(),
            filename: String::new(),
            exts: String::new(),
            min_size: 0,
            max_size: 0,
            action,
            payload: payload.into(),
        }
    }

    #[test]
    fn filename_groups_feed_dollar_n() {
        let mut r = rule(Action::RenameFile, "$2-$1");
        r.filename = r"([a-z]+)([0-9]+)".into();
        let c = apply(&[r], "https://a/1", "ep12.mp4", None, "p", 1, 0);
        assert_eq!(c.name.as_deref(), Some("12-ep"));
    }

    #[test]
    fn url_groups_feed_dollar_n_when_filename_has_none() {
        let mut r = rule(Action::RenameFile, "book-$2");
        r.url = r"books/([a-z]+)/([0-9]+)".into();
        let c = apply(
            &[r],
            "https://a/books/epub/7",
            "a file.epub",
            None,
            "p",
            1,
            0,
        );
        assert_eq!(c.name.as_deref(), Some("book-7"));
    }

    #[test]
    fn named_groups() {
        let mut r = rule(Action::RenameFile, "${author}-$3");
        r.filename = r"(?<author>[a-z]+)\.(?<series>[a-z]+)([0-9]+)".into();
        let c = apply(&[r], "https://a", "alice.witch2001.mp4", None, "p", 1, 0);
        assert_eq!(c.name.as_deref(), Some("alice-2001"));
    }

    #[test]
    fn unknown_group_is_kept_as_written() {
        let mut r = rule(Action::RenameFile, "$9-${nope}");
        r.filename = r"([a-z]+)".into();
        let c = apply(&[r], "https://a", "alice.mp4", None, "p", 1, 0);
        assert_eq!(c.name.as_deref(), Some("$9-${nope}"));
    }
}
