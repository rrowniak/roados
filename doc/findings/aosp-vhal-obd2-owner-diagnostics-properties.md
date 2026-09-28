---
question: Does the AOSP Vehicle Hardware Abstraction Layer define properties that expose OBD2 diagnostics — live values, freeze frames, fault codes, and clearing — to an infotainment application?
answer:  Yes. Four properties at 0x0D00–0x0D03 define live OBD2 sensor values, freeze-frame data whose stringValue may carry a DTC, an index of freeze frames held in vehicle memory, and a write that clears them. All four are @version 2. Three are read-only; the clear is the only writable one. The clear deletes freeze frames, not fault codes, and is VehicleArea:GLOBAL rather than per-module.
tag:      [A]
support:  1
evidence: automotive/vehicle/aidl_property/android/hardware/automotive/vehicle/VehicleProperty.aidl @ 1a56e38, L4123–4232 — OBD2_LIVE_FRAME = 0x0D00 (L4164), OBD2_FREEZE_FRAME = 0x0D01 (L4191), OBD2_FREEZE_FRAME_INFO = 0x0D02 (L4209), OBD2_FREEZE_FRAME_CLEAR = 0x0D03 (L4232)
read:     2026-09-27
decay:    6 months
unblocks: Whether the diagnostics section of doc/IDEA.md can be modelled on a published interface rather than invented from scratch
---

# Does the AOSP VHAL define properties that expose OBD2 diagnostics to an infotainment application?

Yes, and in more detail than the property names suggest. The AIDL specifies
access mode, change mode, wire semantics and the failure cases, including the
awkward ones.

| Property | ID | Access | @version | Semantics |
|---|---|---|---|---|
| `OBD2_LIVE_FRAME` | 0x0D00 | READ | 2 | "Reports a snapshot of the current (live) values of the OBD2 sensors available." |
| `OBD2_FREEZE_FRAME` | 0x0D01 | READ | 2 | sensor values "at the time that a fault occurred and was detected"; `stringValue` "may contain a non-empty diagnostic troubleshooting code (DTC)" |
| `OBD2_FREEZE_FRAME_INFO` | 0x0D02 | READ | 2 | the freeze frames "stored in vehicle memory", each `int64Values` element a fault timestamp |
| `OBD2_FREEZE_FRAME_CLEAR` | 0x0D03 | **WRITE** | 2 | deletion of stored freeze frames |

All four carry `VehiclePropertyGroup:SYSTEM, VehicleArea:GLOBAL,
VehiclePropertyType:MIXED` and `VehiclePropertyChangeMode.ON_CHANGE`.

**The two documented `NOT_AVAILABLE` cases** are both worth knowing before
designing against this:

- a freeze-frame read for a timestamp with no stored frame must return
  `NOT_AVAILABLE`; and because "vehicles may have limited storage for freeze
  frames", a read can return `NOT_AVAILABLE` *even though the timestamp was
  just listed* by `OBD2_FREEZE_FRAME_INFO`. A listing is not a guarantee of
  retrievability.
- `OBD2_FREEZE_FRAME_CLEAR` advertises selective deletion in
  `configArray[0]` — 1 if individual frames can be cleared by timestamp, 0
  otherwise. A `set` with no `int64Values` clears all; with one or more, it
  clears only those and "should the vehicle not support selective clearing …
  this latter mode must return `NOT_AVAILABLE`".

**This does not fully cover doc/IDEA.md's diagnostics list, and the gaps are
specific rather than general:**

- *Fault codes* — covered, but only as the `stringValue` of a freeze frame, so
  a code is reachable only alongside the frame that produced it.
- *Fault reset, per module, codes exported first* (IDEA.md:42) — **not
  covered.** The only write clears freeze frames; it does not clear DTCs, and
  it is `VehicleArea:GLOBAL`, not per-module. There is no fault-code reset
  property in this range.
- *Measurement blocks, read on demand* (IDEA.md:43) — partially. This is a
  snapshot of the sensors the vehicle chooses to expose, not an addressable
  request for an arbitrary measurement block.
- *Recording of measurements over time* (IDEA.md:44) — a client-side concern;
  the property supplies snapshots to record.

**Implication:** the project's diagnostics vocabulary has a published,
Apache-2.0-licensed interface to borrow — including the failure modes, which
are the part a from-scratch design would get wrong. But `doc/IDEA.md:42` is
strictly harder than this range: per-module fault reset with prior export is
not in the VHAL, and adopting this interface means designing that part
independently.

**Reverses if:** a later AOSP release adds DTC-clear or per-module diagnostic
properties to the 0x0D00 range. Re-checking the AIDL is a single fetch and
diff; roughly ten minutes.

## Revisit log

- 2026-09-27 — initial finding, [A] from `VehicleProperty.aidl` @ `1a56e38`
