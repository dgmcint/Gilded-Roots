#!/usr/bin/env python3
"""Turn the plain-text READMEs in packaging/ into PDFs with a normal white page.

A .txt file opens in TextEdit (Mac) or other text editors, which show it white-on-black when the
computer uses dark mode. A PDF always shows as a white page, so the READMEs that ship with the
downloads are PDFs. The .txt files stay the ones to edit; run this after changing them:

    python3 tools/make_readme_pdf.py

Needs reportlab (pip install reportlab). Writes packaging/README.pdf and
packaging/PORTABLE-README.pdf, which are committed alongside the .txt files.

Text layout understood:
  a line underlined with ===  -> the title
  a line underlined with ---  -> a section heading
  CAPITALS ENDING IN A COLON-less line on its own (e.g. "WHAT TO KEEP TOGETHER") -> a heading
  lines indented by 4+ spaces or starting with a path/command -> shown in a fixed-width font
  "  - " or "N. " lines -> list items (kept as written, with a hanging indent)
"""
import os
import re
import sys
from xml.sax.saxutils import escape

from reportlab.lib.colors import HexColor
from reportlab.lib.enums import TA_LEFT
from reportlab.lib.pagesizes import LETTER
from reportlab.lib.styles import ParagraphStyle
from reportlab.lib.units import inch
from reportlab.platypus import HRFlowable, Paragraph, Preformatted, SimpleDocTemplate, Spacer

HERE = os.path.dirname(os.path.abspath(__file__))
PKG = os.path.join(HERE, "..", "packaging")
GOLD = HexColor("#9a7b2f")
INK = HexColor("#1f1f1f")
GREY = HexColor("#555555")

title = ParagraphStyle("title", fontName="Helvetica-Bold", fontSize=20, leading=24, textColor=INK, spaceAfter=4)
head = ParagraphStyle("head", fontName="Helvetica-Bold", fontSize=13, leading=16, textColor=GOLD, spaceBefore=14, spaceAfter=4, keepWithNext=1)
body = ParagraphStyle("body", fontName="Helvetica", fontSize=10.5, leading=14.5, textColor=INK, alignment=TA_LEFT, spaceAfter=3)
item = ParagraphStyle("item", parent=body, leftIndent=18, firstLineIndent=-12)
sub = ParagraphStyle("sub", parent=body, leftIndent=34, firstLineIndent=-12)
code = ParagraphStyle("code", fontName="Courier", fontSize=9, leading=12, textColor=INK, leftIndent=18, backColor=HexColor("#f3f1ec"), borderPadding=(4, 6, 4, 6), spaceBefore=3, spaceAfter=6)
foot = ParagraphStyle("foot", parent=body, fontSize=9, leading=12, textColor=GREY)


def inline(text):
    # escape, then show file names, menu paths and commands a little bolder
    t = escape(text)
    t = re.sub(r"(Gilded ?Roots[\w-]*\.(?:exe|dmg|tar\.gz|txt|pdf))", r"<b>\1</b>", t)
    return t


def is_heading(line):
    s = line.strip()
    return bool(s) and s == s.upper() and any(c.isalpha() for c in s) and len(s) < 60 and not s.startswith(("-", "*"))


def build(src, dst):
    lines = open(src, encoding="utf-8").read().splitlines()
    story = []
    i = 0
    para = []

    def flush():
        if para:
            story.append(Paragraph(inline(" ".join(p.strip() for p in para)), body))
            para.clear()

    first = True
    while i < len(lines):
        line = lines[i].rstrip()
        if first and line.strip():
            first = False
            if not (i + 1 < len(lines) and set(lines[i + 1].strip()) <= {"=", "-"} and lines[i + 1].strip()):
                story.append(Paragraph(inline(line.strip()), title))
                story.append(HRFlowable(width="100%", thickness=1.5, color=GOLD, spaceBefore=2, spaceAfter=10))
                i += 1
                continue
        nxt = lines[i + 1].rstrip() if i + 1 < len(lines) else ""
        if line and nxt and set(nxt) <= {"="} and len(nxt) >= 3:
            flush()
            story.append(Paragraph(inline(line.strip()), title))
            story.append(HRFlowable(width="100%", thickness=1.5, color=GOLD, spaceBefore=2, spaceAfter=10))
            i += 2
            continue
        if line and nxt and set(nxt) <= {"-"} and len(nxt) >= 3:
            flush()
            story.append(Paragraph(inline(line.strip()), head))
            i += 2
            continue
        if not line.strip():
            flush()
            i += 1
            continue
        if is_heading(line) and not line.startswith("  "):
            flush()
            story.append(Paragraph(inline(line.strip()), head))
            i += 1
            continue
        indent = len(line) - len(line.lstrip())
        stripped = line.strip()
        if indent >= 2 and re.search(r"\S {2,}\S", stripped) and not re.match(r"^(- |\d+\. )", stripped):
            # a two-column listing (file name, then what it is): keep the columns lined up
            flush()
            block = [line]
            while i + 1 < len(lines) and lines[i + 1].strip() and lines[i + 1].startswith("  "):
                i += 1
                block.append(lines[i].rstrip())
            base = min(len(b) - len(b.lstrip()) for b in block)
            story.append(Preformatted("\n".join(b[base:] for b in block), code))
            i += 1
            continue
        if indent >= 4 and not re.match(r"^(- |\d+\. )", stripped) and (indent >= 6 or re.match(r"^[\w./~\"'$-]", stripped)) and not para:
            # a block of commands or a table-like listing: keep it exactly as written
            block = [line]
            while i + 1 < len(lines) and lines[i + 1].strip() and (len(lines[i + 1]) - len(lines[i + 1].lstrip())) >= 4 and not re.match(r"^\s*(- |\d+\. )", lines[i + 1]):
                i += 1
                block.append(lines[i].rstrip())
            base = min(len(b) - len(b.lstrip()) for b in block)
            story.append(Preformatted("\n".join(b[base:] for b in block), code))
            i += 1
            continue
        m = re.match(r"^(\s*)(- |\* |\d+\. )(.*)$", line)
        if m:
            flush()
            text = [m.group(3)]
            lead = len(m.group(1)) + len(m.group(2))
            # continuation lines are indented to line up under the text
            while i + 1 < len(lines) and lines[i + 1].strip() and not re.match(r"^\s*(- |\* |\d+\. )", lines[i + 1]) and (len(lines[i + 1]) - len(lines[i + 1].lstrip())) >= lead and (len(lines[i + 1]) - len(lines[i + 1].lstrip())) < lead + 4:
                i += 1
                text.append(lines[i].strip())
            bullet = "•" if m.group(2).strip() in ("-", "*") else m.group(2).strip()
            style = sub if len(m.group(1)) >= 3 else item
            story.append(Paragraph(f"{bullet}&nbsp;&nbsp;{inline(' '.join(text))}", style))
            i += 1
            continue
        para.append(line)
        i += 1
    flush()
    if story and isinstance(story[-1], Paragraph):
        story[-1].style = foot

    doc = SimpleDocTemplate(dst, pagesize=LETTER, leftMargin=0.9 * inch, rightMargin=0.9 * inch, topMargin=0.8 * inch, bottomMargin=0.8 * inch,
                            title="Gilded Roots - Read me", author="DGMCINT", subject="How to install and use Gilded Roots")
    doc.build(story)


if __name__ == "__main__":
    pairs = [("DOWNLOAD-README.txt", "README.pdf"), ("PORTABLE-README.txt", "PORTABLE-README.pdf")]
    for src, dst in pairs:
        build(os.path.join(PKG, src), os.path.join(PKG, dst))
        print("wrote", os.path.normpath(os.path.join(PKG, dst)))
    sys.exit(0)
