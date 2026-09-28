---
question: How does a Volkswagen Group MIB3 infotainment unit receive a software update, and can it be rolled back?
answer:  Not over the air, and not by the owner. A Volkswagen Group of America TSB filed twice with NHTSA documents a USB-stick update path reached through a hidden Service Mode — press and hold the Menu hard key — followed by Software Update/Versions, Start Update, then USB 1. A separate SVM section is the rollback mechanism. A charger of 70 A or greater is mandatory to avoid problems during the update.
tag:      [A]
support:  1
evidence: TSB 91-23-03, transaction 2070941, §"Update Programming" Section 1 (USB) and Section 2 (Software Version Management) — NHTSA filings MC-10239652-0001.pdf (released 2023-07-24) and MC-10252030-0001.pdf (released 2024-03-19)
read:     2026-09-27
decay:    6 months
unblocks: What doc/IDEA.md:92 requires an update mechanism to guarantee, and how far the MIB lineage is from the project's "owner can update and recover it, without us"
---

# How does a Volkswagen Group MIB3 infotainment unit receive a software update, and can it be rolled back?

By USB stick, in a hidden service mode, with a shop-grade battery charger
required. Not over the air. This is documented in Volkswagen Group of America
Technical Service Bulletin **91-23-03**, transaction 2070941, filed with NHTSA
twice — `MC-10239652-0001.pdf` released 2023-07-24 and `MC-10252030-0001.pdf`
released 2024-03-19. Both revisions carry the same procedure.

**The path, as written in the bulletin:**

- Build a USB drive with SD/USB Creator (per bulletin 2054866 / 00-19-04), using
  USB part number `3G0.919.360.TH`.
- Insert it into port 1 of the centre console USB connection.
- **Press and hold the "Menu" hard key on the MIB display until "Service Mode"
  is made available.** The mode is not in the normal interface.
- Software Update/Versions → Start Update → Update → "USB 1" → Start Update,
  confirmed a second time.
- A 70 TU labour operation for the USB path; the display switching off and on
  mid-update is stated as normal.

**Section 2 is SVM — Software Version Management** — which is the mechanism by
which a software version is selected or reverted. It is the rollback path, and
it is a dealer procedure rather than an owner-facing control.

**The charger requirement is not incidental.** The bulletin states that a
supply of **at least 70 A charging current MUST be used to avoid problems
during the update and flash campaigns**. The MIB2 bulletin for the same
mechanism (`MC-10235136-0001.pdf`, released 2023-04-17) names the equipment —
Midtronics InCharge 940 (INC 940) or GRX3000VAS, connected to the vehicle's
12 volt battery — and specifies holding the Menu button for **10 seconds** to
reach Service Mode.

Both bulletins list "Infotainment system will not turn on/boot up" among the
customer-reported symptoms the update is intended to resolve, which is
consistent with the charger requirement being a brick-risk control.

**Implication:** measured against `doc/IDEA.md:92` — "an update can always be
completed or rolled back; a failed one cannot leave the unit unbootable" — the
MIB3 mechanism satisfies the rollback half and fails the others. It needs a
prepared USB stick, it needs external power the owner is told to source as shop
equipment, and the mode that performs it is deliberately hidden. A failed
update presenting as "will not turn on" is a documented failure mode, not a
hypothetical one. This is the behaviour `doc/IDEA.md:96` — "the owner can
update it, and can recover it, without us" — is written against.

**Reverses if:** VW ships OTA to the MIB3 population. One reading of the VW
customer-facing software-update page settles it; roughly ten minutes. Note the
in-window signal pointing the other way: MIB3 is the generation being
superseded, so the more likely change is that MIB3 loses updates rather than
gains them.

## Revisit log

- 2026-09-27 — initial finding, [A] from TSB 91-23-03, both NHTSA revisions read
