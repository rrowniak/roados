---
question: What are the CAN bus bitrates on a 2003 Volkswagen Passat B5.5, and how often do messages repeat?
answer: VW SSP 269 specifies 500 kbit/s for the drivetrain CAN and 100 kbit/s for the comfort and infotainment CAN from MY2000, with the older comfort bus at 62.5 kbit/s. It states message repetition generally falls in a 10-25 ms range, which is what sets the staleness budget the UI can expect.
tag:      [B]
support:  1
evidence: "VW SSP 269, Data transfer on CAN data bus II, read 2026-09-27 via the vaglinks.com PDF mirror rather than a Volkswagen-hosted original. Drivetrain CAN 500 kbit/s; convenience CAN 100 kbit/s; infotainment CAN 100 kbit/s from MY2000, permitted to share the comfort wire pair where rates match; legacy convenience bus 62.5 kbit/s; message repeat rate generally 10-25 ms. SSP 269 also records that the convenience/infotainment bus is single-wire-fault-tolerant by design, with independent drivers and no shared resistors."
read:     2026-09-27
decay:    stable
unblocks: Core sizing, and the bus-load arithmetic that determines whether the UI's 10 ms staleness budget is achievable
---

# What are the CAN bus bitrates on a 2003 Volkswagen Passat B5.5, and how often do messages repeat?

Volkswagen's own self-study program **SSP 269**, "Data transfer on CAN data bus
II", gives the rates: **500 kbit/s** for the drivetrain CAN, and **100 kbit/s**
for the convenience and infotainment CAN from MY2000. The pre-MY2000
convenience bus ran at 62.5 kbit/s. Infotainment may share the comfort wire pair
where the rates match, which is why the count of buses and the count of wire
pairs are not the same question.

The figure that matters most for this project is the repetition rate: messages
repeat "generally in a range of **10 - 25 ms**". That is the period the UI is
being fed at, and it is two and a half times looser than a 10 ms staleness
budget would assume as a worst case.

**The load arithmetic, marked as inference.** One maximally-occupied 8-byte
frame is 128 bits including stuffing. At 10 ms that is 12.8 kbit/s, about
**2.6 %** of a 500 kbit/s bus; at 25 ms, about 1.0 %. A powertrain bus carrying
15 to 25 such messages therefore plausibly runs at 25 to 60 %. The assumed
message count is the unverified input, and no published bus-load figure for the
B5.5 was found. This arithmetic is an inference from the rate and the frame
length, not a measurement, and must not be quoted as bus load.

One further SSP 269 detail is worth carrying: the convenience/infotainment bus
is **single-wire-fault-tolerant by design**, using independent drivers with no
shared resistors. A cut wire on that bus degrades rather than kills it, which
matters for a project that is tapping it non-invasively.

**Implication:** the core has to sustain 500 kbit/s and 100 kbit/s with a
10-25 ms worst-case input period, so the ~1 ms loaded mainline receive latency
measured elsewhere in this corpus is roughly an order of magnitude inside budget.
The 100 kbit/s comfort bus is the one to watch for saturation, not the faster
powertrain bus.

**Reverses if:** the same one-hour `candump` capture settles this too, and would
replace the inferred load with a measured percentage. Worth reading SSP 269 from
a Volkswagen-hosted original to raise this to `[A]`; the mirror is the only
reason it is `[B]`.

## Revisit log

- 2026-09-27 — initial finding, `[B]` from VW SSP 269 via a third-party PDF mirror
