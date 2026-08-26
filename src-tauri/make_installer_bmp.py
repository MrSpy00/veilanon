"""Generate high-DPI, crystal-clear NSIS installer header and sidebar bitmaps from brand logo.

Features:
1. 4x supersampling with Lanczos downsampling for ultra-crisp, anti-aliased graphics and typography.
2. Seamless alpha extraction of the glowing purple 'veilanon' emblem to eliminate harsh background borders.
3. Full Turkish character support (Segoe UI) for slogans ('Gizlilik · Hız · Özgürlük', 'Uçtan Uca Şifreli İletişim').
4. Prominent, elegant aegisSoft developer branding at the bottom.
5. Clean right-aligned header badge seamlessly matching the white NSIS header bar without text clipping.
"""

from PIL import Image, ImageDraw, ImageFont
import numpy as np
import pathlib

BASE = pathlib.Path(__file__).parent  # src-tauri
LOGO = BASE.parent / "brand" / "png" / "veilanon-1024x1024.png"
INSTALLER = BASE / "installer"

SCALE = 4  # 4x supersampling

def get_font(size: int, bold: bool = True) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    font_files = [
        "C:/Windows/Fonts/segoeuib.ttf" if bold else "C:/Windows/Fonts/segoeui.ttf",
        "C:/Windows/Fonts/arialbd.ttf" if bold else "C:/Windows/Fonts/arial.ttf",
    ]
    for fp in font_files:
        p = pathlib.Path(fp)
        if p.exists():
            try:
                return ImageFont.truetype(str(p), size * SCALE)
            except OSError:
                pass
    return ImageFont.load_default()


def extract_emblem() -> Image.Image:
    """Extracts the stylized glowing veilanon emblem with smooth alpha transparency."""
    raw = Image.open(LOGO).convert("RGB")
    arr = np.array(raw, dtype=float)

    # Crop wordmark region (y: 360..680, x: 80..944)
    cropped = arr[360:680, 80:944]
    bright = np.max(cropped, axis=2)

    # Smooth alpha mask: 0 below noise floor (13), smoothly ramp up to 255
    alpha = np.clip((bright - 13.0) / (75.0 - 13.0), 0.0, 1.0)
    alpha = np.power(alpha, 0.85) * 255.0

    rgba = np.dstack([cropped, alpha]).astype(np.uint8)
    return Image.fromarray(rgba, "RGBA")


def draw_center_text(draw, y_pos, width, text, fnt, fill, line_gap=0):
    bbox = draw.textbbox((0, 0), text, font=fnt)
    tw = bbox[2] - bbox[0]
    th = bbox[3] - bbox[1]
    x = (width - tw) // 2 - bbox[0]
    draw.text((x, y_pos - bbox[1]), text, font=fnt, fill=fill)
    return y_pos + th + line_gap


def make_sidebar():
    SW, SH = 164 * SCALE, 314 * SCALE
    emblem = extract_emblem()

    # Premium dark violet gradient: (15, 16, 23) -> (28, 22, 46)
    top_c = np.array([15, 16, 23], dtype=float)
    bot_c = np.array([28, 22, 46], dtype=float)
    canvas = np.zeros((SH, SW, 3), dtype=np.uint8)
    for y in range(SH):
        t = y / (SH - 1)
        canvas[y, :] = (top_c + (bot_c - top_c) * t).astype(np.uint8)

    sidebar = Image.fromarray(canvas, "RGB")

    # Place seamlessly blended emblem
    logo_w = int(SW * 0.84)
    logo_h = int(emblem.height * (logo_w / emblem.width))
    logo_resized = emblem.resize((logo_w, logo_h), Image.LANCZOS)
    logo_x = (SW - logo_w) // 2
    logo_y = int(36 * SCALE)
    sidebar.paste(logo_resized, (logo_x, logo_y), logo_resized)

    draw = ImageDraw.Draw(sidebar)

    # Sleek accent divider bar
    div_y = logo_y + logo_h + int(12 * SCALE)
    div_w = int(36 * SCALE)
    draw.rounded_rectangle(
        [(SW // 2 - div_w, div_y), (SW // 2 + div_w, div_y + int(2 * SCALE))],
        radius=int(1 * SCALE),
        fill=(157, 122, 232),
    )

    # Typography with full Turkish characters
    f_title = get_font(23, bold=True)
    f_tagline = get_font(11, bold=False)
    f_feature = get_font(9, bold=False)
    f_aegis = get_font(16, bold=True)
    f_sub = get_font(9, bold=False)

    y = div_y + int(14 * SCALE)
    y = draw_center_text(draw, y, SW, "veilanon", f_title, (244, 245, 252), int(4 * SCALE))
    y = draw_center_text(draw, y, SW, "Gizlilik  ·  Hız  ·  Özgürlük", f_tagline, (185, 190, 205), int(10 * SCALE))
    y = draw_center_text(draw, y, SW, "Uçtan Uca Şifreli İletişim", f_feature, (140, 145, 165), int(6 * SCALE))

    # Prominent bottom branding: aegisSoft (larger & clearer)
    by = SH - int(56 * SCALE)
    draw_center_text(draw, by, SW, "aegisSoft", f_aegis, (170, 138, 248), int(2 * SCALE))
    draw_center_text(draw, by + int(21 * SCALE), SW, "www.aegissoft.com.tr", f_sub, (135, 140, 155), 0)

    # Downsample with Lanczos for ultra-crisp finish
    final_sidebar = sidebar.resize((164, 314), Image.LANCZOS)
    final_sidebar.save(INSTALLER / "sidebar.bmp", "BMP")
    print(f"sidebar.bmp successfully generated ({final_sidebar.size[0]}x{final_sidebar.size[1]})")


def make_header():
    HW, HH = 150 * SCALE, 57 * SCALE
    emblem = extract_emblem()

    # Clean white background matching NSIS header area
    header = Image.new("RGB", (HW, HH), (255, 255, 255))

    # Create a sleek dark squircle brand badge on the right side
    badge_w = int(140 * SCALE)
    badge_h = int(45 * SCALE)
    badge_x = HW - badge_w - int(8 * SCALE)
    badge_y = (HH - badge_h) // 2

    badge_img = Image.new("RGBA", (badge_w, badge_h), (0, 0, 0, 0))
    b_draw = ImageDraw.Draw(badge_img)

    # Dark rounded background with soft brand purple border
    b_draw.rounded_rectangle(
        [(0, 0), (badge_w - 1, badge_h - 1)],
        radius=int(8 * SCALE),
        fill=(18, 17, 28, 255),
        outline=(157, 122, 232, 220),
        width=int(1.5 * SCALE),
    )

    # Place emblem inside badge
    h_logo_w = int(132 * SCALE)
    h_logo_h = int(emblem.height * (h_logo_w / emblem.width))
    h_logo_resized = emblem.resize((h_logo_w, h_logo_h), Image.LANCZOS)

    b_logo_x = (badge_w - h_logo_w) // 2
    b_logo_y = (badge_h - h_logo_h) // 2
    badge_img.paste(h_logo_resized, (b_logo_x, b_logo_y), h_logo_resized)

    header.paste(badge_img, (badge_x, badge_y), badge_img)

    # Downsample with Lanczos
    final_header = header.resize((150, 57), Image.LANCZOS)
    final_header.save(INSTALLER / "header.bmp", "BMP")
    print(f"header.bmp successfully generated ({final_header.size[0]}x{final_header.size[1]})")


if __name__ == "__main__":
    make_sidebar()
    make_header()
