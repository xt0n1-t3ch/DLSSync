from pathlib import Path

from PIL import Image, ImageDraw, ImageEnhance, ImageFilter, ImageFont


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / ".github/assets/nexus/source/dlssync-v170-cover-background-native.png"
ICON = ROOT / "src-tauri/icons/app-icon-1024.png"
FONT_REGULAR = Path("C:/Windows/Fonts/segoeui.ttf")
FONT_BOLD = Path("C:/Windows/Fonts/segoeuib.ttf")


def font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(str(FONT_BOLD if bold else FONT_REGULAR), size)


def rounded_panel(size: tuple[int, int], radius: int, fill: tuple[int, int, int, int]) -> Image.Image:
    panel = Image.new("RGBA", size, (0, 0, 0, 0))
    ImageDraw.Draw(panel).rounded_rectangle((0, 0, size[0] - 1, size[1] - 1), radius, fill=fill)
    return panel


def compose(width: int, height: int) -> Image.Image:
    canvas = Image.new("RGBA", (width, height), (4, 8, 13, 255))
    art = Image.open(SOURCE).convert("RGB")
    art_side = int(height * 1.72)
    art = art.resize((art_side, art_side), Image.Resampling.LANCZOS)
    art = art.crop((0, (art_side - height) // 2, art_side, (art_side + height) // 2))
    art = ImageEnhance.Contrast(art).enhance(1.06)
    art = ImageEnhance.Color(art).enhance(0.92).convert("RGBA")
    art_x = width - art.width
    canvas.alpha_composite(art, (art_x, 0))

    shade = Image.new("L", (width, height), 0)
    shade_draw = ImageDraw.Draw(shade)
    shade_draw.rectangle((0, 0, int(width * 0.46), height), fill=255)
    for x in range(int(width * 0.46), int(width * 0.76)):
        alpha = int(255 * (1 - (x - width * 0.46) / (width * 0.30)))
        shade_draw.line((x, 0, x, height), fill=max(0, alpha))
    shade = shade.filter(ImageFilter.GaussianBlur(max(4, width // 220)))
    black = Image.new("RGBA", (width, height), (3, 7, 12, 238))
    canvas = Image.composite(black, canvas, shade)

    draw = ImageDraw.Draw(canvas)
    margin = int(width * 0.055)
    icon_size = int(height * 0.18)
    icon = Image.open(ICON).convert("RGBA").resize((icon_size, icon_size), Image.Resampling.LANCZOS)
    icon_y = int(height * 0.115)
    canvas.alpha_composite(icon, (margin, icon_y))

    brand_x = margin + icon_size + int(width * 0.018)
    brand_y = icon_y + int(icon_size * 0.03)
    draw.text((brand_x, brand_y), "DLSSync", font=font(int(height * 0.078), True), fill=(244, 248, 252, 255))
    draw.text(
        (brand_x, brand_y + int(height * 0.088)),
        "SYNC  /  VERIFY  /  APPLY",
        font=font(int(height * 0.026), True),
        fill=(92, 222, 231, 255),
    )

    title_y = int(height * 0.42)
    draw.text(
        (margin, title_y),
        "One trusted sync layer",
        font=font(int(height * 0.087), True),
        fill=(247, 250, 252, 255),
    )
    draw.text(
        (margin, title_y + int(height * 0.115)),
        "DLSS  |  FSR  |  XeSS  |  DirectStorage  |  Drivers",
        font=font(int(height * 0.034)),
        fill=(181, 194, 208, 255),
    )

    badge_labels = ("SIGNED CATALOG", "SHA-256", "AUTHENTICODE", "ROLLBACK")
    badge_font = font(max(12, int(height * 0.021)), True)
    badge_y = int(height * 0.76)
    badge_x = margin
    badge_gap = int(width * 0.009)
    badge_height = int(height * 0.072)
    for index, label in enumerate(badge_labels):
        box = draw.textbbox((0, 0), label, font=badge_font)
        badge_width = box[2] - box[0] + int(width * 0.024)
        fill = (12, 87, 91, 220) if index < 2 else (20, 61, 58, 220)
        panel = rounded_panel((badge_width, badge_height), badge_height // 2, fill)
        canvas.alpha_composite(panel, (badge_x, badge_y))
        draw.text(
            (badge_x + badge_width // 2, badge_y + badge_height // 2),
            label,
            font=badge_font,
            fill=(226, 249, 247, 255),
            anchor="mm",
        )
        badge_x += badge_width + badge_gap

    draw.text(
        (margin, int(height * 0.9)),
        "v1.7.0  |  Windows 10 / 11  |  Open source  |  Zero telemetry",
        font=font(int(height * 0.024)),
        fill=(135, 151, 168, 255),
    )
    return canvas.convert("RGB")


def main() -> None:
    outputs = {
        ROOT / ".github/assets/nexus/banner-2560x720.png": (2560, 720),
        ROOT / ".github/assets/nexus/banner-header-1300x372.png": (1300, 372),
        ROOT / ".github/assets/nexus/preview-card-clean.png": (1200, 675),
        ROOT / ".github/assets/nexus/preview-card-clean-600x338.png": (600, 338),
        ROOT / ".github/assets/preview-card-clean.png": (1200, 675),
    }
    for path, size in outputs.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        compose(*size).save(path, optimize=True)
        print(f"{path}: {size[0]}x{size[1]}")


if __name__ == "__main__":
    main()
