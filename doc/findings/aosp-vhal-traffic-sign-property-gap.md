---
question: Does the AOSP VHAL define a property for traffic sign recognition or speed limit?
answer:  No. Searching the complete VehicleProperty.aidl for speed_limit, traffic_sign and road_event returns zero matches, while the ADAS range 0x1000–0x1100 does define blind spot, lane departure, forward collision, adaptive cruise and driver-attention states. The interface has ADAS properties and no traffic sign property, so a sign-recognition result has no VHAL representation to arrive on.
tag:      [A]
support:  1
evidence: VehicleProperty.aidl @ 1a56e38 — grep -niE "speed_limit|traffic_sign|road_event" over the full 6537-line file returns no match; ADAS properties occupy 0x1000–0x1100
read:     2026-09-27
decay:    6 months
unblocks: Whether traffic sign recognition is ahead of the documented interface or is a gap in it, for doc/IDEA.md:62
---

# Does the AOSP VHAL define a property for traffic sign recognition or speed limit?

No. This is a negative established by exhaustive search of the property
definition file, not an inference from silence elsewhere.

The ADAS property range `0x1000`–`0x1100` in the same file does define
`BLIND_SPOT_WARNING_ENABLED` / `_STATE`,
`LANE_DEPARTURE_WARNING_ENABLED` / `_STATE`, `FORWARD_COLLISION_WARNING_*`,
`LANE_KEEP_ASSIST_*`, `ADAPTIVE_CRUISE_CONTROL_LEAD_VEHICLE_MEASURED_DISTANCE`,
`DRIVER_DISTRACTION_*` and `DRIVER_Drowsiness` / hands-on-detection states.
A search of the whole file for `speed_limit`, `traffic_sign` and `road_event`
returns nothing.

The shape of what is defined is itself informative. The ADAS properties that
exist are mostly a boolean or small state enum — whether a warning is
*enabled*, and what state it is *in* — plus one measured distance. The
interface carries the conclusion of perception performed elsewhere, not the
recognition itself.

**Implication:** a head unit running AAOS cannot be handed a sign-reading
result through the VHAL as it stands. This is the gap `doc/IDEA.md:62` names,
and it is a gap in a documented, widely implemented interface rather than in
one vendor's product. It also means the same absence would apply to a
property this project defines itself: a new sign property would have no
existing ID space convention beyond the 0x1000–0x1100 ADAS range it would sit
beside.

**Reverses if:** an AOSP release adds a sign or speed-limit property. One
grep of the same file settles it; roughly a minute.

## Revisit log

- 2026-09-27 — initial finding, [A] L6 absence, from `VehicleProperty.aidl` @ `1a56e38`
