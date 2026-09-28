---
question: Does any production head unit implement the VHAL OBD2 diagnostic properties 0x0D00–0x0D03?
answer:  Not established. The properties are specified in the AIDL, but specification is L1 — a statement of intent. No implementation was located, and the two attempts to check one both failed to fetch, so this is a gap in the research rather than a verified negative. The correct statement is that no implementing HAL was found, not that none exists.
tag:      [C]
support:  1
evidence: Negative result. Two fetch failures: automotive/vehicle/hal/default/VehicleHalManager.cpp and VehiclePropertyUtils.cpp returned 0 bytes; the directory listing for automotive/vehicle/hal/default/ returned "Object is not found" at commit 1a56e38
read:     2026-09-27
decay:    6 months
unblocks: Whether the OBD2 range is adoptable as a working interface or only as a reference document
---

# Does any production head unit implement the VHAL OBD2 diagnostic properties?

**Not established.** The honest answer is a gap, and it should not be recorded
as a verified negative.

What is established: the four properties are specified in `VehicleProperty.aidl`
at `0x0D00`–`0x0D03` — see
`aosp-vhal-obd2-owner-diagnostics-properties.md`. That is an L1 fact, a
specification. Per `.ai/protocols/evidence.md:75-91`, L1 is what the project
*intends*; L3, the source file and its tests, is the honest test of support.
No implementation was read.

**Two attempts to check the reference implementation both failed to fetch:**

1. `automotive/vehicle/hal/default/VehicleHalManager.cpp` and
   `VehiclePropertyUtils.cpp` over `?format=TEXT` returned 0 bytes after
   base64 decode — empty, so no grep result can be drawn from them.
2. Listing `automotive/vehicle/hal/default/` returned
   `[type.googleapis.com/google.rpc.LocalizedMessage] message: "Object is not
   found"` at commit `1a56e38`. The path does not exist at that location in
   the current tree; the AOSP VHAL implementation is located elsewhere.

A failed fetch is a null result reported as a null result. It is not evidence
of absence, and it must not be cited as though the files had been read.

**A related, genuinely established negative, from a different direction:** the
OEM owner-facing survey conducted 2026-09-27 found that no infotainment system
among BMW iDrive, Mercedes MBUX, Audi MMI, VW MIB, Polestar, Toyota,
Hyundai/Kia, CarPlay or AAOS surfaces fault codes, measurement values or
per-module reset to the owner. BMW routes this to ISTA and Mercedes to XENTRY.
The one product that does expose it is OBDeleven, which reaches DTCs over
OBD-II from a phone and a Bluetooth dongle — not a head unit. That is
L5-adjacent and supports "uncommon", but it is a statement about owner
interfaces, **not** about HAL implementations, and the two must not be
conflated.

**Implication:** adopt the 0x0D00 range as a reference vocabulary, which
`aosp-vhal-obd2-owner-diagnostics-properties.md` supports at `[A]`. Do not
assume a working implementation to copy. The interface's *semantics* are
settled; its *integration* is not.

**Would be settled by:** locating the current AOSP VHAL implementation tree
and grepping it for the 0x0D00 properties, then checking whether any
production AAOS OEM exposes them. The first is an hour of path-hunting; the
second is not publicly answerable, since OEM HAL sources are proprietary, so
it would need a car or a third-party teardown report.

## Searched

- `automotive/vehicle/hal/default/` tree listing at `1a56e38` — "Object is not
  found"
- `VehicleHalManager.cpp`, `VehiclePropertyUtils.cpp` at the same path — 0
  bytes returned
- `VehicleProperty.aidl` full-file grep for `OBD2|FREEZE_FRAME|DIAGNOS|EMISSION|DTC|FAULT`
  — 10 hits, all in the 0x0D00 range definition and the property-name docs;
  no implementation site
- OEM owner-facing survey across nine systems, 2026-09-27 — no owner-facing
  fault-code surface found anywhere; see the OBDeleven precedent

## Revisit log

- 2026-09-27 — initial finding, [C] null result, not established
