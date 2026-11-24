#!/usr/bin/env python3
"""Third groundwork pass: text passed to the program's own helper functions.

Many screens use small helpers (a welcome card, a section heading, a toolbar button) that get
their text from where they are called. When a helper only shows that text (every use of the
parameter puts it on screen), the English given to it at each call is marked with tr() or trf().
Helpers that also use the text for anything else (a saved setting, a lookup) are left alone.
Run from the tools folder:  python3 lang_mark_helpers.py ../src/*.rs
"""
import re
import sys
from collections import defaultdict
from lang_mark import tokenize, code, match_close, split_args, convert_format, is_texty, CALLS, CTORS, CTOR_ARG

TEXT_TYPES = re.compile(r"^&(?:'[a-z_]+\s+)?str$|^impl\s+Into<(?:String|egui::WidgetText|WidgetText)>$|^String$")
# extra: constructors whose text arguments are only shown
EXTRA = {("Tool", "new"): [2, 3]}


def text_of(src, t):
    return src[t[1]:t[2]]


def ui_use(src, ct, m):
    """Is the parameter at ct[m] put straight on screen? (the whole text argument of a UI call,
    possibly as &p, p.to_string(), p.into(), p.clone())"""
    # find the start and end of the simple expression around ct[m]
    a = m
    if a > 0 and text_of(src, ct[a-1]) in ("&", "*"):
        a -= 1
    b = m
    while b + 3 < len(ct) and text_of(src, ct[b+1]) == "." and text_of(src, ct[b+2]) in ("to_string", "into", "clone", "to_owned", "as_str") \
            and text_of(src, ct[b+3]) == "(" and text_of(src, ct[b+4]) == ")":
        b += 4
    before, after = text_of(src, ct[a-1]), text_of(src, ct[b+1])
    if after not in (",", ")"):
        return False
    # walk back to the opening bracket of the call this argument belongs to, counting commas
    depth, idx, k = 0, 0, a - 1
    while k >= 0:
        t = text_of(src, ct[k])
        if ct[k][0] == "punct":
            if t in ")]}":
                depth += 1
            elif t in "([{":
                if depth == 0:
                    break
                depth -= 1
            elif t == "," and depth == 0:
                idx += 1
        k -= 1
    if k < 1 or text_of(src, ct[k]) != "(":
        return False
    name = text_of(src, ct[k-1])
    if name in CALLS and text_of(src, ct[k-2]) == ".":
        return CALLS[name] == idx
    if name == "new" and k >= 4 and text_of(src, ct[k-4]) in CTORS:
        return CTOR_ARG.get(text_of(src, ct[k-4]), 0) == idx
    if name == "layout_no_wrap" or name == "layout":
        return idx == 0
    if name in ("tr", "trv"):
        return True
    return False


def param_ok(src, ct, ptxt, body_start, body_end):
    m = re.match(r"(?:mut\s+)?([a-z_][a-z0-9_]*)\s*:\s*(.+)$", ptxt, re.S)
    if not m or not TEXT_TYPES.match(m.group(2).strip()):
        return None
    pname = m.group(1)
    uses = [j for j in range(body_start, body_end) if ct[j][0] == "ident" and text_of(src, ct[j]) == pname
            and text_of(src, ct[j-1]) != "."]
    return bool(uses) and all(ui_use(src, ct, j) for j in uses)


def find_closures(src, ct):
    """{name: {arg index: ok}} for `let name = |a: &str, ...| body` in one file."""
    out = {}
    for k in range(len(ct) - 4):
        if text_of(src, ct[k]) != "let":
            continue
        j = k + 1
        if text_of(src, ct[j]) == "mut":
            j += 1
        name = text_of(src, ct[j])
        if ct[j][0] != "ident" or text_of(src, ct[j+1]) != "=":
            continue
        j += 2
        if text_of(src, ct[j]) == "move":
            j += 1
        if text_of(src, ct[j]) != "|":
            continue
        # params up to the closing |
        e = j + 1
        depth = 0
        while e < len(ct):
            x = text_of(src, ct[e])
            if x in "([<": depth += 1
            elif x in ")]>": depth -= 1
            elif x == "|" and depth == 0: break
            e += 1
        if e >= len(ct) - 1:
            continue
        params, start = [], j + 1
        depth = 0
        for q in range(j + 1, e + 1):
            x = text_of(src, ct[q])
            if x in "([<": depth += 1
            elif x in ")]>": depth -= 1
            if (x == "," and depth == 0) or q == e:
                if q > start:
                    params.append(src[ct[start][1]:ct[q-1][2]])
                start = q + 1
        b = e + 1
        if text_of(src, ct[b]) == "-":  # -> Type
            while b < len(ct) - 1 and text_of(src, ct[b]) not in ("{", ";"):
                b += 1
        if text_of(src, ct[b]) != "{":
            continue
        body_end = match_close(src, ct, b)
        res = {}
        for i, ptxt in enumerate(params):
            ok = param_ok(src, ct, ptxt, b + 1, body_end)
            if ok:
                res[i] = True
        if res:
            out[name] = (res, body_end)
    return out


def find_helpers(files):
    """{fn name: {param index: ok}} where ok means the param is only shown."""
    found = defaultdict(dict)
    count = defaultdict(int)
    for path in files:
        src = open(path, encoding="utf-8").read()
        ct = code(tokenize(src))
        for k, t in enumerate(ct):
            if text_of(src, t) == "fn" and k + 1 < len(ct):
                count[text_of(src, ct[k+1])] += 1
    for path in files:
        src = open(path, encoding="utf-8").read()
        ct = code(tokenize(src))
        for k, t in enumerate(ct):
            if text_of(src, t) != "fn" or k + 2 >= len(ct):
                continue
            name = text_of(src, ct[k+1])
            p = k + 2
            if text_of(src, ct[p]) == "<":
                depth = 0
                while True:
                    s = text_of(src, ct[p])
                    depth += s == "<"
                    depth -= s == ">"
                    p += 1
                    if depth == 0:
                        break
            if text_of(src, ct[p]) != "(":
                continue
            close = match_close(src, ct, p)
            params = split_args(src, ct, p, close)
            # the body
            q = close + 1
            while q < len(ct) and text_of(src, ct[q]) not in ("{", ";"):
                q += 1
            if q >= len(ct) or text_of(src, ct[q]) == ";":
                continue
            body_end = match_close(src, ct, q)
            is_method = bool(params) and "self" in text_of(src, ct[params[0][0]]) + text_of(src, ct[min(params[0][0]+1, params[0][1])])
            if count[name] > 1:
                continue
            for i, (a, b) in enumerate(params):
                ptxt = src[ct[a][1]:ct[b][2]]
                ok = param_ok(src, ct, ptxt, q + 1, body_end)
                if ok is None:
                    continue
                arg_index = i - 1 if is_method else i
                key = name
                prev = found[key].get(arg_index)
                # the same name defined twice with different results: not safe
                found[key][arg_index] = ok if prev is None else (prev and ok)
                found[key].setdefault("_method", is_method)
    return found


def process(path, helpers):
    src = open(path, encoding="utf-8").read()
    ct = code(tokenize(src))
    closures = find_closures(src, ct)
    edits, n = [], 0
    for k, t in enumerate(ct):
        if t[0] != "ident" or k + 1 >= len(ct) or text_of(src, ct[k+1]) != "(":
            continue
        name = text_of(src, t)
        prev = text_of(src, ct[k-1]) if k else ""
        if prev == "fn":
            continue
        targets = []
        if name in closures and prev != "." and ct[k][1] > closures[name][1] - 10**9:
            targets = list(closures[name][0])
        elif name == "new" and k >= 3 and (text_of(src, ct[k-3]), "new") in EXTRA and prev == ":":
            targets = EXTRA[(text_of(src, ct[k-3]), "new")]
        elif name in helpers and name not in CALLS and name not in ("new",):
            h = helpers[name]
            if (prev == ".") != h.get("_method", False):
                continue
            targets = [i for i, ok in h.items() if i != "_method" and ok]
        if not targets:
            continue
        close = match_close(src, ct, k + 1)
        args = split_args(src, ct, k + 1, close)
        for i in targets:
            if i >= len(args):
                continue
            a0, a1 = args[i]
            expr = src[ct[a0][1]:ct[a1][2]]
            if a0 == a1 and ct[a0][0] == "str" and is_texty(expr):
                edits.append((ct[a0][1], ct[a1][2], "crate::i18n::tr(%s)" % expr)); n += 1
            elif text_of(src, ct[a0]) == "format" and a0 + 2 <= a1 and text_of(src, ct[a0+1]) == "!":
                c2 = match_close(src, ct, a0 + 2)
                if c2 == a1:
                    r = convert_format(src, ct, a0, c2)
                    if r:
                        edits.append((ct[a0][1], ct[a1][2], r)); n += 1
            elif text_of(src, ct[a0]) == "&" and a0 + 3 <= a1 and text_of(src, ct[a0+1]) == "format":
                c2 = match_close(src, ct, a0 + 3)
                if c2 == a1:
                    r = convert_format(src, ct, a0 + 1, c2)
                    if r:
                        edits.append((ct[a0][1], ct[a1][2], "&" + r)); n += 1
    out = src
    for s, e, r in sorted(edits, reverse=True):
        out = out[:s] + r + out[e:]
    if out != src:
        open(path, "w", encoding="utf-8").write(out)
    return n


if __name__ == "__main__":
    files = sys.argv[1:]
    helpers = find_helpers(files)
    shown = {k: [i for i, ok in v.items() if i != "_method" and ok] for k, v in helpers.items()}
    shown = {k: v for k, v in shown.items() if v}
    print("helpers that only show their text:", len(shown))
    for k in sorted(shown):
        print("  ", k, shown[k])
    for p in files:
        src = open(p, encoding="utf-8").read()
        for name, (res, _) in find_closures(src, code(tokenize(src))).items():
            print("   closure", p.split("/")[-1], name, sorted(res))
    tot = sum(process(p, helpers) for p in files)
    print("marked", tot)
