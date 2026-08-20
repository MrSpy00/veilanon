"""Generate a lossless raster-embedded SVG brand logo from the 1024px PNG master."""
import base64
import pathlib

ROOT = pathlib.Path(__file__).parent
PNG = ROOT / "png" / "veilanon-1024x1024.png"
OUT = ROOT / "veilanon-logo.svg"

b64 = base64.b64encode(PNG.read_bytes()).decode()

svg = f"""<?xml version="1.0" encoding="UTF-8"?>
<!-- veilanon brand logo — raster-embedded SVG (lossless, 1024px source). -->
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
     width="1024" height="1024" viewBox="0 0 1024 1024">
  <image width="1024" height="1024" xlink:href="data:image/png;base64,{b64}" />
</svg>
"""
OUT.write_text(svg, encoding="utf-8")
print(f"SVG written: {OUT} ({len(svg)} bytes)")
