# Licences

One row per upstream this repository derives committed bytes from, plus the
project's own licence, so a reader looking for "what licence is this?" finds
all three in one place rather than two. The ISC notice for Lucide is quoted
in full because ISC conditions its grant on the notice appearing in all
copies; `ui/src/ui_demo/assets/icons/LICENSE` carries the same text so it
travels with any copy of the icon directory.

| Upstream | Version | Fetched from | SHA-256 | Licence | What it obliges | What was done |
|---|---|---|---|---|---|---|
| Kenney Car Kit | 3.1 | `https://kenney.nl/media/pages/assets/car-kit/1a312ec241-1775131960/kenney_car-kit.zip` | `fac7dacac5c7874348cf19729af3ef205f3d366493edaf0a827d93f4fdf3d0c4` | CC0 (`License.txt`: *"You can use this content for personal, educational, and commercial purposes"*) | Nothing; crediting Kenney is explicitly optional | Credited anyway in `tools/asset-pipeline/model.json`'s provenance and here. `colormap.png`'s origin is that archive's `Models/GLB format/Textures/colormap.png`, named in `model.json`, because a derived image with no stated origin is an unlicensed image with extra steps. |
| Lucide Icons | 1.52.0 | `https://github.com/lucide-icons/lucide/archive/refs/tags/1.52.0.zip` | `f54137c1f9a08eb452624e13a087a8f759e5e36eeac089fce920ed5940c51034` | ISC **and** MIT (see below) | ISC: the copyright and permission notices must appear in all copies. MIT (Feather-derived subset listed in the archive's `LICENSE`): the same notices must be included in all copies | The full archive `LICENSE` (ISC plus the MIT Feather section) is copied verbatim to `ui/src/ui_demo/assets/icons/LICENSE` and quoted below |
| This project | — | — | — | GPLv3 (`LICENSE` at the root) | — | — |

## Lucide licence text, verbatim from the 1.52.0 archive

`TASK_UI_PRIM_39` describes Lucide's licence as ISC. The archive's `LICENSE`
file is ISC **plus** an MIT section for the icons Lucide derives from the
Feather project — and several baked icons here (`calendar`, `clock`,
`download`, `lock`, `navigation`, `power`, `radio`, `smartphone`,
`circle-alert` and `triangle-alert`, the last two via Lucide's renames of
Feather's `alert-circle`/`alert-triangle`) are in that Feather-derived list.
Quoting only the ISC half would leave the MIT half's identical
include-the-notice obligation undischarged, so both halves are quoted and
both travel in `icons/LICENSE`:

```
ISC License

Copyright (c) 2026 Lucide Icons and Contributors

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

---

The following Lucide icons are derived from the Feather project:

airplay, alert-circle, alert-octagon, alert-triangle, aperture, arrow-down-circle, arrow-down-left, arrow-down-right, arrow-down, arrow-left-circle, arrow-left, arrow-right-circle, arrow-right, arrow-up-circle, arrow-up-left, arrow-up-right, arrow-up, at-sign, calendar, cast, check, chevron-down, chevron-left, chevron-right, chevron-up, chevrons-down, chevrons-left, chevrons-right, chevrons-up, circle, clipboard, clock, code, columns, command, compass, corner-down-left, corner-down-right, corner-left-down, corner-left-up, corner-right-down, corner-right-up, corner-up-left, corner-up-right, crosshair, database, divide-circle, divide-square, dollar-sign, download, external-link, feather, frown, hash, headphones, help-circle, info, italic, key, layout, life-buoy, link-2, link, loader, lock, log-in, log-out, maximize, meh, minimize, minimize-2, minus-circle, minus-square, minus, monitor, moon, more-horizontal, more-vertical, move, music, navigation-2, navigation, octagon, pause-circle, percent, plus-circle, plus-square, plus, power, radio, rss, search, server, share, shopping-bag, sidebar, smartphone, smile, square, table-2, tablet, target, terminal, trash-2, trash, triangle, tv, type, upload, x-circle, x-octagon, x-square, x, zoom-in, zoom-out

The MIT License (MIT) (for the icons listed above)

Copyright (c) 2013-present Cole Bemis

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## What this pipeline refuses, and why it is named here

<!-- denylist-exempt start -->
The trademark guard is structural (a closed 33-entry icon list, a two-URL
fetch allow-list, an input-attribute guard), not a review instruction — but
the refusal is recorded in words once: an absent glyph is the moment an
agent reaches for a set that has it, and the CC0 set that has a gauge-metric
equivalent is simple-icons, which ships tesla.svg. That set is unreachable
from this pipeline by construction. `sedan.glb` carries no badge and no
wordmark (the colormap was looked at: a grid of flat colour swatches,
fully opaque on every pixel), so this asset carries no trademark at all.
<!-- denylist-exempt end -->
