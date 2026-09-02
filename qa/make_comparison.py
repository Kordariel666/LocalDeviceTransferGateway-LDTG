import argparse
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont, ImageOps


ROOT = Path(__file__).resolve().parent.parent
QA_DIR = ROOT / "qa"


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Erzeugt die aktuelle DMDC-Oberflächenübersicht.")
    parser.add_argument("--reference", type=Path, help="Optionale visuelle Zielreferenz.")
    parser.add_argument(
        "--desktop",
        type=Path,
        default=QA_DIR / "desktop-running-1182x852.png",
    )
    parser.add_argument(
        "--mobile",
        type=Path,
        default=QA_DIR / "mobile-login-390x844.png",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=QA_DIR / "style-comparison.png",
    )
    return parser.parse_args()


def main() -> None:
    options = arguments()
    items: list[tuple[str, Path]] = []
    if options.reference:
        items.append(("Zielreferenz", options.reference.resolve()))
    items.extend(
        [
            ("Desktop - Dienst aktiv", options.desktop.resolve()),
            ("Mobile - Login - 390 x 844", options.mobile.resolve()),
        ]
    )

    missing = [str(path) for _, path in items if not path.is_file()]
    if missing:
        raise FileNotFoundError("Fehlende Eingabebilder: " + ", ".join(missing))

    panel_width = 680
    panel_height = 900
    label_height = 52
    canvas = Image.new("RGB", (panel_width * len(items), panel_height), "#080807")
    draw = ImageDraw.Draw(canvas)
    font = ImageFont.load_default(size=18)

    for index, (label, path) in enumerate(items):
        with Image.open(path) as source:
            image = source.convert("RGB")
        fitted = ImageOps.contain(image, (panel_width - 24, panel_height - label_height - 24))
        x = index * panel_width + (panel_width - fitted.width) // 2
        y = label_height + (panel_height - label_height - fitted.height) // 2
        canvas.paste(fitted, (x, y))
        draw.text((index * panel_width + 16, 16), label, fill="#f2ede3", font=font)
        if index:
            draw.line(
                (index * panel_width, 0, index * panel_width, panel_height),
                fill="#38332b",
                width=1,
            )

    options.output.resolve().parent.mkdir(parents=True, exist_ok=True)
    canvas.save(options.output.resolve(), format="PNG")


if __name__ == "__main__":
    main()
