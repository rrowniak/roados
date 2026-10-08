#!/usr/bin/env python3
"""Bake Lucide SVGs into per-theme PNGs at exact on-screen size.

What it is: the icon half of the asset pipeline. For every semantic name in
icons.json, at every size, for both themes, it runs resvg once with a
stylesheet that sets the currentColor the engine cannot set at draw time
(DrawCommand::Image takes a scalar opacity, never a colour) and asserts the
baked pixels are actually that colour.

What it reads: tools/asset-pipeline/icons.json (names, sizes, theme hexes)
and the extracted Lucide tree (default .asset-cache/lucide-1.52.0/icons).

What it writes: ui/src/ui_demo/assets/icons/<theme>/<name>.png — the whole
set into a temporary directory first, moved into place only when every icon
passes, so a run that fails half way leaves the committed set untouched.

What it must not be used for: baking any SVG that fails the input guard. A
plain fill with no colour attribute needs a {fill:...} rule instead of the
{color:...} one used here; the guard refuses such files so the rule stays
correct for this set and impossible to apply to one it is wrong for.
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile

try:
    from PIL import Image
except ImportError:
    print("bake_icons: Pillow is required (python3 with Pillow; see "
          "tools/asset-pipeline/README.md).", file=sys.stderr)
    sys.exit(1)

try:
    import numpy as np
except ImportError:
    print("bake_icons: numpy is required (python3 with numpy; see "
          "tools/asset-pipeline/README.md).", file=sys.stderr)
    sys.exit(1)

REQUIRED_ROOT_ATTRIBUTES = {
    "viewBox": "0 0 24 24",
    "fill": "none",
    "stroke": "currentColor",
    "stroke-width": "2",
}


def fail(message):
    print("bake_icons: error: " + message, file=sys.stderr)
    sys.exit(1)


def find_resvg(explicit):
    """Resolve resvg by absolute path, never by hope.

    ~/.cargo/bin is not on PATH on this host (and command -v resvg returns
    nothing while the binary works), so a bare resvg fails on a host where
    it is installed and sends the next agent looking for a missing
    dependency. The order: $RESVG, then PATH, then $HOME/.cargo/bin/resvg,
    else a hard error naming all three.
    """
    candidates = []
    if explicit:
        candidates.append(explicit)
    found = shutil.which("resvg")
    if found:
        candidates.append(found)
    home = os.environ.get("HOME", "")
    if home:
        candidates.append(os.path.join(home, ".cargo", "bin", "resvg"))
    for candidate in candidates:
        if candidate and os.path.isfile(candidate) and os.access(candidate, os.X_OK):
            return candidate
    fail("resvg 0.48.1 not found; looked at RESVG=%r, PATH, and %s. "
         "Install it (cargo install resvg --version 0.48.1); installing it "
         "is the operator's call and is not done here."
         % (explicit, os.path.join("$HOME", ".cargo", "bin", "resvg")))


def check_input_guard(path, semantic):
    import xml.etree.ElementTree as ET
    try:
        root = ET.parse(path).getroot()
    except ET.ParseError as error:
        fail("input %s (semantic %s) does not parse: %s" % (path, semantic, error))
    tag = root.tag.split("}")[-1] if "}" in root.tag else root.tag
    if tag != "svg":
        fail("input %s (semantic %s): root element is %s, not svg" % (path, semantic, tag))
    for attribute, want in REQUIRED_ROOT_ATTRIBUTES.items():
        got = root.get(attribute)
        if got != want:
            fail("input %s (semantic %s): root %s is %r, need %r"
                 % (path, semantic, attribute, got, want))


def hex_to_rgb(text):
    text = text.lstrip("#")
    return tuple(int(text[i:i + 2], 16) for i in (0, 2, 4))


def check_output_value(path, size, want_rgb, semantic, theme):
    image = Image.open(path)
    if image.mode != "RGBA":
        fail("output %s (semantic %s theme %s): mode is %s, need RGBA"
             % (path, semantic, theme, image.mode))
    if image.size != (size, size):
        fail("output %s (semantic %s theme %s): size is %r, need %dx%d"
             % (path, semantic, theme, image.size, size, size))
    pixels = np.asarray(image).astype(np.int32)
    alpha = pixels[:, :, 3]
    if int(alpha.max()) == 0:
        fail("output %s (semantic %s theme %s): every pixel transparent; "
             "an empty rasterisation is a failure, not a transparent icon"
             % (path, semantic, theme))
    # The mean is coverage-weighted by alpha, not a flat mean over painted
    # pixels. resvg stores straight RGBA, so a faint edge pixel's colour is
    # unpremultiplied back out of a small alpha and quantised bright:
    # plug-zap's flat mean is (28.11, ...) against #1a1a1a (26, ...), a
    # 2.11 deviation that fails the +-2 band while the icon is the right
    # colour — its coverage-weighted mean is (26.73, ...). Weighting counts
    # each pixel as the compositor sees it. The guard's power is unchanged:
    # a missing stylesheet bakes (0, 0, 0), which deviates 26 and 236.
    cover = float(alpha.sum())
    painted = tuple(float((pixels[:, :, c] * alpha).sum()) / cover for c in range(3))
    if any(abs(painted[c] - want_rgb[c]) > 2.0 for c in range(3)):
        fail("output %s (semantic %s theme %s): coverage-weighted mean of "
             "painted pixels (%.2f, %.2f, %.2f) is not within 2 per channel of #%02x%02x%02x. "
             "Without --stylesheet resvg exits 0 and writes a valid black PNG; "
             "only the value tells it apart."
             % (path, semantic, theme, painted[0], painted[1], painted[2], *want_rgb))
    return painted


def main(argv):
    parser = argparse.ArgumentParser(
        description="Bake icons.json's icon set into per-theme PNGs. "
                    "Bakes the whole set or fails; a missing entry is a hard "
                    "failure naming it, never a skipped icon.")
    parser.add_argument("--manifest", required=True, help="path to icons.json")
    parser.add_argument("--icons", default=None,
                        help="extracted Lucide icons directory "
                             "(default: .asset-cache under the repository root)")
    parser.add_argument("--resvg", default=None,
                        help="resvg binary (default: resolved per the order in find_resvg)")
    parser.add_argument("--out", default=None,
                        help="output icons directory "
                             "(default: ui/src/ui_demo/assets/icons under the root)")
    arguments = parser.parse_args(argv)
    with open(arguments.manifest, "r", encoding="utf-8") as handle:
        manifest = json.load(handle)
    root = os.path.dirname(os.path.dirname(os.path.dirname(
        os.path.abspath(arguments.manifest))))
    icons_dir = arguments.icons or os.path.join(
        root, ".asset-cache", manifest["upstream"].replace(".zip", ""), manifest["icons_dir"])
    out_dir = arguments.out or os.path.join(root, "ui", "src", "ui_demo", "assets", "icons")
    resvg = find_resvg(arguments.resvg or os.environ.get("RESVG"))

    icons = manifest["icons"]
    sizes = manifest["sizes"]
    themes = manifest["themes"]
    expected = {(name, size, theme)
                for name in icons for size in sizes for theme in themes}
    print("bake_icons: %d icons x %d sizes x %d themes = %d files (resvg %s)"
          % (len(icons), len(sizes), len(themes), len(expected), resvg))

    staging = tempfile.mkdtemp(prefix="bake_icons_")
    try:
        stylesheets = {}
        for theme, hexcode in themes.items():
            sheet = os.path.join(staging, theme + ".css")
            with open(sheet, "w", encoding="utf-8") as handle:
                # svg{color} because the colour inherits into the shapes;
                # *{color} because usvg's cascade reaches them through the
                # root. Never {fill:...}: every input root carries
                # fill="none", so a fill rule would fill the glyph outlines.
                handle.write("svg{color:%s}*{color:%s}" % (hexcode, hexcode))
            stylesheets[theme] = sheet
        for semantic, filename in icons.items():
            source = os.path.join(icons_dir, filename)
            if not os.path.isfile(source):
                fail("semantic %s wants %s from %s, which does not exist "
                     "(pinned tag %s, manifest %s); never a skipped icon"
                     % (semantic, filename, icons_dir,
                        manifest["upstream"], arguments.manifest))
            check_input_guard(source, semantic)
            for size in sizes:
                for theme, hexcode in themes.items():
                    target_dir = os.path.join(staging, "files", theme)
                    os.makedirs(target_dir, exist_ok=True)
                    target = os.path.join(target_dir, semantic + ".png")
                    # --width/--height once, at the exact on-screen size: no
                    # -z, no --dpi, no supersample-then-downsample. 1 asset
                    # pixel is 1 window pixel (no DPI concept, no mipmaps),
                    # and 24 is Lucide's own viewBox, so the rasteriser
                    # performs no rescale at all. --shape-rendering names
                    # resvg's default explicitly so a future default change
                    # is not a silent diff across every file.
                    completed = subprocess.run(
                        [resvg, "--stylesheet", stylesheets[theme],
                         "--width", str(size), "--height", str(size),
                         "--shape-rendering", "geometricPrecision",
                         source, target],
                        stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                    if completed.returncode != 0:
                        fail("resvg failed on %s (semantic %s theme %s): %s"
                             % (source, semantic, theme,
                                completed.stderr.decode("utf-8", "replace")[-500:]))
                    check_output_value(target, size, hex_to_rgb(hexcode), semantic, theme)
        final_dir = os.path.join(staging, "files")
        # The move happens only after every icon passed — but it is a sync,
        # not a wipe: out_dir also holds icons/LICENSE (the ISC notice that
        # must travel with the directory), which this tool neither writes
        # nor owns. A rmtree here would delete it on every run and the next
        # commit would silently drop the licence from the icon set.
        for theme in themes:
            target_dir = os.path.join(out_dir, theme)
            os.makedirs(target_dir, exist_ok=True)
            staged_dir = os.path.join(final_dir, theme)
            staged = set(os.listdir(staged_dir))
            for name in os.listdir(target_dir):
                if name.endswith(".png") and name not in staged:
                    os.remove(os.path.join(target_dir, name))
            for name in sorted(staged):
                shutil.copyfile(os.path.join(staged_dir, name),
                                os.path.join(target_dir, name))
    finally:
        shutil.rmtree(staging, ignore_errors=True)
    count = sum(1 for _root, _dirs, files in os.walk(out_dir) for name in files
                if name.endswith(".png"))
    want = len(icons) * len(sizes) * len(themes)
    if count != want:
        fail("wrote %d files, manifest names %d" % (count, want))
    print("bake_icons: wrote %d files under %s" % (count, out_dir))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
