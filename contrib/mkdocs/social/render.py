"""Render the link-preview card, `card.svg`, to `doc/public/assets/social-card.png`.

SVG renderers pick fonts from the system and fall back silently, so this
draws every `<text>` as glyph outlines first: `hb-shape` (HarfBuzz) shapes
the string, kerning included, and fontTools draws the glyphs from the font
files in FONTDIR. `rsvg-convert` then rasterises the result.

usage: python3 contrib/mkdocs/social/render.py FONTDIR

FONTDIR holds the files named in FONTS: Inter 4 (`extras/ttf/` of its
release) and JetBrains Mono 2 (`fonts/ttf/`). Needs `hb-shape`,
`rsvg-convert` and fontTools.
"""
import json
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

HERE = Path(__file__).parent
CARD = HERE / "card.svg"
PNG = HERE.parents[2] / "doc/public/assets/social-card.png"

SVG = "http://www.w3.org/2000/svg"
FONTS = {
    ("Inter", "800"): "Inter-ExtraBold.ttf",
    ("Inter", "600"): "Inter-SemiBold.ttf",
    ("Inter", "500"): "Inter-Medium.ttf",
    ("JetBrains Mono", "600"): "JetBrainsMono-SemiBold.ttf",
    ("JetBrains Mono", "500"): "JetBrainsMono-Medium.ttf",
}
INHERITED = ("font-family", "font-weight", "font-size", "letter-spacing", "fill",
             "fill-opacity", "text-anchor")


def outline(text, attrs, fontdir, fonts):
    """The path that draws `text` in place of the `<text>` element."""
    path = str(fontdir / FONTS[(attrs["font-family"], attrs["font-weight"])])
    if path not in fonts:
        fonts[path] = TTFont(path)
    tt = fonts[path]
    scale = float(attrs["font-size"]) / tt["head"].unitsPerEm
    spacing = float(attrs.get("letter-spacing", 0))
    shaped = json.loads(subprocess.run(
        ["hb-shape", "--output-format=json", "--no-glyph-names", path, text.text],
        capture_output=True, text=True, check=True).stdout)
    order = tt.getGlyphOrder()
    glyphs = tt.getGlyphSet()
    x = float(text.get("x"))
    if attrs.get("text-anchor") == "end":
        x -= sum(g["ax"] * scale + spacing for g in shaped) - spacing
    y = float(text.get("y"))
    pen = SVGPathPen(glyphs)
    for g in shaped:
        at = (scale, 0, 0, -scale, x + g["dx"] * scale, y - g["dy"] * scale)
        glyphs[order[g["g"]]].draw(TransformPen(pen, at))
        x += g["ax"] * scale + spacing
    out = ET.Element(f"{{{SVG}}}path", {"d": pen.getCommands()})
    for k in ("fill", "fill-opacity"):
        if k in text.attrib:
            out.set(k, text.get(k))
    return out


def walk(node, inherited, fontdir, fonts):
    for i, child in enumerate(list(node)):
        attrs = dict(inherited)
        attrs.update({k: v for k, v in child.attrib.items() if k in INHERITED})
        if child.tag == f"{{{SVG}}}text":
            node[i] = outline(child, attrs, fontdir, fonts)
        else:
            walk(child, attrs, fontdir, fonts)


def main():
    fontdir = Path(sys.argv[1])
    ET.register_namespace("", SVG)
    tree = ET.parse(CARD)
    walk(tree.getroot(), {}, fontdir, {})
    with tempfile.NamedTemporaryFile("w", suffix=".svg") as outlined:
        tree.write(outlined, encoding="unicode")
        outlined.flush()
        subprocess.run(["rsvg-convert", "-w", "1200", "-h", "630", outlined.name,
                        "-o", str(PNG)], check=True)


if __name__ == "__main__":
    main()
