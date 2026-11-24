#!/usr/bin/env python3
"""Second groundwork pass: marks status-bar and notice messages where they are set
(self.status = "...".to_string(), self.status = format!(...), self.notice = Some(...)).
Run from the project folder:  python3 tools/lang_mark_status.py src/*.rs
"""
import sys
from lang_mark import tokenize, code, match_close, convert_format, is_texty

FIELDS = {"status", "notice"}


def process(path):
    src = open(path, encoding="utf-8").read()
    ct = code(tokenize(src))
    edits, n = [], 0
    for k in range(len(ct) - 4):
        if src[ct[k][1]:ct[k][2]] != "self" or src[ct[k+1][1]] != ".":
            continue
        if src[ct[k+2][1]:ct[k+2][2]] not in FIELDS or src[ct[k+3][1]:ct[k+3][2]] != "=" or src[ct[k+4][1]] == "=":
            continue
        j = k + 4
        # Some( ... )
        if src[ct[j][1]:ct[j][2]] == "Some" and src[ct[j+1][1]] == "(":
            j += 2
        t = ct[j]
        if t[0] == "str" and is_texty(src[t[1]:t[2]]) and j + 2 < len(ct) and src[ct[j+1][1]] == "." \
                and src[ct[j+2][1]:ct[j+2][2]] in ("to_string", "into", "to_owned"):
            edits.append((t[1], t[2], "crate::i18n::tr(%s)" % src[t[1]:t[2]])); n += 1
        elif src[t[1]:t[2]] == "String" and src[ct[j+3][1]:ct[j+3][2]] == "from" and ct[j+5][0] == "str" \
                and is_texty(src[ct[j+5][1]:ct[j+5][2]]):
            s5 = ct[j+5]
            edits.append((s5[1], s5[2], "crate::i18n::tr(%s)" % src[s5[1]:s5[2]])); n += 1
        elif src[t[1]:t[2]] == "format" and src[ct[j+1][1]] == "!" and src[ct[j+2][1]] == "(":
            close = match_close(src, ct, j + 2)
            r = convert_format(src, ct, j, close)
            if r:
                edits.append((t[1], ct[close][2], r)); n += 1
    out = src
    for s, e, r in sorted(edits, reverse=True):
        out = out[:s] + r + out[e:]
    if out != src:
        open(path, "w", encoding="utf-8").write(out)
    return n


if __name__ == "__main__":
    tot = 0
    for p in sys.argv[1:]:
        tot += process(p)
    print("marked", tot)
