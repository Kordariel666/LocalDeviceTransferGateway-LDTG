from pathlib import Path

from PIL import Image, ImageDraw, ImageFont, ImageOps


ROOT = Path(r"C:\Users\marcb\Documents\Projekte\DesktopMobileDataCenter-DMDC")
REFERENCE = Path(
    r"C:\Users\marcb\.codex\generated_images\01a04f2c-5e85-7520-a57c-457ca534c70c\exec-3c0689bd-bcf5-4a06-92af-32f3e3aff042.png"
)
ITEMS = [
    ("Zielreferenz", REFERENCE),
    ("Desktop · Dienst läuft", ROOT / "qa" / "desktop-running-1182x852.jpg"),
    ("Mobile · Login · 390 px", ROOT / "qa" / "mobile-login-390x844.png"),
]

panel_width = 680
panel_height = 900
label_height = 52
canvas = Image.new("RGB", (panel_width * len(ITEMS), panel_height), "#080807")
draw = ImageDraw.Draw(canvas)
font = ImageFont.load_default(size=18)

for index, (label, path) in enumerate(ITEMS):
    image = Image.open(path).convert("RGB")
    fitted = ImageOps.contain(image, (panel_width - 24, panel_height - label_height - 24))
    x = index * panel_width + (panel_width - fitted.width) // 2
    y = label_height + (panel_height - label_height - fitted.height) // 2
    canvas.paste(fitted, (x, y))
    draw.text((index * panel_width + 16, 16), label, fill="#f2ede3", font=font)
    if index:
        draw.line((index * panel_width, 0, index * panel_width, panel_height), fill="#38332b", width=1)

canvas.save(ROOT / "qa" / "style-comparison.png")
