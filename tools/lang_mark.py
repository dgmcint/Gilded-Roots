#!/usr/bin/env python3
"""Marks on-screen English text in the Rust code for translation (one-off groundwork tool).

For each call that puts text on screen (ui.label, ui.button, menu_button, on_hover_text,
RichText::new, Window::new, hint_text, ...), the text argument is wrapped:
  "literal"            -> tr("literal")
  format!("..{x}..", a) -> trf("..{x}..", &[segments], &[parts])   (same output in English)
  other expressions     -> trv(expr)   (compiles only for text built into the program;
                                        tools/lang_fix.py removes it where it does not)
Run from the project folder:  python3 tools/lang_mark.py src/*.rs
"""
import re
import sys

# call name -> index of the text argument
CALLS = {
    "label": 0, "button": 0, "small_button": 0, "heading": 0, "menu_button": 0,
    "checkbox": 1, "selectable_label": 1, "selectable_value": 2, "radio_value": 2,
    "radio": 1, "toggle_value": 1, "on_hover_text": 0, "on_disabled_hover_text": 0,
    "on_hover_text_at_pointer": 0, "hint_text": 0, "hyperlink_to": 0, "selected_text": 0,
    "collapsing": 0, "colored_label": 1, "small": 0, "strong": 0, "weak": 0,
    "add_filter": 0, "set_title": 0, "set_description": 0, "from_label": 0,
    "image_and_text": 1, "show_tooltip_text": 2, "link": 0,
}
# Type::new(text) constructors
CTORS = {"RichText", "Button", "Label", "Window", "CollapsingHeader", "Checkbox",
         "SelectableLabel", "Link", "Hyperlink", "RadioButton"}
CTOR_ARG = {"Checkbox": 1, "SelectableLabel": 1, "RadioButton": 1}


def tokenize(src):
    """Yields (kind, start, end). Kinds: str, rawstr, char, ident, punct, ws, comment."""
    i, n = 0, len(src)
    toks = []
    while i < n:
        c = src[i]
        if c in " \t\r\n":
            j = i
            while j < n and src[j] in " \t\r\n":
                j += 1
            toks.append(("ws", i, j)); i = j; continue
        if src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            toks.append(("comment", i, j)); i = j; continue
        if src.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j): depth += 1; j += 2
                elif src.startswith("*/", j): depth -= 1; j += 2
                else: j += 1
            toks.append(("comment", i, j)); i = j; continue
        m = re.match(r'b?r(#*)"', src[i:])
        if m and (i == 0 or not (src[i-1].isalnum() or src[i-1] == "_")):
            hashes = m.group(1)
            end = src.find('"' + hashes, i + m.end())
            j = end + 1 + len(hashes)
            toks.append(("rawstr", i, j)); i = j; continue
        if c == '"' or (c == "b" and i + 1 < n and src[i+1] == '"' and not (i and (src[i-1].isalnum() or src[i-1] == "_"))):
            j = i + (2 if c == "b" else 1)
            while src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            toks.append(("str" if c == '"' else "bstr", i, j + 1)); i = j + 1; continue
        if c == "'":
            # char literal or lifetime
            m = re.match(r"'(\\(x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]+\}|.)|[^\\'])'", src[i:], re.S)
            if m:
                toks.append(("char", i, i + m.end())); i += m.end(); continue
            j = i + 1
            while j < n and (src[j].isalnum() or src[j] == "_"):
                j += 1
            toks.append(("ident", i, j)); i = j; continue
        if c.isalpha() or c == "_":
            j = i
            while j < n and (src[j].isalnum() or src[j] == "_"):
                j += 1
            toks.append(("ident", i, j)); i = j; continue
        if c.isdigit():
            m = re.match(r"[0-9][0-9_]*(\.[0-9][0-9_]*)?([eE][+-]?[0-9]+)?[a-z0-9]*", src[i:])
            # don't swallow "1.." ranges or method calls on numbers
            j = i + m.end()
            if "." in m.group(0) and not re.match(r"[0-9_]*\.[0-9]", m.group(0)):
                j = i + 1
            toks.append(("num", i, j)); i = j; continue
        toks.append(("punct", i, i + 1)); i += 1
    return toks


def code(toks):
    return [t for t in toks if t[0] not in ("ws", "comment")]


OPEN, CLOSE = "([{", ")]}"


def match_close(src, ct, k):
    """ct[k] is an opening bracket; returns index of its closing bracket in ct."""
    depth = 0
    for m in range(k, len(ct)):
        kind, a, b = ct[m]
        if kind == "punct":
            ch = src[a]
            if ch in OPEN: depth += 1
            elif ch in CLOSE:
                depth -= 1
                if depth == 0:
                    return m
    raise ValueError("unbalanced")


def split_args(src, ct, k, end):
    """Arguments between ct[k] '(' and ct[end] ')': list of (first, last) code-token indices."""
    args, depth, start = [], 0, k + 1
    pipes = 0  # inside closure params |a, b|
    for m in range(k + 1, end):
        kind, a, b = ct[m]
        if kind != "punct":
            continue
        ch = src[a]
        if ch in OPEN: depth += 1
        elif ch in CLOSE: depth -= 1
        elif ch == "|" and depth == 0:
            pipes ^= 1
        elif ch == "," and depth == 0 and not pipes:
            args.append((start, m - 1)); start = m + 1
    if start <= end - 1:
        args.append((start, end - 1))
    return args


def is_texty(lit):
    """A literal worth translating: has letters, and is not a code word like "BIRT" or "_TODO"."""
    body = lit[1:-1]
    if body in ("OK",):
        return True
    if not re.search(r"[A-Za-z]", body):
        return False
    letters = re.sub(r"\{[^}]*\}", "", body)
    if not re.search(r"[a-z]", letters):
        # all capitals: only words like "OK"? leave alone
        return False
    if re.fullmatch(r"[a-z0-9_.\-/#]+", body) and " " not in body:
        # lower-case identifiers, file extensions, ids
        return False
    return True


FMT_HOLE = re.compile(r"\{\{|\}\}|\{([^{}]*)\}")


def convert_format(src, ct, k_macro, end):
    """format!( ... ) at ct[k_macro] ('format'), ct[k_macro+2] '(', ct[end] ')'. Returns text or None."""
    args = split_args(src, ct, k_macro + 2, end)
    if not args:
        return None
    f0, f1 = args[0]
    if f0 != f1 or ct[f0][0] != "str":
        return None
    lit = src[ct[f0][1]:ct[f0][2]]
    if not is_texty(lit):
        return None
    body = lit[1:-1]
    rest = [src[ct[a][1]:ct[b][2]] for a, b in args[1:]]
    pos_args, named = [], {}
    for r in rest:
        m = re.match(r"\s*([A-Za-z_][A-Za-z0-9_]*)\s*=(?!=)\s*(.*)$", r, re.S)
        if m:
            named[m.group(1)] = m.group(2)
        else:
            pos_args.append(r)
    segs, parts, last, auto = [], [], 0, 0
    for m in FMT_HOLE.finditer(body):
        if m.group(1) is None:
            continue  # escaped brace, stays in the segment text
        inner = m.group(1)
        name, _, spec = inner.partition(":")
        if "$" in spec or "*" in spec:
            return None
        if name == "":
            if auto >= len(pos_args):
                return None
            expr = pos_args[auto]; auto += 1
        elif name.isdigit():
            if int(name) >= len(pos_args):
                return None
            expr = pos_args[int(name)]
        elif name in named:
            expr = named[name]
        elif re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name):
            expr = name
        else:
            return None
        segs.append(body[last:m.start()])
        if spec:
            parts.append('format!("{:%s}", %s)' % (spec, expr.strip()))
        else:
            parts.append("(%s).to_string()" % expr.strip())
        last = m.end()
    segs.append(body[last:])
    if not parts:
        # no holes: a plain text written with format!
        return None

    def seg_lit(s):
        # segments are literal text: undo {{ }} escaping (still a Rust string literal body)
        return '"' + s.replace("{{", "{").replace("}}", "}") + '"'
    return "crate::i18n::trf(%s, &[%s], &[%s])" % (lit, ", ".join(seg_lit(s) for s in segs), ", ".join(parts))


def process(path):
    src = open(path, encoding="utf-8").read()
    toks = tokenize(src)
    ct = code(toks)
    edits = []  # (start, end, replacement)
    stats = {"tr": 0, "trf": 0, "trv": 0}

    def wrap_arg(a0, a1):
        """Wrap the expression ct[a0..=a1]."""
        s, e = ct[a0][1], ct[a1][2]
        expr = src[s:e]
        k0 = ct[a0]
        # literal
        if a0 == a1 and k0[0] == "str":
            if is_texty(expr):
                edits.append((s, e, "crate::i18n::tr(%s)" % expr)); stats["tr"] += 1
            return
        # format!(...)
        if k0[0] == "ident" and src[k0[1]:k0[2]] == "format" and a0 + 2 <= a1 and src[ct[a0+1][1]] == "!" \
                and src[ct[a0+2][1]] == "(":
            close = match_close(src, ct, a0 + 2)
            if close == a1:
                r = convert_format(src, ct, a0, close)
                if r:
                    edits.append((s, e, r)); stats["trf"] += 1
            return
        if a0 + 1 <= a1 and src[k0[1]] == "&" and ct[a0+1][0] == "ident" and src[ct[a0+1][1]:ct[a0+1][2]] == "format":
            close = match_close(src, ct, a0 + 3) if a0 + 3 <= a1 else None
            if close == a1:
                r = convert_format(src, ct, a0 + 1, close)
                if r:
                    edits.append((s, e, "&" + r)); stats["trf"] += 1
            return
        # constructors and things that are not plain text: leave (their own text is handled)
        first = src[k0[1]:k0[2]]
        if first == "if":
            for j in range(a0, a1 + 1):
                t = ct[j]
                if t[0] == "str" and src[ct[j-1][1]] == "{" and j + 1 <= a1 and src[ct[j+1][1]] == "}" \
                        and is_texty(src[t[1]:t[2]]):
                    edits.append((t[1], t[2], "crate::i18n::tr(%s)" % src[t[1]:t[2]])); stats["tr"] += 1
            return
        if first in CTORS or first in ("egui", "eframe", "crate", "match", "move", "|", "RichText", "WidgetText", "LayoutJob"):
            return
        # plain paths: x, self.x, x.y, &x, *x, x[i], x.0, x.method()
        if any(t[0] in ("str", "rawstr") for t in ct[a0:a1+1]):
            return
        if any(src[t[1]] in "{|" for t in ct[a0:a1+1] if t[0] == "punct"):
            return
        edits.append((s, e, "crate::i18n::trv(%s)" % expr)); stats["trv"] += 1

    for k, t in enumerate(ct):
        if t[0] != "ident":
            continue
        name = src[t[1]:t[2]]
        if k + 1 >= len(ct) or ct[k+1][0] != "punct" or src[ct[k+1][1]] != "(":
            continue
        prev = src[ct[k-1][1]:ct[k-1][2]] if k else ""
        if name in CALLS and prev == ".":
            idx = CALLS[name]
        elif name in ("new", "from_label_and_url") and k >= 3 and src[ct[k-1][1]:ct[k-1][2]] == ":" \
                and src[ct[k-3][1]:ct[k-3][2]] in CTORS:
            idx = CTOR_ARG.get(src[ct[k-3][1]:ct[k-3][2]], 0)
        else:
            continue
        if prev == "fn" or (k >= 1 and src[ct[k-1][1]:ct[k-1][2]] == "fn"):
            continue
        close = match_close(src, ct, k + 1)
        args = split_args(src, ct, k + 1, close)
        if idx < len(args):
            wrap_arg(*args[idx])

    # apply edits, innermost/last first; skip overlapping ones (outer already covers)
    edits.sort(key=lambda x: (x[0], -x[1]))
    keep = []
    for ed in edits:
        if keep and ed[0] < keep[-1][1]:
            # nested inside a previous edit: drop the outer one? nested edits happen when an
            # argument contains another call (e.g. trv(x.label("..."))) - outer wins only if
            # it is a literal/format conversion; otherwise keep the inner one.
            outer = keep[-1]
            if outer[2].startswith("crate::i18n::trv("):
                keep[-1] = ed
            continue
        keep.append(ed)
    out = src
    for s, e, r in sorted(keep, reverse=True):
        out = out[:s] + r + out[e:]
    if out != src:
        open(path, "w", encoding="utf-8").write(out)
    return stats


if __name__ == "__main__":
    total = {"tr": 0, "trf": 0, "trv": 0}
    for p in sys.argv[1:]:
        st = process(p)
        for k in total: total[k] += st[k]
        print(p, st)
    print("total", total)
