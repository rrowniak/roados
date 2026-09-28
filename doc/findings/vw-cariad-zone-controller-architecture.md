---
question: What does CARIAD state about how Volkswagen Group decouples sensors from functions and centralises compute?
answer:  That sensors are "completely decoupled from the functions they perform", with a few zone controllers taking simple computation near the sensors and passing data over ethernet to central high-performance computers. It also states that in the traditional architecture a new function always brings a new control unit, and that the unified architecture arrives from the middle of the decade. This is CARIAD describing its own roadmap, so it is intent, not verified implementation.
tag:      [B]
support:  1
evidence: cariad.technology/de/en/news/stories/future-unified-architecture.html — "Sensors in the vehicle are then completely decoupled from the functions they perform. Instead, a few zone controllers will take over the computation of simple tasks"; and "A new function always brings with it a new control unit"
read:     2026-09-27
decay:    6 months
unblocks: Whether the coupling doc/IDEA.md's extension seam is designed to escape is a real and worsening problem in the target lineage
---

# What does CARIAD state about how Volkswagen Group decouples sensors from functions?

CARIAD describes the direction explicitly. Zone controllers sit near the
sensors, take on simple computation, and hand data to central high-performance
computers over ethernet. The stated motivation is the coupling that makes
software updates expensive: in the traditional architecture "more than 100
control units and various software communicate with each other", and "A new
function always brings with it a new control unit", so a software update for
one function must be applied across several control units, which they describe
as "an extremely time-consuming process that limits the convenience of
over-the-air updates".

The named components are the software platform **VW.OS**, the cloud
**VW.AC**, and a "Big Loop" by which data for automated-driving development is
collected. Availability is stated as "in Volkswagen Group vehicles from the
middle of the decade".

**This is intent, not implementation.** It is the vendor's own site describing
its own roadmap, which under `.ai/protocols/evidence.md:75-91` is L1 — `[A]`
for a specification, `[B]` for a capability claim. The claim is a claim about
the future, so it is tagged `[B]` and no higher. No shipping vehicle was
examined, and no teardown or service document was read to confirm the ethernet
backbone or the zone partitioning.

**What is independently corroborated, at `[A]`:** the MIB3 self-study program
filed with NHTSA describes the same decoupling one generation earlier, in
hardware. Information Electronics Control Module 1 J794 receives image data
"From driver assist systems control unit J1121 via LVDS", the overhead camera
data arrives over LVDS/Ethernet, and "Online traffic sign information" is
listed as an Audi connect service. So the pattern CARIAD describes as the
future is already how a shipping VAG head unit receives perception results:
from a separate ECU, over a dedicated high-bandwidth link, not off the vehicle
bus. That is a filed manufacturer document, and it is a different evidence
class from the CARIAD page.

**Implication:** this is the clearest available statement of the problem
`doc/IDEA.md`'s extension seam is aimed at. A function that needs its own
control unit is a function that cannot be added by adding software to
something that already exists — which is precisely the shape of the seam
described at IDEA.md:16-19, and the reason the VHAL, not a control unit, is
the more useful model. The "middle of the decade" framing also dates the
change, and the project targets a 2003 car, so nothing in this architecture is
available as a retrofit regardless.

**Reverses if:** a VAG service or training document describes the shipping
zone architecture with part numbers and bus topology. That would move this to
`[A]`; such documents are filed with NHTSA, and MIB3's was found there.

## Searched

- web search: "CARIAD software architecture zones infotainment MIB4 head unit
  2023 2024 Audi Volkswagen" — returned this page plus CARIAD product and news
  index pages; no zone-controller part numbers or topology found
- NHTSA filings for a VAG self-study program on the unified/zone architecture —
  not searched by filing number; MIB3's 2021 document was found and is
  cited above as the corroborating source

## Revisit log

- 2026-09-27 — initial finding, [B] vendor roadmap statement; [A] corroboration from NHTSA-filed MIB3 self-study program
