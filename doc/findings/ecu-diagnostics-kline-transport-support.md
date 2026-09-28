---
question: Does the ecu_diagnostics crate provide a K-line (ISO 9141) serial transport?
answer: Not natively. Its transport layer section lists exactly three protocols — CAN, ISO-TP, and VAG TP2.0 — and its hardware API checklist covers only CAN-based interfaces, with D-PDU marked TBA. The KWP2000 application layer is fully reusable, so the gap is the physical layer alone; an ELM327-class adapter driven in SLCAN mode may supply it instead, which is unverified.
tag:      [A]
support:  1
evidence: "ecu_diagnostics README.md:92-95, Transport layer support, complete list of three entries — CAN, ISO-TP, VW-TP2 (VW Transport Protocol 2). README.md:97-120, Hardware API checklist — Passthru SAE J2534, SocketCAN, SLCAN, PCAN-USB, with D-PDU (ISO 22900-2) marked TBA at lines 119-120. README.md:13, Implements UDS, KWP2000 and OBD2. README.md:18, ISO-TP transport layer, LIN, J1850 and DoIP is work in progress at this time. Crate at version 0.107.5, licence field GPL-3.0 in Cargo.toml."
read:     2026-09-27
decay:    1 month
unblocks: The K-line design in doc/IDEA.md — whether the application layer is reusable and only the physical layer is missing
---

# Does the ecu_diagnostics crate provide a K-line (ISO 9141) serial transport?

Not natively, and the gap is narrower than it first appears. The crate's
`README.md:92-95` gives the complete transport layer support in three lines:
CAN, ISO-TP, and VW-TP2 (VW Transport Protocol 2). The hardware API checklist
at `README.md:97-120` then enumerates the ways to reach a bus — Passthru
(SAE J2534), SocketCAN, SLCAN, PCAN-USB — and closes with D-PDU (ISO 22900-2)
marked "TBA". Every entry is CAN-based. No serial port and no ISO 9141 appears
anywhere in the file.

The crate's own roadmap line corroborates the absence rather than contradicting
it. `README.md:18` reads "ISO-TP transport layer, LIN, J1850 and DoIP is work in
progress at this time" — and LIN and J1850 are the other single-wire serial-family
buses, so raw serial transports are acknowledged as unfinished rather than
omitted by oversight.

**What is reusable is the expensive part.** `README.md:13` confirms the crate
implements UDS, KWP2000 and OBD2, and the transport table confirms it speaks VAG
TP2.0 — the framing Volkswagen used, and the thing `vag-blocks`, `vwcanread` and
SpeckMobil each reimplemented before being abandoned. Adopting this crate buys
the KWP2000 service layer, session control, security access and TP2.0 framing.

**The caveat that could dissolve the whole finding.** SLCAN is the serial link
protocol spoken by ELM327-class adapters, and those adapters support K-line as
well as CAN — an ELM327 in K-line mode terminates the physical layer in
firmware. If `ecu_diagnostics`' SLCAN transport can drive such an adapter in
K-line mode, the crate reaches K-line without implementing ISO 9141 at all.
**This is unverified**: `README.md:111-113` describes SLCAN as carrying
"ISO-TP (Software)" and CAN, and says nothing about adapter-side K-line modes.
It was not tested, and the adapter's own AT-command behaviour is outside the
crate.

**Implication:** two viable shapes for K-line, differing in what has to be
written. Either implement ISO 9141 init in Rust over `serialport-rs` and reuse
`ecu_diagnostics` above it, or buy an ELM327-class adapter and reuse the crate
all the way down. The first is more work and no third-party firmware in the
path; the second is less work and puts a 20-year-old protocol implementation
between the project and the car. Note that `ecu_diagnostics` is GPL-3.0-only,
which is compatible with this project's GPL-3.0-or-later but forecloses ever
re-licensing the core more permissively without replacing it.

**Reverses if:** the crate adds a serial or K-line transport, or its SLCAN
implementation documents adapter K-line modes. `README.md:92-95` is the single
line to watch, the crate ships releases roughly monthly, and either change would
show up as a diff to those three lines.

## Revisit log

- 2026-09-27 — initial finding, `[A]` from `ecu_diagnostics` `README.md:92-95`, `97-120`, `13` and `18`, all read
