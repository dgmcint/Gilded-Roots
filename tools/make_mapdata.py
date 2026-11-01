#!/usr/bin/env python3
"""Builds the offline map data that is compiled into Gilded Roots (assets/map/*.z).

Inputs (download once into a folder and pass it as the first argument):
  ne_50m_admin_0_countries.geojson        Natural Earth, public domain
  ne_50m_admin_1_states_provinces.geojson Natural Earth, public domain
  ne_10m_admin_1_states_provinces.geojson Natural Earth, public domain (names and codes only)
  cities.json                              github.com/lutangar/cities.json, from GeoNames (CC BY 4.0)

Outputs (zlib-compressed):
  land.z    country shapes as triangles (to fill) and rings (to outline)
  states.z  state and province outlines
  places.z  tab-separated tables: countries, states/provinces, towns
"""
import json, struct, sys, zlib, os, unicodedata
import numpy as np
import mapbox_earcut as earcut

src = sys.argv[1]
out = sys.argv[2]
os.makedirs(out, exist_ok=True)

def q(v):
    return int(round(v * 100))

def rings_of(geom):
    if geom is None:
        return []
    if geom['type'] == 'Polygon':
        return [geom['coordinates']]
    if geom['type'] == 'MultiPolygon':
        return geom['coordinates']
    return []

def thin(ring, step=0.02):
    """Drop points closer than `step` degrees to the last kept one (keeps shapes, saves space)."""
    keep = [ring[0]]
    for p in ring[1:]:
        if abs(p[0] - keep[-1][0]) + abs(p[1] - keep[-1][1]) >= step:
            keep.append(p)
    if keep[-1] != ring[-1]:
        keep.append(ring[-1])
    return keep

# ---- land: triangles and outlines --------------------------------------------------------
countries = json.load(open(os.path.join(src, 'ne_50m_admin_0_countries.geojson')))
verts = []      # (lon, lat) quantised
tris = []       # indices into verts
rings = []      # (start, len) into verts
for f in countries['features']:
    for poly in rings_of(f['geometry']):
        poly = [thin(r) for r in poly if len(r) >= 4]
        if not poly or len(poly[0]) < 4:
            continue
        flat = []
        ends = []
        for r in poly:
            flat.extend(r)
            ends.append(len(flat))
        arr = np.array([[p[0], p[1]] for p in flat], dtype=np.float64)
        idx = earcut.triangulate_float64(arr, np.array(ends, dtype=np.uint32))
        base = len(verts)
        for p in flat:
            verts.append((q(p[0]), q(p[1])))
        tris.extend(int(i) + base for i in idx)
        # outline rings
        start = base
        for e in ends:
            rings.append((start, base + e - start))
            start = base + e
b = bytearray()
b += struct.pack('<I', len(verts))
for x, y in verts:
    b += struct.pack('<hh', x, y)
b += struct.pack('<I', len(tris))
for t in tris:
    b += struct.pack('<I', t)
b += struct.pack('<I', len(rings))
for s, n in rings:
    b += struct.pack('<II', s, n)
open(os.path.join(out, 'land.z'), 'wb').write(zlib.compress(bytes(b), 9))
print('land', len(verts), 'points', len(tris) // 3, 'triangles', len(rings), 'rings')

# ---- state and province outlines -------------------------------------------------------
states = json.load(open(os.path.join(src, 'ne_50m_admin_1_states_provinces.geojson')))
b = bytearray()
srings = []
for f in states['features']:
    for poly in rings_of(f['geometry']):
        for r in poly:
            r = thin(r, 0.03)
            if len(r) >= 3:
                srings.append(r)
b += struct.pack('<I', len(srings))
for r in srings:
    b += struct.pack('<I', len(r))
    for p in r:
        b += struct.pack('<hh', q(p[0]), q(p[1]))
open(os.path.join(out, 'states.z'), 'wb').write(zlib.compress(bytes(b), 9))
print('states', len(srings), 'rings')

# ---- place names -----------------------------------------------------------------------
def clean(s):
    return (s or '').replace('\t', ' ').replace('\n', ' ').strip()

lines = []
for f in countries['features']:
    p = f['properties']
    cc = p.get('ISO_A2_EH') or p.get('ISO_A2')
    if not cc or cc == '-99':
        continue
    names = {clean(p.get(k)) for k in ('NAME', 'NAME_LONG', 'FORMAL_EN', 'ADMIN', 'NAME_EN', 'ISO_A3') if p.get(k)}
    lines.append('C\t%s\t%s\t%.2f\t%.2f' % (cc, '|'.join(sorted(n for n in names if n)), p.get('LABEL_Y', 0), p.get('LABEL_X', 0)))

a1 = json.load(open(os.path.join(src, 'ne_10m_admin_1_states_provinces.geojson')))
for f in a1['features']:
    p = f['properties']
    code = p.get('gn_a1_code') or ''
    if '.' not in code:
        continue
    names = {clean(p.get('name')), clean(p.get('name_en'))}
    for alt in clean(p.get('name_alt')).split('|'):
        names.add(alt.strip())
    if p.get('postal'):
        names.add(clean(p.get('postal')))
    names = {n for n in names if n}
    lines.append('S\t%s\t%s\t%.2f\t%.2f' % (code, '|'.join(sorted(names)), p.get('latitude') or 0, p.get('longitude') or 0))

cities = json.load(open(os.path.join(src, 'cities.json')))
for c in cities:
    try:
        lat, lon = float(c['lat']), float(c['lng'])
    except Exception:
        continue
    lines.append('T\t%s\t%s\t%s\t%.3f\t%.3f' % (clean(c['name']), c['country'], c.get('admin1', ''), lat, lon))
data = '\n'.join(lines).encode('utf-8')
open(os.path.join(out, 'places.z'), 'wb').write(zlib.compress(data, 9))
print('places', len(lines), 'lines', len(data) // 1024, 'KB raw')
