#!/usr/bin/env python3
"""Render the two data pages of the fork site from Markdown sources in ../docs."""
import html
import pathlib
import re

ROOT = pathlib.Path(__file__).parent.parent
DST = pathlib.Path(__file__).parent

HEADER = """<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="shortcut icon" href="assets/icon.svg">
<title>{title} — ZDownloader3</title>
<link rel="stylesheet" href="style.css">
</head>
<body>
<main>
<h1 class="product">{title}</h1>
<p class="audible">{blurb}</p>
"""
FOOTER = """
<p class="footer"><a href="index.html">← Home</a> · <a href="why.html">Why</a> · <a href="docs.html">Docs</a> · <a href="roadmap.html">Roadmap</a> · <a href="changelog.html">Changelog</a></p>
</main>
</body>
</html>
"""

def md_to_html(md: str) -> str:
    out = []
    for raw in md.split("\n"):
        line = raw.rstrip()
        if not line.strip():
            out.append("")
            continue
        m = re.match(r"^(#{1,4})\s+(.*)$", line)
        if m:
            level = len(m.group(1)) + 1  # shifted: h2 under our page h1
            out.append(f"<h{level}>{html.escape(m.group(2))}</h{level}>")
            continue
        if line.startswith("> "):
            out.append(f"<blockquote><p>{render_inline(line[2:])}</p></blockquote>")
            continue
        if re.match(r"^- ", line):
            out.append(f"<li>{render_inline(line[2:])}</li>")
            continue
        out.append(f"<p>{render_inline(line)}</p>")
    text = "\n".join(out)
    text = re.sub(r"(?:<li>.*</li>\n)+", lambda m: "<ul>\n" + m.group(0) + "</ul>\n", text)
    text = re.sub(r"<blockquote>", "<blockquote>", text)
    return text

def render_inline(text: str) -> str:
    text = html.escape(text)
    text = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", text)
    text = re.sub(r"`([^`]+)`", r"<code>\1</code>", text)
    text = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", r'<a href="\2">\1</a>', text)
    return text

pages = [
    ("roadmap.html", "Roadmap", "What the fork owns, what it inherits, what ships when.", ROOT / "docs" / "fork-roadmap.md", "What the fork owns, what it inherits, what ships when."),
    ("changelog.html", "Changelog", "The fork's ledger.", ROOT / "CHANGELOG.md", "The fork's ledger."),
]
for name, title, blurb, src, bl in pages:
    body = md_to_html(src.read_text(encoding="utf-8"))
    page = HEADER.format(title=title, blurb=bl) + body + FOOTER
    (DST / name).write_text(page, encoding="utf-8")
    print("wrote", name)
