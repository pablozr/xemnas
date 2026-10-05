"""Subsets the app's fonts for the exported architecture page and writes them
as base64 text (`crates/application/src/page/fonts/*.b64`), so the page stays a
single offline file. Run after changing the fonts or the characters below.

    python tools/page-fonts.py

Needs fontTools (`pip install fonttools`). The fonts are OFL-licensed; their
licenses sit next to the sources in `apps/desktop-gpui/assets/fonts`.
"""
import base64
import io
import os

from fontTools import subset
from fontTools.ttLib import TTFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FONTS = os.path.join(ROOT, 'apps', 'desktop-gpui', 'assets', 'fonts')
OUT = os.path.join(ROOT, 'crates', 'application', 'src', 'page', 'fonts')

LATIN = list(range(0x20, 0x7F)) + list(range(0xA0, 0x100))
PUNCT = [0x2013, 0x2014, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2026, 0x2190, 0x2192, 0x2191, 0x2193, 0x00B7, 0x2318]
JOBS = [
    ('inter', 'InterVariable.ttf', LATIN + PUNCT),
    ('mono', 'JetBrainsMono-VariableFont_wght.ttf', list(range(0x20, 0x7F)) + [0xB7, 0x2026, 0x2192]),
    ('display', 'BricolageGrotesque-Variable.ttf', LATIN + PUNCT),
]

os.makedirs(OUT, exist_ok=True)
for name, source, codes in JOBS:
    options = subset.Options()
    options.flavor = 'woff'
    options.layout_features = ['kern', 'liga', 'calt', 'ccmp', 'locl', 'mark', 'mkmk', 'tnum', 'cv11', 'ss01']
    options.drop_tables += ['DSIG']
    options.notdef_outline = True
    font = TTFont(os.path.join(FONTS, source))
    subsetter = subset.Subsetter(options)
    subsetter.populate(unicodes=codes)
    subsetter.subset(font)
    font.flavor = 'woff'
    buffer = io.BytesIO()
    font.save(buffer)
    data = buffer.getvalue()
    with open(os.path.join(OUT, name + '.woff.b64'), 'w', newline='\n') as f:
        f.write(base64.b64encode(data).decode())
    print(name, len(data) // 1024, 'KB')
