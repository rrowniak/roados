#!/usr/bin/env python3
"""The gate: every claim the asset pipeline makes, checked from committed bytes.

What it is: one command that runs every check in order and stops at the
first failure with the failing check named. It reads what the other tools
wrote and re-derives it — through their own readers, not copies — so a
hand-edited artefact is caught, not just a bad run.

What it reads: tools/asset-pipeline/icons.json, the committed tree
ui/src/ui_demo/assets/, LICENSES.md, MANIFEST.sha256, and the cached
upstream tree under .asset-cache/ (for the sedan GLB and its colormap).

What it writes: nothing, except with --write-manifest (which regenerates
ui/src/ui_demo/assets/MANIFEST.sha256) — and --changed only prints.

What it must not be used for: asserting PNG reproducibility by hash. The
IDAT stream is zlib output and zlib is pinned by nothing here, so a PNG
hash verifies this host's encoder, not the pipeline. PNGs are checked by
pixels (size, mode, non-emptiness, coverage-weighted colour mean); only the
byte-stable artefacts (sedan.glb, colormap.png) are checked by hash, and
the output says which is which.

Usage:
  check_assets.py --all             run checks 1-6, stop at the first failure
  check_assets.py --changed         print changed/added/removed vs MANIFEST.sha256
  check_assets.py --write-manifest  (re)generate MANIFEST.sha256, sorted
"""

import argparse
import hashlib
import json
import os
import sys

TOOLS_DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, TOOLS_DIR)
import bake_icons

ROOT = os.path.dirname(os.path.dirname(TOOLS_DIR))
ASSETS = os.path.join(ROOT, "ui", "src", "ui_demo", "assets")
MANIFEST_PATH = os.path.join(ASSETS, "MANIFEST.sha256")
# The byte-stable model artefacts and the cached upstream files they are
# byte-for-byte copies of. The committed model is Kenney's GLB itself; there is
# no converter between the two, so the check is a comparison of two files.
CACHE = os.path.join(ROOT, ".asset-cache", "kenney_car-kit", "Models", "GLB format")
STABLE_ASSETS = [
    ("ui/src/ui_demo/assets/sedan.glb", os.path.join(CACHE, "sedan.glb")),
    (
        "ui/src/ui_demo/assets/colormap.png",
        os.path.join(CACHE, "Textures", "colormap.png"),
    ),
]
ATLAS_MAX_IMAGE = 512

DENYLIST = ["tesla", "simple-icons", "brandfetch", "worldvectorlogo", "svglogo", "wikimedia"]
EXEMPT_START = "<!-- denylist-exempt start -->"
EXEMPT_END = "<!-- denylist-exempt end -->"


def fail(check, message):
    print("check_assets: [%s] FAIL: %s" % (check, message), file=sys.stderr)
    sys.exit(1)


def ok(check, message):
    print("check_assets: [%s] ok: %s" % (check, message))


def load_json(name):
    path = os.path.join(TOOLS_DIR, name)
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def check_upstream_bytes():
    """1. The committed model artefacts are byte copies of the cached upstream.

    The runtime reads Kenney's GLB directly (there is no offline converter), so
    the guarantee this check enforces is that the committed `sedan.glb` and
    `colormap.png` are byte-identical to the files under `.asset-cache/` that
    `fetch_upstream.sh` pinned. A hand-edited committed model is caught here; a
    moved upstream is caught by `upstream.sha256` at fetch time.
    """
    for committed, upstream in STABLE_ASSETS:
        committed_path = os.path.join(ROOT, committed)
        if not os.path.isfile(upstream):
            fail("upstream", "cached upstream %s is absent; run fetch_upstream.sh"
                 % os.path.relpath(upstream, ROOT))
        with open(committed_path, "rb") as handle:
            committed_bytes = handle.read()
        with open(upstream, "rb") as handle:
            upstream_bytes = handle.read()
        if committed_bytes != upstream_bytes:
            fail("upstream", "committed %s is not byte-identical to %s"
                 % (committed, os.path.relpath(upstream, ROOT)))
        ok("upstream", "committed %s is the cached upstream's bytes" % committed)


def read_manifest_file():
    entries = {}
    with open(MANIFEST_PATH, "r", encoding="utf-8") as handle:
        for line in handle.read().splitlines():
            line = line.strip()
            if not line:
                continue
            digest, path = line.split("  ", 1)
            entries[path] = digest
    return entries


def check_hashes():
    """2. Byte-stable artefacts by hash, encoder-dependent ones by pixels.

    sedan.glb and colormap.png are byte-stable (byte copies): their hashes must
    match. icons/** are encoder-dependent (zlib): their manifest lines must
    exist but their hashes are NOT compared here — check_icons and check_pixels
    verify them instead. A gate that quietly treated a PNG hash as a
    reproducibility claim would be worse than none.
    """
    entries = read_manifest_file()
    stable = [committed for committed, _upstream in STABLE_ASSETS]
    for path in stable:
        if path not in entries:
            fail("hashes", "MANIFEST.sha256 has no line for byte-stable %s" % path)
        full = os.path.join(ROOT, path)
        with open(full, "rb") as handle:
            digest = hashlib.sha256(handle.read()).hexdigest()
        if digest != entries[path]:
            fail("hashes", "byte-stable %s differs from MANIFEST.sha256" % path)
        ok("hashes", "byte-stable %s matches MANIFEST.sha256" % path)
    icons = sorted(p for p in entries if "/icons/" in p)
    if not icons:
        fail("hashes", "MANIFEST.sha256 lists no icons")
    for path in icons:
        if not os.path.isfile(os.path.join(ROOT, path)):
            fail("hashes", "manifest-listed %s is absent from the tree" % path)
    ok("hashes", "%d icons/** are encoder-dependent: presence checked, "
                  "bytes deliberately not compared (see check_pixels)" % len(icons))


def check_licences():
    """3. LICENSES.md names both upstreams; the ISC notice travels."""
    root_licences = os.path.join(ROOT, "LICENSES.md")
    with open(root_licences, "r", encoding="utf-8") as handle:
        text = handle.read()
    for needle in ("Kenney", "CC0", "Lucide", "ISC",
                   "Copyright (c) 2026 Lucide Icons and Contributors",
                   "Permission to use, copy, modify"):
        if needle not in text:
            fail("licences", "LICENSES.md does not contain %r" % needle)
    icons_licence = os.path.join(ASSETS, "icons", "LICENSE")
    with open(icons_licence, "r", encoding="utf-8") as handle:
        travelling = handle.read()
    for needle in ("Copyright (c) 2026 Lucide Icons and Contributors",
                   "Permission to use, copy, modify"):
        if needle not in travelling:
            fail("licences", "icons/LICENSE does not carry the ISC notice (%r)" % needle)
    if travelling not in text:
        fail("licences", "LICENSES.md does not quote the icons/LICENSE notice in full")
    ok("licences", "both upstreams named with licences; ISC notice verbatim here and travelling")


def denylist_hits():
    """4. The trademark denylist, with the one documented exemption.

    The exemption is the unmapped record: icons.json's
    unmapped[].reason and LICENSES.md's marked refusal section name the
    refused CC0 set and its car-maker mark to record the refusal, and a
    denylist that forbade the record of the refusal would forbid the
    record. Exempted matches are reported, not silently skipped.
    """
    hits = []
    exempted = []

    def scan_text(origin, text):
        for number, line in enumerate(text.splitlines(), 1):
            lowered = line.lower()
            for term in DENYLIST:
                if term in lowered:
                    hits.append("%s:%d: %s" % (origin, number, term))

    def unmapped_reasons(parsed):
        found = []
        if isinstance(parsed.get("unmapped"), list):
            for entry in parsed["unmapped"]:
                if isinstance(entry, dict) and "reason" in entry:
                    found.append(entry["reason"])
        return found

    manifest_name = "icons.json"
    parsed = load_json(manifest_name)
    reasons = unmapped_reasons(parsed)
    with open(os.path.join(TOOLS_DIR, manifest_name), "r", encoding="utf-8") as handle:
        for line in handle.read().splitlines():
            lowered = line.lower()
            if any(r.lower() in lowered for r in reasons):
                for term in DENYLIST:
                    if term in lowered:
                        exempted.append("%s unmapped reason: %s" % (manifest_name, term))
            else:
                for term in DENYLIST:
                    if term in lowered:
                        hits.append("%s: %s" % (manifest_name, term))

    with open(os.path.join(ROOT, "LICENSES.md"), "r", encoding="utf-8") as handle:
        in_exempt = False
        for number, line in enumerate(handle.read().splitlines(), 1):
            if EXEMPT_START in line:
                in_exempt = True
                continue
            if EXEMPT_END in line:
                in_exempt = False
                continue
            lowered = line.lower()
            for term in DENYLIST:
                if term in lowered:
                    if in_exempt:
                        exempted.append("LICENSES.md:%d: %s (marked refusal)" % (number, term))
                    else:
                        hits.append("LICENSES.md:%d: %s" % (number, term))

    for base, _dirs, files in os.walk(ASSETS):
        for name in files:
            if name.endswith((".png", ".roados")):
                continue
            full = os.path.join(base, name)
            with open(full, "r", encoding="utf-8", errors="replace") as handle:
                scan_text(os.path.relpath(full, ROOT), handle.read())
    # Binary names cannot carry a mark, but the check is cheap and explicit.
    names_blob = b"".join(os.path.relpath(os.path.join(b, f), ROOT).encode()
                          for b, _d, files in os.walk(ASSETS) for f in files)
    for term in DENYLIST:
        if term.encode() in names_blob.lower():
            hits.append("assets tree filename: %s" % term)
    return hits, exempted


def check_denylist():
    hits, exempted = denylist_hits()
    if hits:
        fail("denylist", "; ".join(hits))
    ok("denylist", "no denylisted string outside the documented exemption (%s)"
       % ("; ".join(sorted(set(exempted))) if exempted else "exemption unused"))


def check_icons():
    """5. The icon count, both directions, and the atlas size bound."""
    manifest = load_json("icons.json")
    want = {"%s/%s.png" % (theme, name)
            for name in manifest["icons"] for theme in manifest["themes"]}
    have = set()
    for theme in manifest["themes"]:
        directory = os.path.join(ASSETS, "icons", theme)
        if not os.path.isdir(directory):
            fail("icons", "missing directory %s" % directory)
        for name in os.listdir(directory):
            if name.endswith(".png"):
                have.add("%s/%s" % (theme, name))
            else:
                fail("icons", "non-PNG file in icon directory: %s/%s" % (theme, name))
    if want != have:
        fail("icons", "manifest names %d files, directory holds %d "
                      "(missing %r, extra %r)"
             % (len(want), len(have),
                sorted(want - have)[:3], sorted(have - want)[:3]))
    oversized = []
    for theme in manifest["themes"]:
        directory = os.path.join(ASSETS, "icons", theme)
        for name in os.listdir(directory):
            if not name.endswith(".png"):
                continue
            image = bake_icons.Image.open(os.path.join(directory, name))
            if image.size[0] > ATLAS_MAX_IMAGE or image.size[1] > ATLAS_MAX_IMAGE:
                oversized.append("%s/%s" % (theme, name))
    if oversized:
        fail("icons", "over ATLAS_MAX_IMAGE, escapes the atlas: %r" % oversized)
    ok("icons", "%d files, manifest and directory agree both ways, all within %dpx"
       % (len(want), ATLAS_MAX_IMAGE))


def check_pixels():
    """6. Per-PNG value assertions re-run from the committed file."""
    manifest = load_json("icons.json")
    count = 0
    for theme, hexcode in manifest["themes"].items():
        for semantic in manifest["icons"]:
            for size in manifest["sizes"]:
                path = os.path.join(ASSETS, "icons", theme, semantic + ".png")
                # check_output_value exits naming the file on any mismatch:
                # a hand-edited PNG is caught, not just a bad run.
                bake_icons.check_output_value(path, size, bake_icons.hex_to_rgb(hexcode),
                                              semantic, theme + " (committed)")
                count += 1
    ok("pixels", "%d committed PNGs re-asserted (size, RGBA, non-empty, colour mean)" % count)


def generated_artefacts():
    """Exactly the pipeline's outputs: the model, the colormap, the icons.

    demo.png is a pre-existing committed asset and is not listed: the
    manifest describes what the pipeline puts in the tree, so a reviewer can
    confirm that everything else did not change.
    """
    paths = [committed for committed, _upstream in STABLE_ASSETS]
    for theme in ("dark", "light"):
        directory = os.path.join(ASSETS, "icons", theme)
        for name in sorted(os.listdir(directory)):
            if name.endswith(".png"):
                paths.append("ui/src/ui_demo/assets/icons/%s/%s" % (theme, name))
    return paths


def read_tree_hashes():
    current = {}
    for path in generated_artefacts():
        full = os.path.join(ROOT, path)
        with open(full, "rb") as handle:
            current[path] = hashlib.sha256(handle.read()).hexdigest()
    return current


def cmd_changed():
    """7. Name what changed against the committed MANIFEST.sha256."""
    recorded = read_manifest_file()
    current = read_tree_hashes()
    changed = sorted(p for p in recorded if p in current and recorded[p] != current[p])
    added = sorted(p for p in current if p not in recorded)
    removed = sorted(p for p in recorded if p not in current)
    print("changed (%d): %s" % (len(changed), " ".join(changed) if changed else "-"))
    print("added (%d): %s" % (len(added), " ".join(added) if added else "-"))
    print("removed (%d): %s" % (len(removed), " ".join(removed) if removed else "-"))
    return 0


def cmd_write_manifest():
    current = read_tree_hashes()
    with open(MANIFEST_PATH, "w", encoding="utf-8") as handle:
        for path in sorted(current):
            handle.write("%s  %s\n" % (current[path], path))
    print("check_assets: wrote %s (%d artefacts)" % (MANIFEST_PATH, len(current)))
    return 0


def main(argv):
    parser = argparse.ArgumentParser(description="Gate for the asset pipeline.")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--all", action="store_true", help="run checks 1-6, stop at first failure")
    group.add_argument("--changed", action="store_true", help="name changed/added/removed vs manifest")
    group.add_argument("--write-manifest", action="store_true", help="(re)generate MANIFEST.sha256")
    arguments = parser.parse_args(argv)
    if arguments.changed:
        return cmd_changed()
    if arguments.write_manifest:
        return cmd_write_manifest()
    for step in (check_upstream_bytes, check_hashes, check_licences,
                 check_denylist, check_icons, check_pixels):
        step()
    print("check_assets: --all green")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
