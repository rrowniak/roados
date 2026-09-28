---
question: Does Tesla publish any user-interface performance figure — frame rate, touch input latency, or cold boot time?
answer:  No. Zero occurrences of frame rate, frames per second, fps, input latency, touch latency, latency, response time, boot time, cold boot, startup time, milliseconds or benchmark across 171 manual pages in three manuals. The only published duration is the touchscreen restart procedure, which is a component restart and explicitly not a cold boot. The load-time figures in circulation are practitioner measurements of a third-party browser application, which measure neither Tesla's code nor Tesla's hardware.
tag:      [A]
support:  2
evidence: Zero occurrences of the term list across 171 pages of Model 3 NA 2024.44.25.3, Model 3 UK 2026.14 and Model S/X UK 2026.14. The single duration, Model 3 NA 2024.44.25.3 §Touchscreen — "Tesla recommends that you restart the touchscreen to address any unresponsive or sluggish behavior. Wait about 30 seconds for the touchscreen to restart."
read:     2026-09-27
decay:    6 months
unblocks: doc/IDEA.md:85-87 and 128 — the "Tesla-level or better" responsiveness bar
---

# Does Tesla publish any user-interface performance figure — frame rate, touch input latency, or cold boot time?

No. Searched all 171 pages of the three-manual corpus for `frame rate`,
`frames per second`, `fps`, `refresh rate`, `input latency`, `touch latency`,
`latency`, `response time`, `boot time`, `cold boot`, `startup time`,
`milliseconds`, `benchmark`, `p60`, `p95`. No hit for any of them. The `ms`
hits are the word *systems* and the plural of *system* in prose. This is an L6
absence over a complete primary corpus.

**The one number that is not a number.** The only duration Tesla publishes is
in §Touchscreen:

> Tesla recommends that you restart the touchscreen to address any
> unresponsive or sluggish behavior. Wait about 30 seconds for the touchscreen
> to restart.

Three things disqualify it, and all three matter. It is a **restart, not a
boot** — the context is a remedy for an unresponsive screen, not a
specification for time-to-first-frame, and the same section tells the owner how
to *force* a restart and notes that a software update resets the screen
automatically. The **system under measurement is unstated**, so thirty seconds
for a component restart is not comparable against a cold boot and is not a
claim that the car takes thirty seconds to start. And **"sluggish" is the
trigger, not the measurement** — the manual acknowledges a responsiveness
problem and offers a repair, without quantifying either. The S/X UK 2026.14
manual carries the same procedure.

**Two false positives worth naming.** A sweep for responsiveness terms returns
eleven hits, all in §Display, all about layout: how the interface reflows
across a centre display, a rear display, a rear passenger display, or none.
`Responsive` there means "adapts to the display arrangement" — a naming
collision, not a performance claim. And the load-time figures that do circulate
for Tesla infotainment are, consistently, measurements of a third-party
application — a community web app served from a browser inside the car's own
browser — taken by the people who built it. They measure the app, the browser,
the vehicle's LTE throughput and a network path to a server Tesla does not
operate. They do not measure Tesla's UI framework, its renderer or its
hardware. The largest and most reproducible number in the public domain is a
measurement of a third party's code.

**Implication:** `doc/IDEA.md:85-87` requires interface responsiveness and
acknowledges "Tesla-level or better" as the bar. On this evidence, *Tesla-level*
is not a number — it is an impression this survey's practitioners formed from a
third-party app on Tesla hardware. A responsiveness target cannot be derived
from Tesla, because there is nothing to derive it from, and a bar that is a
qualitative impression is not a specification: it cannot be met or failed on
evidence. This is not an argument against the target, it is an argument for
restating it in verifiable terms or accepting it as a judgement call. What the
evidence does *not* support is quoting a figure to Tesla. A cold-boot figure,
by contrast, is a well-defined instrumented measurement a vehicle project can
take on its own hardware in an afternoon — Tesla need not publish one for
roados to have its own.

**Reverses if:** Tesla publishes an interface performance specification or a
boot-time figure, in a manual or elsewhere; or a later manual version documents
interface timing; or a controlled measurement of Tesla's *own* interface —
not a third-party app — becomes available, reproducible and attributable to the
system rather than the browser environment. The third is the likeliest, and it
would change the finding from "no figure exists" to "a figure exists but was
not published by the manufacturer", which are different claims.

## Searched

171 manual pages as text across the three manuals, for the full term list
above, plus a direct read of §Touchscreen and §Display. Outside the manuals,
Tesla's public infotainment material carries no interface timing
specification; third-party app measurements are excluded from the corpus for
the reasons given.

## Revisit log

- 2026-09-27 — initial finding, [A] L6 absence, 171 manual pages
