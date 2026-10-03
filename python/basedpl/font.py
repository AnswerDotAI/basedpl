"""Build SAX2B with BPL's script glyphs and tailed arrows. Requires fonttools and brotli."""
import io
from copy import deepcopy
from pathlib import Path
from fastcore.net import urlread
from fontTools.ttLib import TTFont
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib.tables._g_l_y_f import GlyphCoordinates

SAX2_URL = 'https://raw.githubusercontent.com/abrudz/SAX2/df113b78ef1c9bf49134371a54ce915f65f9f47e/'

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
    for char, source in zip('⁰¹²³⁴⁵⁶⁷⁸⁹⁻ᵀᵘ', '0123456789-Tu'): add(char, source, 2/3, 392)
    for char, source in zip('₀₁₂₃₄₅₆₇₈₉₋', '⁰¹²³⁴⁵⁶⁷⁸⁹⁻'): add(char, source, 1, -532)
    for char, source in zip('ₓᵣₑⱼ', 'xrej'):
        if ord(char) in cmap: continue
        add(char, source, 2/3, -140)
        name = cmap[ord(char)]
        # Subscript letters occupy a full box-drawing cell.
        font['hmtx'][name] = font['hmtx'][cmap[ord(source)]][0], glyf[name].xMin

def add_tailed_arrows(font):
    "Add right and left tailed arrows using SAX2's right-arrow outline."
    cmap, glyf = font.getBestCmap(), font['glyf']
    src = cmap[ord('→')]
    right = deepcopy(glyf[src])
    right.removeHinting()
    points = list(right.coordinates)
    # In the pinned SAX2 outline, points 19–23 form the shaft's left end.
    head = points[:19] + points[24:]
    offset = right.xMin - min(x for x, y in head)
    join = points[18][0] + offset
    for i in range(19, 24): right.coordinates[i] = join, points[i][1]
    right.coordinates.extend((x+offset, y) for x, y in head)
    right.flags.extend(right.flags[:19] + right.flags[24:])
    right.endPtsOfContours.append(len(right.coordinates)-1)
    right.numberOfContours += 1
    advance = font['hmtx'][src][0]
    for char, mirror in [('↣', False), ('↢', True)]:
        if ord(char) in cmap: continue
        glyph = deepcopy(right)
        if mirror: glyph.coordinates = GlyphCoordinates((advance-x, y) for x, y in glyph.coordinates)
        add_glyph(font, char, glyph, advance)

def build(dest='nbs/fonts'):
    "Write SAX2B as TTF and WOFF2 with SAX2's licence to `dest`."
    dest = Path(dest)
    dest.mkdir(parents=True, exist_ok=True)
    font = TTFont(io.BytesIO(urlread(SAX2_URL + 'SAX2.ttf', decode=False, timeout=30)))
    add_scripts(font)
    add_tailed_arrows(font)
    names = {1: 'SAX2B', 3: 'SAX2B-2.001', 4: 'SAX2B', 6: 'SAX2B'}
    for record in font['name'].names:
        if record.nameID in names:
            font['name'].setName(names[record.nameID], record.nameID, record.platformID, record.platEncID, record.langID)
    ttf, woff2 = dest/'SAX2B.ttf', dest/'SAX2B.woff2'
    font.save(ttf)
    font.flavor = 'woff2'
    font.save(woff2)
    (dest/'LICENSE-SAX2').write_text(urlread(SAX2_URL + 'LICENSE', timeout=30))
    return ttf, woff2
