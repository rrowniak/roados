---
question: Does Tesla infotainment support DAB+ digital radio?
answer:  Not according to any Tesla document read. Across 171 manual pages in the US and UK, "DAB", "DAB+" as standalone terms and "digital radio" never appear. The only digital-radio standard Tesla names anywhere is HD Radio — a conditional feature in the US Model 3 manual, and in the UK S/X manual only a trademark notice crediting iBiquity Digital Corporation. The UK §Feature Availability Statement, whose purpose is to list features that vary by market, mentions no radio band at all. A claim that DAB+ was standard on early UK Model S cars is in circulation but could not be verified against any primary source and is not resolved here.
tag:      [A]
support:  2
evidence: Zero standalone occurrences of "DAB", "DAB+" or "digital radio" in 171 pages; the only DAB substring hits are inside "unavoidable" in §Collision Avoidance Assist, verified in context. Model S/X UK 2026.14 §About this Owner Information — "HD Radio is a registered trademark of iBiquity Digital Corporation." Model 3 NA 2024.44.25.3 §Media — "If available, touch HD® to play high definition versions of the selected frequency." Model S/X UK 2026.14 §Media — "Selections you play on FM (if equipped) radio are not included in the Recents list." Zero radio-band mentions in §Feature Availability Statement in either manual.
read:     2026-09-27
decay:    3 months
unblocks: doc/IDEA.md:28 ("FM radio; DAB+ where licensing permits") and 107 (HD Radio royalties)
---

# Does Tesla infotainment support DAB+ digital radio?

Not according to any Tesla document read. Searched all 171 pages of the
three-manual corpus for `DAB`, `DAB+`, `digital radio`, `Digital Audio
Broadcasting`, `DAB radio`, `SiriusXM`, `Sirius`. No standalone occurrence of
any. The only `DAB` hits are the letters inside "un**avoi**dab**le**" in
§Collision Avoidance Assist — a substring artefact, verified by printing each
in context.

Two documents that *should* mention it do not. **§Feature Availability
Statement** is by its own framing the list of features that differ by market and
model, and it has **zero** mentions of any radio band, in either the Model 3 NA
or the S/X UK manual. The UK — where DAB+ is the established standard and FM
simulcast is being phased out — is exactly where that statement would be
expected to say something. **§Media in the S/X UK manual** describes the radio
as "Choose from a list of available radio stations or touch the numeric keypad
to directly tune the radio to a specific frequency", naming no band at all in
that description, and names FM exactly once elsewhere, in an aside about the
Recents list, and conditionally: "Selections you play on FM (**if equipped**)
radio are not included in the Recents list."

**What Tesla names instead: HD Radio, the North American system.** As a
feature, in Model 3 NA §Media: "If available, touch HD® to play high
definition versions of the selected frequency" — conditional, US, with real
UI. As a trademark only, in Model S/X UK §About this Owner Information: "HD
Radio is a registered trademark of iBiquity Digital Corporation." The UK
manual's Media section does not offer it, so the only place a UK driver could
learn Tesla deals in HD Radio is the boilerplate at the back of the manual.
The two are not inconsistent — a US-only feature and a global trademark notice
coexist — but the asymmetry is the finding: the digital-radio system Tesla's
documentation can speak about is the one carrying per-unit royalty obligations,
and the one relevant to a European car is absent from the documentation.

**The unverified counter-claim.** A practitioner report circulates that DAB+
was standard on early UK Model S cars from 2014, that an MCU1-to-MCU2 retrofit
removed AM, FM and DAB+ as a cost-reduction measure, and that a paid
restoration was offered for cars built before a March 2018 cutover. It is
**not verified**: the article was not fetched, no primary source was located,
and no Tesla document in the corpus corroborates or contradicts it. `[C]`,
snippet-level, recorded rather than resolved. It should not be silently
dropped, and it should not be treated as evidence. Note the shape of the
conflict: the counter-claim is about *historical* hardware, the corpus about
*current* software. A document silent on whether the car it describes has a
radio band is not thereby evidence the car never had one. Both can be true —
a 2014 car with DAB+, a 2026 car without, and the manual silent in both cases.

**Implication:** `doc/IDEA.md:28` lists "FM radio; DAB+ where licensing
permits" as a feature and 107 records "HD Radio — per-unit receiver royalties
are unresolved". On 28, the evidence supports a statement rather than a gap:
Tesla's UK documentation for 2026.14 neither claims DAB+ nor describes a
digital band, so a DAB+ implementation cannot be justified as closing a visible
Tesla gap, because there is no visible gap in the documentation to close. On
107, the royalty line is *corroborated by Tesla's own manual* — the trademark
notice names iBiquity, the licensor whose per-unit royalties are the unresolved
item. That is not new evidence, but it means the line rests on a primary
document rather than on assumption. The licensing design survives either way:
"where licensing permits" is a licence-encumbrance design, not a market-gating
one, and the DAB+ question does not change it — it only removes a marketing
reason for it.

**Reverses if:** a Tesla UK or European manual documents a DAB feature or
names DAB in §Media or §Feature Availability Statement; a Tesla service page
documents DAB+ hardware for a current model; or a *fetched* primary source
corroborates the early-Model-S claim, which would change this from "no current
support documented" to "support withdrawn" — a materially different claim. A
Tesla European media page that is not in the owner's manual would also settle
it. Re-running the term list over one manual is about an hour.

## Searched

171 manual pages as text for the term list above, plus a direct read of §Media
and §Feature Availability Statement in both the Model 3 NA and S/X UK manuals
and of §About this Owner Information in both. The counter-claim was located in
a search result and deliberately not cited: the article was not fetched, so no
page, section or quote is available, and under `.ai/protocols/evidence.md` a
URL inferred from a search snippet is not a citation.

## Revisit log

- 2026-09-27 — initial finding, [A] L6 absence over 171 pages, plus [C]
  unverified counter-claim recorded and left open
