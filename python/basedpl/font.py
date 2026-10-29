"""Build BPL fonts with script glyphs, arrows and the token separator. Requires fonttools and brotli."""
import io
from copy import deepcopy
from pathlib import Path
from fastcore.net import urlread
from fontTools.ttLib import TTFont
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib.tables._g_l_y_f import GlyphCoordinates
from .assets import GENERATED

SAX2_URL = 'https://raw.githubusercontent.com/abrudz/SAX2/df113b78ef1c9bf49134371a54ce915f65f9f47e/'
BQN386_URL = 'https://raw.githubusercontent.com/dzaima/BQN386/4d8b9f668ba76a15ca9cd44d9bfedaf95a4c0d96/'

def add_glyph(font, char, glyph, advance):
    "Register a glyph with its character mapping and horizontal metrics."
    name, glyf = f'uni{ord(char):04X}', font['glyf']
    glyf[name] = glyph
    glyph.recalcBounds(glyf)
    font['hmtx'][name] = advance, glyph.xMin
    for table in font['cmap'].tables:
        if table.isUnicode(): table.cmap[ord(char)] = name
    font.setGlyphOrder(glyf.glyphOrder)

def add_scripts(font):
    "Add BPL's missing script glyphs without replacing existing glyphs."
    cmap, glyf = font.getBestCmap(), font['glyf']
    def add(char, source, scale, offset):
        if ord(char) in cmap: return
        src = cmap[ord(source)]
        pen = TTGlyphPen(font.getGlyphSet())
        pen.addComponent(src, (scale, 0, 0, scale, 0, offset))
        add_glyph(font, char, pen.glyph(), round(font['hmtx'][src][0]*scale))
    for char, source in zip('⁰¹²³⁴⁵⁶⁷⁸⁹⁻ᵇᵀᵘ', '0123456789-bTu'): add(char, source, 2/3, 392)
    for char, source in zip('₀₁₂₃₄₅₆₇₈₉₋', '⁰¹²³⁴⁵⁶⁷⁸⁹⁻'): add(char, source, 1, -532)
    for char, source in zip('ₓᵣₑⱼ', 'xrej'):
        if ord(char) in cmap: continue
        add(char, source, 2/3, -140)
        name = cmap[ord(char)]
        # Subscript letters occupy a full box-drawing cell.
        font['hmtx'][name] = font['hmtx'][cmap[ord(source)]][0], glyf[name].xMin

def add_arrows(font):
    "Add tailed arrows and a right arrow from bar using SAX2's right-arrow outline."
    cmap, glyf = font.getBestCmap(), font['glyf']
    src = cmap[ord('→')]
    right = deepcopy(glyf[src])
    right.removeHinting()
    points = list(right.coordinates)
    advance = font['hmtx'][src][0]
    if ord('↦') not in cmap:
        glyph = deepcopy(right)
        x, y, top = right.xMin, right.yMin, right.yMax
        # Points 18 and 24 bound the shaft's thickness in the pinned outline.
        edge = x + points[24][1] - points[18][1]
        glyph.coordinates.extend([(x, y), (x, top), (edge, top), (edge, y)])
        glyph.flags.extend([1]*4)
        glyph.endPtsOfContours.append(len(glyph.coordinates)-1)
        glyph.numberOfContours += 1
        add_glyph(font, '↦', glyph, advance)
    # In the pinned SAX2 outline, points 19–23 form the shaft's left end.
    head = points[:19] + points[24:]
    offset = right.xMin - min(x for x, y in head)
    join = points[18][0] + offset
    for i in range(19, 24): right.coordinates[i] = join, points[i][1]
    right.coordinates.extend((x+offset, y) for x, y in head)
    right.flags.extend(right.flags[:19] + right.flags[24:])
    right.endPtsOfContours.append(len(right.coordinates)-1)
    right.numberOfContours += 1
    for char, mirror in [('↣', False), ('↢', True)]:
        if ord(char) in cmap: continue
        glyph = deepcopy(right)
        if mirror: glyph.coordinates = GlyphCoordinates((advance-x, y) for x, y in glyph.coordinates)
        add_glyph(font, char, glyph, advance)

def thicken_slashes(font, amount=0.05):
    "Thicken the shared diagonal of / and ⌿ perpendicular to its slope."
    cmap, glyf = font.getBestCmap(), font['glyf']
    slash = glyf[cmap[ord('/')]]
    points = list(slash.coordinates)
    cx, cy = (slash.xMin + slash.xMax)/2, (slash.yMin + slash.yMax)/2
    # Points 1–2 form one straight edge of the diagonal in the pinned SAX2 outline.
    dx, dy = points[2][0] - points[1][0], points[2][1] - points[1][1]
    for char in '/⌿':
        name = cmap[ord(char)]
        glyph = glyf[name]
        glyph.removeHinting()
        for i, (x, y) in enumerate(points):
            shift = amount*((x-cx)*dy - (y-cy)*dx)/(dx*dx + dy*dy)
            glyph.coordinates[i] = round(x + shift*dy), round(y - shift*dx)
        glyph.recalcBounds(glyf)
        font['hmtx'][name] = font['hmtx'][name][0], glyph.xMin

def add_separator(font):
    "Add three half-size dots in the character cell, closer together than a colon."
    cmap, glyf = font.getBestCmap(), font['glyf']
    source = cmap[ord('.')]
    dot, colon = glyf[source], glyf[cmap[ord(':')]]
    advance = font['hmtx'][source][0]
    x = advance/2 - (dot.xMin + dot.xMax)/4
    y = (colon.yMin + colon.yMax)/2 - (dot.yMin + dot.yMax)/4
    gap = font['head'].unitsPerEm/8
    pen = TTGlyphPen(font.getGlyphSet())
    for offset in (-gap, 0, gap): pen.addComponent(source, (0.5, 0, 0, 0.5, round(x), round(y+offset)))
    add_glyph(font, '⁝', pen.glyph(), advance)

def write_font(font, name, version, web_dest):
    names = {1: name, 3: f'{name}-{version}', 4: name, 5: f'Version {version}', 6: name}
    for record in font['name'].names:
        if record.nameID in names:
            font['name'].setName(names[record.nameID], record.nameID, record.platformID, record.platEncID, record.langID)
    ttf, woff2 = GENERATED/f'fonts/{name}.ttf', web_dest/f'{name}.woff2'
    font.save(ttf)
    font.flavor = 'woff2'
    font.save(woff2)
    return ttf, woff2

def build():
    "Write installable fonts and licences to the package source, and webfonts to the browser assets and docs."
    web_dest = Path('nbs/fonts')
    web_dest.mkdir(parents=True, exist_ok=True)
    (GENERATED/'fonts').mkdir(parents=True, exist_ok=True)
    font = TTFont(io.BytesIO(urlread(SAX2_URL + 'SAX2.ttf', decode=False, timeout=30)), recalcTimestamp=False)
    add_scripts(font)
    add_arrows(font)
    thicken_slashes(font)
    add_separator(font)
    paths = write_font(font, 'SAX2B', '2.002', Path(__file__).with_name('browser'))
    font = TTFont(io.BytesIO(urlread(BQN386_URL + 'BQN386.ttf', decode=False, timeout=30)), recalcTimestamp=False)
    add_separator(font)
    paths += write_font(font, 'BQN386B', '1.001', web_dest)
    for name, url in [('SAX2', SAX2_URL), ('BQN386', BQN386_URL)]:
        (GENERATED/f'fonts/LICENSE-{name}').write_text(urlread(url + 'LICENSE', timeout=30))
    return paths
