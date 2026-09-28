---
question: Can the Tesla software-release aggregators tesla-info and Not a Tesla App be used to determine which infotainment features apply to which car and hardware?
answer:  Not for model or hardware applicability. On a single tesla-info release page the same vehicle is listed under two different hardware vocabularies — "Seen on AP Hardware HW3" and "Seen on AP versions AP3 AP4" — in the same table, and the CPU field carries both "Intel Ryzen" and bare "Ryzen", so Intel and AMD are indistinguishable. Fleet-share figures on the same pages mix "(no cars)" with percentages. The feature text these sites quote is usable at [B]; the model, hardware and market gating attached to it is not usable at any confidence.
tag:      [A]
support:  1
evidence: Fetched 2026-09-27. tesla-info.com/release/2026.14.6.7 carries, on one page, "Seen on M3 MY / Seen on AP Hardware HW3 / Seen on MCU hardware Intel Ryzen" and "Seen on M3 MY / Seen on AP versions AP3 AP4 / Seen on MCU hardware Intel Ryzen", plus "Seen on MCU hardware Ryzen" with no vendor prefix, and uses both "MS" and "MS(2021+)" in model lists. tesla-info.com/release/2026.20 carries "(no cars)", "(2.28 % of cars)" and "(3.81 % of cars)" on one page, and all three vocabularies (HW1 HW2,5 HW3 HW4 PreHW1 / AP1 AP2,5 AP3 AP4 PreAP1 / Intel Ryzen / Ryzen). tesla-info.com/release/2026.27.6 has region lists truncated mid-token: "...LT LU LV MD MK MO MX MY NL NO PL PR PS PT RO R".
read:     2026-09-27
decay:    1 month
unblocks: Which claims in the 2025-2026 release-note layer of this survey may be cited at all, and at what tag
---

# Can the Tesla software-release aggregators tesla-info and Not a Tesla App be used to determine which infotainment features apply to which car and hardware?

Not for model or hardware applicability. **The ceiling is structural:** Tesla
publishes no release notes on the web — no official per-build changelog, no
compatibility matrix, no hardware-eligibility table. Every release-note claim
about a Tesla infotainment feature is therefore by construction a third
party's transcription of an in-car artefact that cannot be opened
independently. The best available tag for the *text* is `[B]` and it can never
be `[A]`. This finding is about the second layer: the metadata attached to the
text, which is separately unusable.

**Three hardware vocabularies on one page.** On the 2026.14.6.7 release page
at tesla-info, fetched 2026-09-27, all of the following appear on one page:

| Field label | Values seen |
|---|---|
| `Seen on AP Hardware` | `HW1 HW2,5 HW3 HW4 PreHW1` |
| `Seen on AP versions` | `AP1 AP2,5 AP3 AP4 PreAP1` |
| `Seen on MCU hardware` | `Intel Ryzen`, and `Ryzen` with no vendor prefix |

The first two are one axis in two spellings, and they are used interchangeably
*within the same table*: `M3 MY` is listed once under `AP Hardware HW3` and
again under `AP versions AP3 AP4`. The page is self-consistent only for a
reader who already knows the mapping, which is the thing the page is supposed
to supply. The third is worse. `Ryzen` is a substring of `Intel Ryzen`, so
the bare form is ambiguous between "any Ryzen" and "a Ryzen whose vendor the
site failed to record" — the field cannot distinguish an Intel head unit from
an AMD one. Any claim of the form "this feature is HW4 only, therefore Intel
Ryzen, therefore not on the older AMD units" reads a vendor out of a field that
does not carry one. `HW2,5`, `PreHW1` and `PreAP1` compound it: pre-generations,
a comma-joined mid-generation, four numbered generations, in two parallel
spellings, with no legend on the page.

**Fleet-share figures are self-inconsistent.** tesla-info.com/release/2026.20
carries `(no cars)`, `(0.54 % of cars)`, `(2.28 % of cars)`, `(3.81 % of cars)`
and `(0.65 % of cars)` on one page. A page reporting "(no cars)" for the build
it is named after is reporting a fleet count, and a fleet count of zero for a
build the same page documents across a wide model list is not credible as a
measurement. Either the counter measures something narrower than "cars" — an
anonymised subset, an opt-in cohort, a partial crawl — or it is unreliable, and
the page does not say which. Without that, the figure is not a percentage of
the fleet. `teslascope` compounds it: one 2026.14 build's page reports "Number
of Cars 0 / Percent of Fleet 0% / Total Pending 2" alongside an unrelated
"62.6 % of drives using FSD".

**Model lists are partial and build-internal.** The codes are terse and
internal — `MX`, `MX(2022+)`, `MS(2021+)`, `MS`, `MY`, `MY(J)`, `M3`, `M3(H)`,
`CT` — and `MS` and `MS(2021+)` both appear on the same page, as do `MY` and
`MY(J)` and `M3` and `M3(H)`, so whether a car is pre- or post-2021 is not
answerable from the list. The list is also silently partial: several notes on
the same 2026.14.6.7 page carry a model list and several carry none, and a
missing list is not a statement that the feature is unavailable elsewhere.
Region lists are unreliable as text, being truncated mid-token.

**Not a Tesla App** was reached only as a search-result excerpt and was not
fetched, so under `.ai/protocols/evidence.md` it cannot be cited and is
excluded from `support`. The excerpt is recorded because it is the origin of
the taxonomy conflict, and it is instructive: it gates a feature on **model**
("New S 3 New X Y CT") and in at least one case on a **prerequisite** ("... /
Ambient Lights"), while teslascope gates the same feature on **AI generation**
("Model S (Refresh) (AI4), Cybertruck (AI4), Model Y (AI4), Model 3 (AI4),
Model X (Refresh) (AI4)") and drops the prerequisite. Two aggregators, same
feature, one gating on what the car is and the other on what chip it has. The
correct gating is a fact about cars, and neither site states it in a form that
can be checked — so it is recorded here, not resolved.

**What survives.** For the *text* of a feature: `[B]`, usable as "an aggregator
transcribes a 2026.14 in-car note reading …", with the build number and fetch
date. Also `[B]`: that the feature is new in build X. For model, hardware
generation, named market, and any percentage of the fleet: unusable, at any
confidence.

**Implication:** this finding constrains the other Tesla findings in this
directory, and states it rather than leaving it implicit. The six findings
built on *manuals* — `tesla-owner-interface-fault-code-exposure.md`,
`tesla-software-rollback.md`, `tesla-ui-performance-figures.md`,
`tesla-safety-toggle-drive-cycle-scope.md`, `tesla-pin-gated-parental-safety-policy.md`,
`tesla-dab-plus-radio-availability.md` — cite primary, complete documentation
and none of them relies on a release-note aggregator; they are unaffected. The
2025-2026 release-note layer of this survey — FSD v14 driver-monitoring
changes, Automatic Collision Evasion, the Cybertruck occupant-classification
change, blind-spot accent lighting, Automatic Software Updates — is `[B]` for
feature text and **unusable** for the hardware and market gating attached to
it. Every statement of the form "HW4 only" or "not on the older cars" in that
material is withdrawn, including where such a statement was load-bearing for a
differentiator argument.

**Reverses if:** tesla-info publishes a legend for its hardware fields or
normalises them to one vocabulary; the counter field is documented and "no
cars" is shown to measure a named subset; or Tesla begins publishing release
notes or a compatibility matrix directly, which removes the whole layer. The
first two are page changes that could happen at any time, which is why `decay`
is 1 month against the 6 months the manual-derived findings carry.

## Searched

tesla-info.com release pages for 2026.14.6.7, 2026.20 and 2026.27.6, all three
fetched 2026-09-27 and read as text, for `Seen on`, `Seen in`, `AP Hardware`,
`AP versions`, `MCU hardware`, `% of cars`, `no cars`, `Last updated`,
`Models:`. Not a Tesla App and teslascope were reached only as search-result
excerpts and are not cited as sources.

## Revisit log

- 2026-09-27 — initial finding, [A] L4 direct observation, three pages of one
  aggregator. Withdraws the hardware and market gating from the 2025-2026
  release-note layer of the survey.
