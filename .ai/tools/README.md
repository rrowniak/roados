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
