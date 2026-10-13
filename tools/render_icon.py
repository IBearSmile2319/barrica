# Icono de Barrica: un barril de roble sobre un squircle color vino.
#   python3 tools/render_icon.py   →  assets/icon.png y assets/Barrica.icns
import math
import subprocess
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

SS = 4  # supersampling
N = 1024 * SS


def px(v):
    return v * SS


def mix(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def hexrgb(h):
    return tuple(int(h[i : i + 2], 16) for i in (1, 3, 5))


def vgradient(top, bottom):
    g = Image.new("RGB", (1, 256))
    for y in range(256):
        g.putpixel((0, y), mix(top, bottom, y / 255))
    return g.resize((N, N))


def main():
    out = Path(__file__).resolve().parent.parent / "assets"
    out.mkdir(exist_ok=True)
    icon = Image.new("RGBA", (N, N), (0, 0, 0, 0))

    # Squircle de macOS (824 px, radio 185) con sombra suave.
    box = [px(100), px(100), px(924), px(924)]
    shadow = Image.new("L", (N, N), 0)
    ImageDraw.Draw(shadow).rounded_rectangle([box[0], box[1] + px(12), box[2], box[3] + px(12)], px(185), fill=110)
    icon.paste((0, 0, 0, 255), mask=shadow.filter(ImageFilter.GaussianBlur(px(14))))
    body = Image.new("L", (N, N), 0)
    ImageDraw.Draw(body).rounded_rectangle(box, px(185), fill=255)
    icon.paste(vgradient(hexrgb("#9C2149"), hexrgb("#3A0818")), mask=body)
    glow = Image.new("L", (N, N), 0)  # brillo arriba
    ImageDraw.Draw(glow).ellipse([px(180), px(40), px(844), px(520)], fill=70)
    glow = Image.composite(glow.filter(ImageFilter.GaussianBlur(px(60))), Image.new("L", (N, N), 0), body)
    icon.paste((255, 190, 210, 255), mask=glow)

    # Barril: medio ancho que se abomba hacia el centro.
    cx, top, H, W = px(512), px(268), px(500), px(236)
    hw = lambda t: W * (0.80 + 0.20 * math.sin(math.pi * t))
    ey = lambda t: 0.17 * hw(t)  # semieje vertical de la elipse (perspectiva desde arriba)
    yt = lambda t: top + t * H
    steps = 120
    left = [(cx - hw(t), yt(t)) for t in (i / steps for i in range(steps + 1))]
    right = [(cx + hw(t), yt(t)) for t in (i / steps for i in range(steps, -1, -1))]
    bottom = [(cx + hw(1) * math.cos(a), yt(1) + ey(1) * math.sin(a)) for a in (math.pi - math.pi * i / 60 for i in range(61))]
    front_top = [(cx + hw(0) * math.cos(a), yt(0) + ey(0) * math.sin(a)) for a in (math.pi * i / 60 for i in range(61))]
    silhouette = left + bottom + right + front_top

    # Sombra del barril sobre el fondo.
    drop = Image.new("L", (N, N), 0)
    ImageDraw.Draw(drop).ellipse([cx - px(250), yt(1) - px(10), cx + px(250), yt(1) + px(95)], fill=120)
    icon.paste((20, 0, 6, 255), mask=drop.filter(ImageFilter.GaussianBlur(px(22))))

    # Madera con sombreado cilíndrico (luz desde la izquierda).
    wood = Image.new("RGB", (N, 1))
    dark, light = hexrgb("#6E3A16"), hexrgb("#D99A55")
    for x in range(N):
        u = (x - cx) / W  # -1..1
        k = max(0.0, math.cos(min(abs(u + 0.28), 1.5)))
        wood.putpixel((x, 0), mix(dark, light, k**1.4))
    mask = Image.new("L", (N, N), 0)
    ImageDraw.Draw(mask).polygon(silhouette, fill=255)
    icon.paste(wood.resize((N, N)), mask=mask)

    d = ImageDraw.Draw(icon)
    # Duelas: líneas que siguen la panza.
    for u in (-0.72, -0.43, -0.14, 0.15, 0.44, 0.73):
        pts = [(cx + u * hw(t), yt(t) + ey(t) * math.sqrt(max(0, 1 - u * u))) for t in (i / steps for i in range(steps + 1))]
        d.line(pts, fill=hexrgb("#8A4D1E"), width=px(5), joint="curve")

    # Aros de hierro con brillo.
    for t in (0.13, 0.31, 0.69, 0.87):
        arc = [(cx + hw(t) * math.cos(a), yt(t) + ey(t) * math.sin(a)) for a in (math.pi * i / 80 for i in range(81))]
        d.line(arc, fill=hexrgb("#2B2427"), width=px(26), joint="curve")
        d.line([(x, y - px(8)) for x, y in arc[3:-3]], fill=hexrgb("#8C8186"), width=px(5), joint="curve")

    # Tapa en perspectiva.
    lid = [cx - hw(0), yt(0) - ey(0), cx + hw(0), yt(0) + ey(0)]
    d.ellipse(lid, fill=hexrgb("#5A2D10"))
    inset = px(18)
    d.ellipse([lid[0] + inset, lid[1] + inset * 0.35, lid[2] - inset, lid[3] - inset * 0.35], fill=hexrgb("#C9874A"))
    for f in (-0.33, 0.0, 0.33):  # tablas de la tapa
        y = yt(0) + f * ey(0)
        half = (hw(0) - inset) * math.sqrt(1 - f * f)
        d.line([(cx - half, y), (cx + half, y)], fill=hexrgb("#8A4D1E"), width=px(4))

    # Grifo de latón con una gota de vino.
    gy = yt(0.5)
    brass, brass_dark, brass_light = hexrgb("#D9A441"), hexrgb("#8F6418"), hexrgb("#F6D88A")
    d.rounded_rectangle([cx - px(48), gy - px(46), cx + px(48), gy - px(30)], px(8), fill=brass_dark)  # manija
    d.rounded_rectangle([cx - px(9), gy - px(36), cx + px(9), gy], px(4), fill=brass_dark)
    d.ellipse([cx - px(30), gy - px(18), cx + px(30), gy + px(22)], fill=brass_dark)  # brida
    d.ellipse([cx - px(24), gy - px(14), cx + px(24), gy + px(16)], fill=brass)
    d.rounded_rectangle([cx - px(12), gy, cx + px(12), gy + px(62)], px(8), fill=brass)  # caño
    d.rounded_rectangle([cx - px(6), gy + px(4), cx - px(1), gy + px(56)], px(3), fill=brass_light)
    drop_y = gy + px(78)
    d.polygon([(cx, drop_y - px(10)), (cx - px(10), drop_y + px(8)), (cx + px(10), drop_y + px(8))], fill=hexrgb("#B0123E"))
    d.ellipse([cx - px(11), drop_y, cx + px(11), drop_y + px(22)], fill=hexrgb("#B0123E"))

    final = icon.resize((1024, 1024), Image.LANCZOS)
    final.save(out / "icon.png")

    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp) / "Barrica.iconset"
        iconset.mkdir()
        for size in (16, 32, 128, 256, 512):
            final.resize((size, size), Image.LANCZOS).save(iconset / f"icon_{size}x{size}.png")
            final.resize((size * 2, size * 2), Image.LANCZOS).save(iconset / f"icon_{size}x{size}@2x.png")
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(out / "Barrica.icns")], check=True)
    print(out / "Barrica.icns")


if __name__ == "__main__":
    main()
