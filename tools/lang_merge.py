#!/usr/bin/env python3
"""Adds a batch of translations to a language file, after checking them.

    python3 tools/lang_merge.py de batch.json

batch.json maps English text (or its number in lang/en.json, counting from 0) to the translation.
A translation is refused if its {...} parts do not match the English. Prints what was refused.
"""
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HOLE = re.compile(r"\{\{|\}\}|\{[^{}]*\}")


def holes(s):
    return sorted(h for h in HOLE.findall(s) if h not in ("{{", "}}"))


def main():
    code, batch_path = sys.argv[1], sys.argv[2]
    en = json.load(open(os.path.join(ROOT, "lang", "en.json"), encoding="utf-8"))
    keys = [k for k in en if not k.startswith("_")]
    path = os.path.join(ROOT, "lang", code + ".json")
    cur = json.load(open(path, encoding="utf-8")) if os.path.exists(path) else {}
    batch = json.load(open(batch_path, encoding="utf-8"))
    added, bad = 0, []
    for k, v in batch.items():
        key = keys[int(k)] if k.isdigit() and k not in en else k
        if key not in en:
            bad.append((k, "not in en.json"))
            continue
        if not isinstance(v, str) or not v.strip():
            continue
        if holes(key) != holes(v):
            bad.append((key, f"parts differ: {holes(key)} vs {holes(v)}"))
            continue
        # keep leading/trailing spaces the same as the English (they join pieces of text)
        lead = key[: len(key) - len(key.lstrip())]
        trail = key[len(key.rstrip()):]
        cur[key] = lead + v.strip() + trail
        added += 1
    ordered = {k: cur[k] for k in keys if k in cur}
    ordered.update({k: v for k, v in cur.items() if k not in ordered})
    with open(path, "w", encoding="utf-8") as f:
        json.dump(ordered, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(f"{code}: added {added}, now {len(ordered)}/{len(keys)}")
    for k, why in bad:
        print("  refused:", json.dumps(k, ensure_ascii=False)[:80], "-", why)


if __name__ == "__main__":
    main()
