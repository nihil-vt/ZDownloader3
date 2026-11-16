# Packagizer

ZDownloader3's rule chain for the link grabber, mirroring JDownloader2's Packagizer (`org.jdownloader.controlling.packagizer`). Links are matched when they are added (and again on demand) and **have their names changed, their packages moved, or their destination changed** — in the order your rules are listed, each rule seeing what the ones before it did.

Open it: **Downloads → Packagize&r**. "Apply to list" re-runs the chain on the links already staged in the grabber.

## Rule fields

| Field | Meaning |
|---|---|
| Enabled | the rule fires; disabled rules are kept for later |
| Name | shown in the list; free text |
| URL | regex matched against the whole URL (empty = any) |
| File name | regex against the file name (empty = any) |
| Extensions | comma/space separated (`rar 7z zip`); empty = any |
| Min/Max size | bytes; `0` = unbounded |
| Action | see below |

All conditions must hold for a rule to fire. Later rules see renames made by earlier ones — rule 2's `<filename>` is rule 1's result.

## Actions

| Action | What it does |
|---|---|
| Move to package | the link joins (creating it if needed) a package named by the pattern |
| Rename file | the link is renamed (not its destination) |
| Set package's destination | the package the link ends up in saves to this path |
| Stop after this rule | the chain ends for this link |

## Variables

In the payload pattern:

- `<filename>` — the name, with extension
- `<name>` — the name without extension
- `<ext>` — the extension
- `<packagename>` — the link's package *before* the rule runs
- `<index>` — its 1-based index in that package
- `<simpledate:FORMAT>` — today, e.g. `<simpledate:yyyy-MM-dd>` (subset: yyyy MM dd HH mm ss)

Unknown variables are carried through untouched.

## Notes and safety

- Bad regexes are inert (the rule never matches, they never crash).
- Renames never produce an empty or punctuation-only name; `..` is refused in destination patterns (a rule cannot move files outside the download folder).
- Rules apply at **add time** in the link grabber — the staged list you tick before adding — and on demand via "Apply to list". Direct IPC/command-line downloads bypass the grabber and are not touched.

## A starter rule: all ISOs into one package

| Field | Value |
|---|---|
| Extensions | `iso` |
| Action | Move to package |
| Payload | `Distros` |

## A starter rule: versioned releases, folder by date

| Field | Value |
|---|---|
| File name | `.*` |
| Action | Set package's destination |
| Payload | `releases/<simpledate:yyyy-MM-dd>` |

## Known limits (vs JD2's Packagizer)

- No regex capture groups in variables (`$1`-style); combine rules instead.
- No "priority" action yet (JD2 can set download priority per rule).
- No "disable this download" action (needs a global-enable action, not per-download).
