#!/usr/bin/env python3
"""Removes trv(...) wherever the compiler says it does not fit (the text was not built into the
program: a name, a number, text the user typed). Repeats until no error points at a trv.
Run from the project folder after tools/lang_mark.py:  python3 tools/lang_fix.py
"""
import json
import subprocess

MARK = "crate::i18n::trv("


def wrappers(src):
    """(start, open_paren_end, close_index) for every trv( in src."""
    out, i = [], 0
    while True:
        i = src.find(MARK, i)
        if i < 0:
            return out
        j, depth = i + len(MARK), 1
        k = j
        in_str = False
        while depth:
            c = src[k]
            if in_str:
                if c == "\\": k += 1
                elif c == '"': in_str = False
            elif c == '"': in_str = True
            elif c == "(": depth += 1
            elif c == ")": depth -= 1
            k += 1
        out.append((i, j, k - 1))
        i = j


def spans_of(msg, acc):
    for sp in msg.get("spans", []):
        acc.append((sp["file_name"], sp["byte_start"], sp["byte_end"]))
        if sp.get("expansion"):
            pass
    for ch in msg.get("children", []):
        spans_of(ch, acc)


def once():
    p = subprocess.run(["cargo", "check", "--message-format=json"], capture_output=True, text=True)
    targets = {}
    errors = 0
    for line in p.stdout.splitlines():
        try:
            m = json.loads(line)
        except ValueError:
            continue
        if m.get("reason") != "compiler-message" or m["message"]["level"] != "error":
            continue
        errors += 1
        acc = []
        spans_of(m["message"], acc)
        for f, a, b in acc:
            targets.setdefault(f, []).append((a, b))
    removed = 0
    for f, spans in targets.items():
        try:
            src = open(f, encoding="utf-8").read()
        except OSError:
            continue
        raw = src.encode("utf-8")
        # byte offsets -> char offsets
        def ch(o):
            return len(raw[:o].decode("utf-8", "ignore"))
        spans = [(ch(a), ch(b)) for a, b in spans]
        ws = wrappers(src)
        hit = set()
        for a, b in spans:
            best = None
            for w in ws:
                if w[0] <= a and b <= w[2] + 1:
                    if best is None or w[0] > best[0]:
                        best = w
            if best:
                hit.add(best)
        for s, j, c in sorted(hit, reverse=True):
            src = src[:s] + src[j:c] + src[c + 1:]
            removed += 1
        if hit:
            open(f, "w", encoding="utf-8").write(src)
    return errors, removed


if __name__ == "__main__":
    for n in range(15):
        e, r = once()
        print(f"pass {n + 1}: {e} errors, removed {r}")
        if e == 0 or r == 0:
            break
