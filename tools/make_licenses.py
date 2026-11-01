#!/usr/bin/env python3
"""Regenerates THIRD_PARTY_LICENSES.txt from Cargo.lock (run from the project folder)."""
import json, os, glob, hashlib, subprocess
meta = json.loads(subprocess.run(['cargo', 'metadata', '--format-version', '1'], capture_output=True, text=True).stdout)
pk = sorted((p for p in meta['packages'] if p['name'] != 'gilded-roots'), key=lambda p: (p['name'].lower(), p['version']))
texts, listing = {}, []
for p in pk:
    root = os.path.dirname(p['manifest_path'])
    for f in sorted(glob.glob(os.path.join(root, '*'))):
        b = os.path.basename(f).upper()
        if os.path.isfile(f) and b.startswith(('LICENSE', 'LICENCE', 'COPYING', 'NOTICE', 'UNLICENSE')):
            t = open(f, encoding='utf-8', errors='replace').read().strip()
            if t:
                h = hashlib.sha1(t.encode()).hexdigest()
                texts.setdefault(h, [t, os.path.basename(f), []])[2].append(f"{p['name']} {p['version']}")
    listing.append(f"{p['name']} {p['version']}  -  {p.get('license') or 'see repository'}" + (f"  -  {p['repository']}" if p.get('repository') else ''))
apache = next((open(f, errors='replace').read().strip() for f in glob.glob(os.path.expanduser('~/.cargo/registry/src/*/*/LICENSE-APACHE')) if 'TERMS AND CONDITIONS' in open(f, errors='replace').read()), '')
mit = open(os.path.join(os.path.dirname(__file__), 'mit.txt')).read().strip() if os.path.exists(os.path.join(os.path.dirname(__file__), 'mit.txt')) else ''
L = "=" * 78
out = ["GILDED ROOTS - THIRD-PARTY SOFTWARE NOTICES\n",
       "Gilded Roots is built with the open-source libraries and data listed below, each used under its own licence.",
       "All of them are permissive (MIT, Apache-2.0, BSD, Zlib, ISC, Unicode, Boost, OFL for fonts and similar).\n",
       L, "MAP DATA", L,
       "Country, state and province outlines: Natural Earth (naturalearthdata.com). Public domain.",
       "Town names and positions: GeoNames (www.geonames.org), via github.com/lutangar/cities.json.",
       "  Licensed under Creative Commons Attribution 4.0 (CC BY 4.0): https://creativecommons.org/licenses/by/4.0/",
       "  The data was reduced to the fields the map needs (name, country, state code, position).\n",
       L, "LIBRARIES", L] + listing + ["", L, "LICENCE TEXTS", L]
for t, fn, names in sorted(texts.values(), key=lambda x: x[2][0].lower()):
    shown = ", ".join(names[:12]) + (f" and {len(names) - 12} more" if len(names) > 12 else "")
    out += ["", "-" * 78, f"{fn} - used by: {shown}", "-" * 78, t]
out += ["", L, "STANDARD LICENCE TEXTS", L,
        "Some libraries state their licence without shipping a separate file. For those, the standard texts apply,",
        "with copyright held by the library's authors (see each library's repository).", ""]
if mit:
    out += ["-" * 78, "MIT License (standard text)", "-" * 78, mit, ""]
out += ["-" * 78, "Apache License 2.0 (standard text)", "-" * 78, apache, ""]
open('THIRD_PARTY_LICENSES.txt', 'w').write("\n".join(out) + "\n")
print(len(pk), 'packages')
