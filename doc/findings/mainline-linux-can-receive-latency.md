---
question: What receive latency does mainline Linux achieve on SocketCAN, loaded and unloaded, and does the project need PREEMPT_RT or a separate MCU?
answer: CTU FEE have published continuous daily CAN latency measurements since April 2023, reporting that mainline is typically around 0.1 ms unloaded, with maximums near 1 ms for an in-kernel gateway and near 3 ms for a userspace gateway under load. PREEMPT_RT improves the unloaded maximum to about 0.2 ms but does not remove loaded tail spikes. SocketCAN's receive path runs in a softirq shared with every other network device, which is why it cannot be priority-boosted the way a character driver's hard IRQ can.
tag:      [B]
support:  1
evidence: "CTU FEE CAN latency team, embedded world 2024 paper — mainline unloaded typical 0.1 ms, in-kernel gateway loaded max ~1 ms, userspace gateway loaded max ~3 ms even at SCHED_RR priority 80, PREEMPT_RT unloaded max ~0.2 ms with glitches to 0.45 ms. linux-can mailing list, 2024-03, CTU team — threaded NAPI plus priority boost gives under 1 ms in most cases, some runs still 10 ms. Sojka and Pisa, RTLWS 2011 — SocketCAN employs a soft-IRQ shared by all network devices where LinCAN does its RX in hard-IRQ context, and the tuning that fixes LinCAN does not affect SocketCAN. PREEMPT_RT mechanism at kernel.org/doc/html/next/core-api/real-time/theory.html [A]. Volkswagen Group Research in-kernel Linux CAN gateway evaluated by CTU FEE in 2011 is the production precedent. Support is 1 because these are one research group's publications, not independent groups. No figure was measured on this project's target hardware."
read:     2026-09-27
decay:    never
unblocks: The MCU-versus-Linux decision, and whether PREEMPT_RT is a dependency of the core
---

# What receive latency does mainline Linux achieve on SocketCAN, loaded and unloaded, and does the project need PREEMPT_RT or a separate MCU?

**None of these numbers were measured on this project's hardware**, which is
the single most important caveat and the reason this is `[B]` and not `[A]`.
Benchmarks do not port. They were taken on CTU FEE's test hardware, not on a
4-core ARM SBC, and the relevant differences between those two are memory
bandwidth and thermal behaviour, not throughput.

Subject to that, the CTU FEE CAN latency team have run continuous daily
measurements since April 2023 and report:

- **Mainline, unloaded** — typically around **0.1 ms**.
- **Mainline, loaded** — maximum around **1 ms** for an in-kernel gateway,
  around **3 ms** for a userspace gateway, the latter even at `SCHED_RR`
  priority 80.
- **PREEMPT_RT, unloaded** — maximum around **0.2 ms**, with glitches to
  0.45 ms.
- **PREEMPT_RT, loaded** — under 1 ms in most cases with threaded NAPI and
  priority boost, but some runs still show 10 ms spikes.
- For reference on the same hardware, RTEMS achieves roughly 60 µs under load.

**The mechanism is the part that generalises.** From Sojka and Pisa at RTLWS
2011: LinCAN does all RX processing in hard-IRQ context, whereas SocketCAN
employs a **softirq shared by all network devices**. The same authors record that
tuning which brings LinCAN down to unloaded values — boosting the IRQ thread
priority — leaves SocketCAN completely unaffected. So the mitigation is threaded
NAPI (`echo 1 > /sys/class/net/can0/threaded`, then `chrt -f` the resulting
`napi/can0-*` thread), which converts the softirq into a schedulable thread.
That is a tunnelling, not a fix; the shared-softirq property is structural.

**The precedent is the strongest single argument here.** Volkswagen Group
Research built an in-kernel Linux CAN gateway in 2011 — the same vehicle
generation as this project's target — and had CTU FEE evaluate it. That is a
real manufacturer having concluded Linux was sufficient for this class of
workload, on this class of car.

**Implication:** an MCU gateway is not needed, and PREEMPT_RT is not needed
either. The arithmetic that settles it is in `passat-b55-can-bus-bitrates.md`:
messages repeat every 10-25 ms, and a maximally-occupied frame takes about
256 µs on a 500 kbit/s wire, so the project is 40 to 100 times slower than the
bus and has no deadline to miss. It is also a consumer, not a control loop —
`doc/IDEA.md:62` scopes assistance features as warning-only and lines 113-115
exclude actuating braking, steering and throttle-influencing cruise. What the
core does need is `SCHED_FIFO` on the core process, threaded NAPI with the CAN
NAPI thread boosted, a modest receive ring, and `mlockall`; that buys the
~1 ms loaded figure against a 10 ms budget. PREEMPT_RT earns its cost only if a
control function is added later, and it carries real regression risk — the same
team caught a VFP breakage across all ARM-32 in 6.3 and a latency regression in
6.8.0-rc1-rt1.

**Reverses if:** a sub-millisecond *worst-case* requirement appears under
adversarial network load, which the measurements above do not promise. CTU FEE
publish and document their own test harness, so running it against the actual
target board settles this in an afternoon — cheaper than any further reading,
and the one measurement that would upgrade this finding's transferability.

## Revisit log

- 2026-09-27 — initial finding, `[B]` from CTU FEE publications, none measured on target hardware
