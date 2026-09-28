---
question: Does the AOSP VHAL define a property for conventional 12 V battery voltage?
answer:  No. A search of the full VehicleProperty.aidl for VOLTAGE returns zero matches. Every battery-related property is EV-specific — EV_BATTERY_LEVEL, EV_BATTERY_AVERAGE_TEMPERATURE, EV_BATTERY_INSTANTANEOUS_CHARGE_RATE and EV_BATTERY_DISPLAY_UNITS. The conventional 12 V battery has no property, so doc/IDEA.md's battery-voltage line has no VHAL counterpart and would require a new property to be defined.
tag:      [A]
support:  1
evidence: VehicleProperty.aidl @ 1a56e38 — grep -oE "[A-Z_]*VOLTAGE[A-Z_]*" over the full file returns no match; battery properties are EV_BATTERY_AVERAGE_TEMPERATURE, EV_BATTERY_DISPLAY_UNITS, EV_BATTERY_INSTANTANEOUS_CHARGE_RATE, EV_BATTERY_LEVEL
read:     2026-09-27
decay:    6 months
unblocks: Whether doc/IDEA.md's vehicle-data list maps onto the VHAL or needs extending for battery voltage
---

# Does the AOSP VHAL define a property for conventional 12 V battery voltage?

No. The absence is total rather than partial: searching the full
`VehicleProperty.aidl` for any identifier containing `VOLTAGE` returns zero
matches.

The four battery properties in the interface are all conditioned on the
traction battery — `EV_BATTERY_LEVEL`, `EV_BATTERY_AVERAGE_TEMPERATURE`,
`EV_BATTERY_INSTANTANEOUS_CHARGE_RATE`, `EV_BATTERY_DISPLAY_UNITS`. Related
properties such as `EV_CHARGE_STATE` and `EV_BATTERY_CAPACITY` are likewise
EV-scoped. The interface models the high-voltage system and does not model
the 12 V system at all.

**Implication:** `doc/IDEA.md:35` lists battery voltage among the vehicle data
the first vehicle should show, and the Passat B5.5 is a conventional 12 V car,
so this is the case that actually matters to the project's first target. There
is no existing property to copy, no existing ID to conform to, and no
reference implementation to read. If the project wants a VHAL-shaped
vocabulary for it, the property has to be defined, and defining one is a
different kind of work from adopting one — it carries an ABI that other
implementations would then depend on.

**Reverses if:** an AOSP release adds a 12 V or non-EV battery property. One
grep settles it; roughly a minute.

## Revisit log

- 2026-09-27 — initial finding, [A] L6 absence, from `VehicleProperty.aidl` @ `1a56e38`
