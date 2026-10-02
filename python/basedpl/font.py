"""Build SAX2B with BPL's superscripts and subscripts. Requires fonttools and brotli."""
import io
from pathlib import Path
from fastcore.net import urlread
from fontTools.ttLib import TTFont
from fontTools.pens.ttGlyphPen import TTGlyphPen

SAX2_URL = 'https://raw.githubusercontent.com/abrudz/SAX2/df113b78ef1c9bf49134371a54ce915f65f9f47e/'

def add_scripts(font):
    "Add BPL's missing script glyphs without replacing existing glyphs."
    cmap, glyf = font.getBestCmap(), font['glyf']
    def add(char, source, scale, offset):
        if ord(char) in cmap: return
        name, src = f'uni{ord(char):04X}', cmap[ord(source)]
        pen = TTGlyphPen(font.getGlyphSet())
        pen.addComponent(src, (scale, 0, 0, scale, 0, offset))
        glyph = glyf[name] = pen.glyph()
        glyph.recalcBounds(glyf)
        font['hmtx'][name] = round(font['hmtx'][src][0]*scale), glyph.xMin
        for table in font['cmap'].tables:
            if table.isUnicode(): table.cmap[ord(char)] = name
    for char, source in zip('⁰¹²³⁴⁵⁶⁷⁸⁹⁻ᵀᵘ', '0123456789-Tu'): add(char, source, 2/3, 392)
    for char, source in zip('₀₁₂₃₄₅₆₇₈₉₋', '⁰¹²³⁴⁵⁶⁷⁸⁹⁻'): add(char, source, 1, -532)
    if ord('ₓ') not in cmap:
        add('ₓ', 'x', 2/3, -140)
        name = cmap[ord('ₓ')]
        # The integer box marker occupies a full character cell.
        font['hmtx'][name] = font['hmtx'][cmap[ord('x')]][0], glyf[name].xMin
    font.setGlyphOrder(glyf.glyphOrder)

def build(dest='nbs/fonts'):
    "Write SAX2B as TTF and WOFF2 with SAX2's licence to `dest`."
    dest = Path(dest)
    dest.mkdir(parents=True, exist_ok=True)
    font = TTFont(io.BytesIO(urlread(SAX2_URL + 'SAX2.ttf', decode=False, timeout=30)))
    add_scripts(font)
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
