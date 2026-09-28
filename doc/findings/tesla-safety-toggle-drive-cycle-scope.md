---
question: Can a Tesla owner silence a driver-safety feature, and does the silence persist across drives?
answer:  Yes for the current drive, and no beyond it. Three separate features — Driver Drowsiness Warning, Intelligent Speed Assist and Sentry Mode — can be silenced from the touchscreen, and every one re-arms automatically at the start of the next drive cycle. Intelligent Speed Assist states outright that its audible warnings can be muted every drive cycle but not disabled. No permanent owner-side mute exists for any of the three.
tag:      [A]
support:  2
evidence: Model S/X UK 2026.14 §Driver Drowsiness Warning — "...for the current drive cycle (every time the vehicle is in Park and you walk away). Driver Drowsiness Warning automatically re-enables at the start of every drive cycle." Same sentence with the setting named, Model 3 UK (snapshot 2024-05-30): "by touching Controls > Safety > Driver Drowsiness Warning for the current drive cycle". Model S/X UK 2026.14 §Intelligent Speed Assist — "The audible speed limit warnings can be muted every drive cycle but not disabled." and "The speeding warnings automatically re-enable at the start of every drive cycle."
read:     2026-09-27
decay:    6 months
unblocks: Whether a per-drive-cycle scope is the right default for a warning-only feature needing no new hardware
---

# Can a Tesla owner silence a driver-safety feature, and does the silence persist across drives?

Yes for the current drive, and no beyond it. Three features, two manuals, one
rule: a mute is a property of the drive, not of the car.

**Driver Drowsiness Warning** (Model S/X UK 2026.14) — "...for the current
drive cycle (every time the vehicle is in Park and you walk away). Driver
Drowsiness Warning automatically re-enables at the start of every drive
cycle." The Model 3 UK manual states the same sentence with the setting named:
*touch Controls > Safety > Driver Drowsiness Warning for the current drive
cycle*.

**Intelligent Speed Assist** (Model S/X UK 2026.14) — "The audible speed
limit warnings can be muted every drive cycle but not disabled." And on the
mute action: "To mute the speeding chime and display for the rest of the
drive, press the speaker icon located at the top of the touchscreen. The
speeding warnings automatically re-enable at the start of every drive cycle
(every time the vehicle is in Park and you walk away)."

**Sentry Mode** (Model S/X UK 2026.14, §Touchscreen) — "...is parked, touch to
manually enable or disable Sentry Mode for the current drive cycle. To
automatically turn Sentry Mode on (or off) every time you leave your vehicle,
enable the setting from..." and "..., the shortcuts on the vehicle's
touchscreen and mobile app will only work for the current drive cycle."

**Why the mechanism is worth taking.** Tesla resolves the tension between
"the driver knows best about the next hour" and "the driver must not silence
this permanently" by scoping the opt-out to the trip. Three properties follow,
each of which a project implementing a warning feature would have to choose
deliberately. There is no sticky state: no arrangement of settings leaves a
warning off, and the re-arm is not a reset or a revert to a previous value but
a new trip beginning. The re-arm is automatic and unconditional — no menu, no
setting, no advance warning. And a separate permanent switch exists for the
opposite decision, which is what makes the pattern coherent rather than merely
restrictive: in Sentry Mode the current-drive-cycle control is the *temporary*
one, and a separate setting decides the default. The manual is explicit that
the shortcuts "will only work for the current drive cycle". The temporary
control is not a weaker version of the real setting; it is a different thing,
deliberately scoped so it cannot be misused to defeat the setting.

**Implication:** `doc/IDEA.md` has no driver-monitoring section and does not
ask whether a warning can be silenced. This finding supplies an answer to a
question the design will eventually face, at no hardware cost. The transferable
design: a warning feature with no new hardware, whose owner may mute the
warning for the current trip, whose mute re-arms at the start of every trip,
and whose default is set by a separate permanent control. That is safer than a
binary on/off toggle, and adoptable as a default rather than as a feature. The
counter-reading belongs in the record: a driver annoyed by a chime re-mutes it
every drive, and the re-arm becomes theatre. Tesla accepted that cost on the
grounds that a permanent mute is worse, which is a judgement about the feature
and not a general result.

**Reverses if:** a manual documents a permanent owner-side mute for any of the
three; or a later version changes "for the current drive cycle" to a
persistent setting or adds a user-chosen mute duration; or the app gains a
persistent toggle the manual does not document. The last is the least
unlikely — Tesla has shipped features ahead of its manuals. Re-reading two
sections of one manual is minutes.

## Searched

Model S/X UK 2026.14 §Driver Drowsiness Warning, §Intelligent Speed Assist,
§Touchscreen; Model 3 UK §Driver Drowsiness Warning; Model 3 NA 2024.44.25.3
for `current drive cycle`, `every drive cycle`, `not disabled`, `muted`,
`re-enable`, `silence`, `off permanently`. No occurrence of a permanent or
persistent mute for any of the three.

Not covered: this is a Tesla pattern, not a surveyed-industry pattern — no
second manufacturer's primary document was consulted. Sentry Mode is included
as the clearest demonstration of the mechanism, **not** as a safety feature.
Model Y, Cybertruck and the FSD-supervised variants were not searched, and
Driver Drowsiness Warning is not Driver Monitoring: a driver-scoring or
gaze-tracking feature may be governed by separate supervision rules.

## Revisit log

- 2026-09-27 — initial finding, [A] L4 direct quotation, two manuals
