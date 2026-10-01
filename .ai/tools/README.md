# Tools

Small utilities an agent may reach for. Each one is documented with its purpose
and, more importantly, with what it may **not** be used for.

A tool here is not an authority. Using one does not make its output evidence —
`../protocols/evidence.md` decides that, and it decides it the same way whether
a tool was involved or not.

## `topy.py` — reduce a fetched HTML page to text

```sh
python3 .ai/tools/topy.py page.html
python3 .ai/tools/topy.py 'pages/*.html'
```

Reads each path (globs expand), strips `<script>`, `<style>`, `<nav>` and
`<footer>`, converts block-level tags to newlines, strips the remaining tags,
unescapes entities, and collapses whitespace. Prints a separator and the text.

### Use it for

- **Locating a clause in a large page.** Licence texts, crate documentation,
  specification pages, RFCs. Save the page, reduce it, search the text for the
  section you need, then go to the HTML for the actual citation. This is the
  intended use and the reason the tool exists: `webfetch` may summarise a large
  page, and a summarised licence is not a licence.
- **Checking what a page actually served.** Sometimes a documentation site
  disagrees with its own source, and the served HTML is the evidence. `topy`
  shows you what a plain HTTP client received, with no rendering and no
  JavaScript — which is itself a finding when it is empty.

### Do not use it for

- **Citing its output.** The output has no line numbers matching any source and
  the filtering is lossy. A finding's `EVIDENCE` field names a file and a line —
  name the saved HTML, not this text.
- **Proving a page's content.** A fact rendered by JavaScript is absent here.
  Absence in the output is not absence in the page; it is a `[C]` at best, and
  usually a reason to say "JS-rendered, not read".
- **Pages where the evidence might be in what was dropped.** `<nav>`,
  `<footer>`, `<script>` and `<style>` are removed without comment. Licence
  headers, version and copyright footers, and JSON-LD structured data are all
  common places for exactly the facts a research question asks about. If the
  question is about provenance, licence or version, read the HTML.

### Known limits

- `errors='replace'` on decode means a byte sequence that is not valid UTF-8 is
  silently replaced with U+FFFD rather than raising. For a tool whose output may
  be quoted, that is the wrong default, and it can corrupt an identifier or a
  clause number without any visible sign. Changing it to a strict decode is a
  one-line fix, pending a decision.
- No timeout, no size cap, no atomic read. Point it at pages, not at
  arbitrarily large files.
- Regex-based tag stripping is not an HTML parser. It is adequate for
  prose-shaped pages and wrong for documents where the structure carries the
  meaning. When structure matters, read the source.

### Not covered

- HTML that needs rendering. `webfetch` with `format: text` handles the common
  case better and should be tried first. This tool exists for what it cannot
  keep: full, deterministic, unsummarised output from a page you already have on
  disk.

## `fps-check.sh` — measure the demo's frame rate, and judge it against a floor

```sh
.ai/tools/fps-check.sh                # measure a 10 s release run
.ai/tools/fps-check.sh 5 55           # measure 5 s and require 55 fps
FPS_MIN_FPS=55 .ai/tools/fps-check.sh # the same floor, from the environment
```

Builds `ui_demo` in release, runs it with `ROADOS_RUN_SECONDS`, and reads the one
`roados-fps key=value …` line the demo prints on stdout. Exit status is 1 when a
floor was given and missed, or when the run produced no report at all.

### Use it for

- **Answering "did this change cost anything?"** The average over a fixed run, the
  worst single frame, and how many frames took longer than two of the loop's own
  16 ms slots. `doc/ui/IMPLEMENTATION_STATE.md` § *The frame rate, measured*
  carries the baseline this repository compares against.
- **Any performance claim at all.** A claim with a number behind it can be
  checked; the same claim with "it looked the same" behind it cannot, and a still
  of a 4 fps application is pixel-identical to a still of a 60 fps one.

### Do not use it for

- **Proving the demo is correct.** It measures a rate and nothing else. A run at
  60 fps of a window drawing the wrong thing passes it.
- **Claiming a regression on a shared or loaded machine from one run.** One run is
  one sample of a machine that may be running something else. Compare three runs
  before and three after, as the benchmark tables in
  `doc/ui/IMPLEMENTATION_STATE.md` do, and say which you did.
- **Comparing a debug build with a release one.** This script builds release,
  because a debug build's rate is a fact about unoptimised Rust rather than about
  the interface. The two differ by about 15 fps on this repository's demo, which
  is a number worth knowing and not one to regress against.
- **Judging a run that never happened.** A missing `roados-fps` line is a run that
  failed, and the script exits 1 on it for that reason.

### Known limits

- **The rate it reports is the loop's, not the display's.** There is no vsync and
  no frame pacing: the loop waits 16 ms for an event and then draws, so the two
  costs are serialised and a ceiling of about 62 fps is built into the shape of
  the loop. A number near that ceiling is a loop that nothing in the interface is
  holding back.
- **It needs a display.** A headless run measures nothing, and the script says so
  rather than reporting zero.
- **The floor is an argument, not a constant in this file.** A number about a
  machine belongs to whoever measured it, and there is one copy of the baseline
  rather than two.
