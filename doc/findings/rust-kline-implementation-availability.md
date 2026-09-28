---
question: Does a production-quality Rust implementation of the K-line (ISO 9141) physical layer exist that the core process could depend on?
answer: None was found. A crates.io search across K-line, KWP, OBD and ISO 9141/14230 query terms returned two crates created within the last three months with under 60 downloads each, plus unrelated AES key-wrap crates that share the kwp name. The most capable Rust diagnostic stack, ecu_diagnostics, implements the KWP2000 application layer and VAG TP2.0 but ships no ISO 9141 transport, so the physical layer would have to be written.
tag:      [C]
support:  1
evidence: "crates.io /q/ API searches for kwp, kwp2000, kline, k-line, 9141, iso-9141, iso-14230, kw1281, vag, vwa, obd, queried 2026-09-27. industrial-protocols-kline 1.1.2, erikwang2013/industrial-protocols-rust, created 2026-07-23, 56 downloads, 3 versions. obd2-rs 0.1.1, facorazza/obd2-rs, created 2026-08-18, 42 downloads. ecu_diagnostics transport list at README.md:92-95 contains CAN, ISO-TP and VW-TP2 only, no ISO 9141. This is a negative result and is tagged [C] per the null-result rule; the depth of freediag's maintenance was not assessed for contributor count or issue close rate, so no claim is made that it is maintained."
read:     2026-09-27
decay:    1 week
unblocks: Whether the K-line work in doc/IDEA.md is a dependency choice or an implementation project, and what the bus adapter line item actually has to contain
---

# Does a production-quality Rust implementation of the K-line (ISO 9141) physical layer exist that the core process could depend on?

No. This is a negative result and is reported as one, with the search that
produced it, because a null result without its search is indistinguishable from
not having looked.

The closest Rust asset is `ecu_diagnostics` 0.107.5, which is the best KWP2000
implementation in the ecosystem. Its `README.md:92-95` lists the complete
transport layer support as three items — CAN, ISO-TP, and VW-TP2 — and its
hardware API checklist at `README.md:97-120` adds only CAN-based interfaces:
J2534 passthru, SocketCAN, SLCAN, PCAN-USB, with D-PDU marked "TBA". No serial
or ISO 9141 entry appears anywhere. The crate's own roadmap line corroborates
this: `README.md:18` lists "ISO-TP transport layer, LIN, J1850 and DoIP" as
work in progress, and LIN and J1850 are the other single-wire serial-family
buses.

That leaves the K-line work split in two. The **application layer** — KWP2000
service encoding, session control, security access, VAG's TP2.0 framing — is
available and is the part that is genuinely reverse-engineered and tedious.
The **physical layer** — 5-baud initialisation, fast init, the `0x6C`/`0x6D`
sync pattern, and its timing — is not, and is a few hundred lines over a serial
port.

**Implication:** K-line in `doc/IDEA.md` is an implementation project, not a
dependency choice. Budget it as bespoke protocol work with a bring-up cost
against a real car, and treat the ECU-side KWP2000 layer as free. This is the
single largest piece of original engineering in the project and it is not the
part that looks risky.

**Reverses if:** a K-line implementation appears with recorded traffic from a
VAG K-line, or `ecu_diagnostics` gains a serial transport — its
`README.md:92-95` is the one line to watch, and the crate publishes releases
roughly monthly. Both crates found here are three months old; if either gains
users before the project reaches implementation, re-check rather than build.

## Searched

- **crates.io** `/q/` API, 2026-09-27, queries: `kwp`, `kwp2000`, `kline`,
  `k-line`, `9141`, `iso-9141`, `iso-14230`, `kw1281`, `vag`, `vwa`, `obd`.
  Two genuine hits, both created within three months and both under 60
  downloads. The remainder of the `kwp` result space is AES Key Wrap
  (`aes-kw`, `belt-kwp`) — a name collision with Keyword Protocol, irrelevant.
- **GitHub**, 2026-09-27, for C and non-Rust prior art: `fenugrec/freediag`
  (GPL-3.0, last push 2024-09-09, 493 stars, provides `libdiag` with
  ISO 9141 and KWP1281 over K-line — the reference implementation, but C and
  with no CAN support), `jazdw/vag-blocks` (GPL-3.0, `archived: true`),
  `notyal/vwcanread` (GPL-3.0, WIP, 1 contributor), `Boromatic/SpeckMobil`
  (GPL-3.0, last push 2014-06-10), `fjvva/ecu-tool` (no licence file,
  `lic=None`, legally unusable), `baconwaifu/PyVCDS` (`NOASSERTION`,
  archived).
- **Not searched:** non-English registries, GitLab, code-search across GitHub
  for an un-crates.io K-line implementation. A Rust K-line living outside
  crates.io would have been missed.

## Revisit log

- 2026-09-27 — initial finding, negative result, `[C]` per the null-result rule
