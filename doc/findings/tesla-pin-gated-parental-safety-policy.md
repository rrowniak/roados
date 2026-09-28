---
question: Can a Tesla owner apply a PIN-gated policy that limits the driver's speed and acceleration and locks safety settings against change?
answer:  Yes. Parental Controls, in the Safety & Security Settings section of both the Model 3 and the Model S/X UK manuals at software 2026.14, offers Limit Speed, Reduce Acceleration, Require Safety Features and an overnight curfew, all behind a 4-digit PIN the owner sets on the touchscreen. The policy is also drive-constrained — the vehicle must be in Park, safety settings cannot be changed while driving — and cannot be disabled from the mobile app.
tag:      [A]
support:  2
evidence: Model S/X UK 2026.14 §Parental Controls — "Require a PIN to access the parental controls. The PIN must be 4 digits." with Limit Speed, Reduce Acceleration, Require Safety Features and a 23:00-04:00 curfew to paired phone keys. Model 3 UK 2026.14 §Parental Controls — "You must set a 4-digit PIN. Touch Require Safety Features to turn on or off the following safety features..." Model S/X UK — "If Valet Mode and Speed Limit Mode are enabled, the PIN prompt will not appear when the Parental Controls screen is opened."
read:     2026-09-27
decay:    6 months
unblocks: Whether a second-driver or valet policy belongs in the comfort section of doc/IDEA.md, which currently lists only heated seats and automatic headlights
---

# Can a Tesla owner apply a PIN-gated policy that limits the driver's speed and acceleration and locks safety settings against change?

Yes. Parental Controls appear in §Safety & Security Settings in both UK
manuals at software 2026.14. Both require a 4-digit PIN set by the owner, both
list the same four capabilities, and neither is reachable while driving.

**Limit Speed** — the speedometer shows the set limit and a chime sounds when
it is exceeded, so the enforcement is audible rather than intervention-based.
On the Model S/X the limit is expressed as an *offset* from the detected speed
limit sign, −5 to +5 mph, not as an absolute value. **Reduce Acceleration** —
limits acceleration by measuring how quickly the accelerator pedal is applied,
and chimes when the limit is exceeded. **Require Safety Features** — forces a
specified set of driver-assistance features to stay on, per-model:

| In the "Require Safety Features" list | Model 3 UK | Model S/X UK |
|---|---|---|
| AEB, Obstacle-Aware Acceleration, Auto. Blind Spot Camera | yes | yes |
| Blind Spot Collision Warning Chime | yes | yes |
| Lane Departure Avoidance → Assist | yes | yes |
| Speed Limit Warning → Chime | yes | yes |
| Forward Collision Warning → Early | yes | yes |
| Allow Mobile Access | yes | yes |
| Park Assist Chimes | yes | — |
| **Speed Limit → Relative** | — | **yes** |
| **Speed Limit → Offset +5 mph** | — | **yes** |

**Curfew** — a time window, default 23:00 to 04:00, configurable; outside it a
paired phone key starts a countdown and the paired phones are notified.

**Three properties make it a policy rather than a preference.** The PIN is the
boundary and the owner sets it — not an account password, not a service
credential, a 4-digit touchscreen secret guarding a deliberately low-value
capability. The threat model is a borrower's thumb on a touchscreen, not an
attacker. The policy is *drive-constrained*, not merely permission-gated: the
vehicle must be in Park to change safety settings, and on the Model S/X the
manual is explicit that if Valet Mode and Speed Limit Mode are enabled the PIN
prompt does not appear at all on the Parental Controls screen — the policy
prompt is deliberately deprioritised below an already-active mode. That is a
real interaction between two gating mechanisms, invisible until implemented.
And the remote path is one-way: the manual states parental controls cannot be
disabled from the mobile app. The app is on the paired-phone side of the trust
boundary, so it can receive a curfew notification but never undo the policy.
The touchscreen is the only place the PIN is honoured.

**Implication:** `doc/IDEA.md` §Comfort (L74-78) currently offers heated seats
and automatic headlights, plus the not-yet-implemented "driver profile
selection with key and PIN-gated safety policy". This finding says that idea
already ships, from one manufacturer, on a touchscreen, with no hardware beyond
what the car has. The transferable design is four separable elements: a mode
off by default and engaged by a physical action; policy expressed as a *ceiling
rather than an override*; a per-model capability list; and a remote path that
can only tighten, not loosen. Element 2 is the interesting one and differs
from conventional valet modes, which limit speed by intervening. Tesla's does
not intervene at all — it chimes and shows the limit. Enforcement is the
driver's knowledge of the boundary, and the policy constrains the car's own
assistance, not its brakes.

**Reverses if:** a later version changes the PIN requirement or allows
parental-control changes from the app; a US or other-market manual documents a
materially different control set, in particular one gating on the *key* rather
than a touchscreen PIN; or a different mechanism exists, such as a
driver-profile PIN prompt at vehicle entry. Reading §Safety & Security Settings
in one manual is minutes.

## Searched

Model 3 UK 2026.14 §Safety & Security Settings and §Parental Controls; Model
S/X UK 2026.14, same two sections; Model 3 NA 2024.44.25.3 for `parental`,
`PIN`, `curfew`, `Limit Speed`, `Reduce Acceleration`, `Require Safety
Features`, `Valet`.

Not covered: two manuals from one publisher, so this is a Tesla pattern rather
than a surveyed-industry pattern; the US manual was not searched for this
section, so availability of Parental Controls in the US is not established
either way; Model Y and Cybertruck were not searched; and the manual does not
say whether the policy binds to a specific key or profile, which is the
question a project with key-based driver profiles would actually need answered.
Curfew requires a paired phone; the other three controls do not.

## Revisit log

- 2026-09-27 — initial finding, [A] L4 direct quotation, two manuals
