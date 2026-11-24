#!/usr/bin/env python3
"""Collects every English text that can be translated into lang/en.json, and shows how much of
each language is done.

    python3 tools/lang_extract.py           update lang/en.json and show progress
    python3 tools/lang_extract.py missing de   list the German texts not translated yet

lang/en.json lists each English text with where it is used (file:line), for the translator.
Each language file (lang/de.json, ...) has the same English texts with their translations.
Texts are taken from tr("..."), trf("...") and the lists in the code that menus and toolbars
show through trv(...).
"""
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(__file__))
from lang_mark import tokenize, code, is_texty  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# files with no on-screen text of their own (file formats, sample data, tests)
SKIP = {"tests.rs", "bigtree.rs", "i18n.rs", "gedcom.rs", "paf.rs", "sample.rs", "temples.rs", "storage.rs", "archive.rs"}
NOT_TEXT_CALLS = {"format", "println", "eprintln", "write", "writeln", "panic", "expect", "assert", "assert_eq",
                  "debug_assert", "unreachable", "todo", "include_str", "include_bytes", "concat", "env", "join",
                  "starts_with", "ends_with", "contains", "find", "split", "replace", "strip_prefix",
                  "strip_suffix", "get", "insert", "remove", "push_str", "eq", "trim_start_matches", "id_salt",
                  "from_id_salt", "Id", "new_id", "salt", "with_extension", "extension"}


def literal_value(lit):
    """The text of a Rust string literal, with its escapes worked out."""
    body, out, i = lit[1:-1], [], 0
    simple = {"n": "\n", "t": "\t", "r": "\r", "0": "\0", "\\": "\\", '"': '"', "'": "'"}
    while i < len(body):
        c = body[i]
        if c != "\\":
            out.append(c); i += 1; continue
        e = body[i + 1]
        if e in simple:
            out.append(simple[e]); i += 2
        elif e == "u":
            j = body.index("}", i)
            out.append(chr(int(body[i + 3:j], 16))); i = j + 1
        elif e == "x":
            out.append(chr(int(body[i + 2:i + 4], 16))); i += 4
        elif e == "\n":
            # line continuation: skip the newline and the next line's leading spaces
            i += 2
            while i < len(body) and body[i] in " \t\n\r":
                i += 1
        else:
            out.append(e); i += 2
    return "".join(out)


def collect():
    found = {}
    src_dir = os.path.join(ROOT, "src")
    for fn in sorted(os.listdir(src_dir)):
        if not fn.endswith(".rs") or fn in SKIP:
            continue
        path = os.path.join(src_dir, fn)
        src = open(path, encoding="utf-8").read()
        ct = code(tokenize(src))
        line_at = lambda pos: src.count("\n", 0, pos) + 1
        stack = []  # (bracket char, is_call, call name)
        test_end = -1  # skip #[cfg(test)] items and #[test] functions
        for k, (kind, a, b) in enumerate(ct):
            text = src[a:b]
            if k <= test_end:
                continue
            if text == "#" and k + 4 < len(ct) and src[ct[k+2][1]:ct[k+2][2]] in ("cfg", "test"):
                attr = src[ct[k+1][1]:ct[k+6][2]] if k + 6 < len(ct) else ""
                if src[ct[k+2][1]:ct[k+2][2]] == "test" or "cfg(test)" in attr.replace(" ", ""):
                    m = k
                    while m < len(ct) and src[ct[m][1]] != "{":
                        m += 1
                    depth = 0
                    for m2 in range(m, len(ct)):
                        ch = src[ct[m2][1]]
                        if ct[m2][0] == "punct" and ch == "{": depth += 1
                        elif ct[m2][0] == "punct" and ch == "}":
                            depth -= 1
                            if depth == 0:
                                test_end = m2
                                break
                    continue
            if kind == "punct" and text in "([{":
                prev = src[ct[k-1][1]:ct[k-1][2]] if k else ""
                prev2 = src[ct[k-2][1]:ct[k-2][2]] if k > 1 else ""
                is_call = text == "(" and (ct[k-1][0] == "ident" or prev in (")", ">", "!")) and prev not in ("if", "match", "in", "return")
                name = prev2 if prev == "!" else prev
                stack.append((text, is_call, name))
                continue
            if kind == "punct" and text in ")]}":
                if stack:
                    stack.pop()
                continue
            if kind != "str":
                continue
            prev = src[ct[k-1][1]:ct[k-1][2]]
            # tr("...") / trf("...") / trv("...")
            if len(ct) > k and stack and stack[-1][1] and stack[-1][2] in ("tr", "trv", "trf"):
                # trf's first argument only (the others are its pieces)
                if stack[-1][2] != "trf" or prev == "(":
                    if prev == "(":
                        found.setdefault(literal_value(text), f"{fn}:{line_at(a)}")
                continue
            if any(fr[1] and fr[2] == "trf" for fr in stack):
                continue  # the pieces of a trf, not texts of their own
            if not is_texty(text) or re.search(r"<[a-zA-Z/!?][^>]*>", text):
                continue  # code words, and HTML or SVG for printouts
            # a list or table entry, or a match arm's result: may be shown through trv
            nxt = src[ct[k+1][1]:ct[k+1][2]] if k + 1 < len(ct) else ""
            if nxt not in (",", ")", "]", "}"):
                continue
            value = literal_value(text)
            if not re.match(r"[A-Z0-9(<+.\"]", value) or re.search(r"[&=@]", value) or "{" in value and "}" in value and not re.search(r"[a-z]{3,} [a-z]{2,}", value):
                continue  # list entries shown on screen start with a capital letter; skip data and code
            # a field of a list of lessons or steps (the guided tour): title: "...", text: "..."
            prev2 = src[ct[k-2][1]:ct[k-2][2]] if k > 1 else ""
            if fn == "tour.rs" and prev == ":" and prev2 in ("title", "about", "text"):
                found.setdefault(value, f"{fn}:{line_at(a)}")
                continue
            if prev == "=>" or (stack and not stack[-1][1] and stack[-1][0] in "([") :
                found.setdefault(literal_value(text), f"{fn}:{line_at(a)}")
    return found


def load(path):
    try:
        return json.load(open(path, encoding="utf-8"))
    except (OSError, ValueError):
        return {}


def main():
    lang_dir = os.path.join(ROOT, "lang")
    found = collect()
    en = {"_about": "Every English text in Gilded Roots, with where it is used. Translations go in the "
                    "other files in this folder, as \"English\": \"translation\". Keep {…} parts as they are."}
    en.update(dict(sorted(found.items(), key=lambda kv: kv[0].lower())))
    if len(sys.argv) >= 3 and sys.argv[1] == "missing":
        tr = load(os.path.join(lang_dir, sys.argv[2] + ".json"))
        for k, where in en.items():
            if not k.startswith("_") and not tr.get(k, "").strip():
                print(json.dumps(k, ensure_ascii=False), " #", where)
        return
    with open(os.path.join(lang_dir, "en.json"), "w", encoding="utf-8") as f:
        json.dump(en, f, ensure_ascii=False, indent=1)
        f.write("\n")
    total = len(found)
    print(f"{total} English texts in lang/en.json")
    for fn in sorted(os.listdir(lang_dir)):
        if fn == "en.json" or not fn.endswith(".json"):
            continue
        tr = load(os.path.join(lang_dir, fn))
        done = sum(1 for k in found if tr.get(k, "").strip())
        print(f"  {fn[:-5]}: {done}/{total} ({100 * done // max(total, 1)}%)")


if __name__ == "__main__":
    main()
