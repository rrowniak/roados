---
question: How does RAUC guarantee that a failed update cannot leave the unit unbootable?
answer: By absence of a positive mark rather than presence of an error. RAUC writes the new slot, marks it pending and reboots; the running system must then report itself good, and if that call never arrives — panic, kernel oops, power cut — the boot counter expires and the bootloader reverts to the other slot. RAUC is LGPL-2.1 and exposes this over D-Bus as a client to drive, not a library to embed.
tag:      [A]
support:  1
evidence: "rauc.readthedocs.io/en/latest/, read 2026-09-27 — an update may be interrupted at any point without breaking the running system; mark boots as successful or failed; symmetric setup Root-FS A and B; bootloader support for grub, barebox with bootchooser, u-boot and EFI; storage backends include ext4, eMMC boot partitions with atomic update, squashfs, UBI/UBIFS; streaming over HTTP with no intermediate storage on target; RAUC is NOT a full-blown updating application or GUI and is controlled over D-Bus. Licence: rauc/rauc COPYING, GNU LESSER GENERAL PUBLIC LICENSE Version 2.1, 19 February 1999, 501 lines, read 2026-09-27. The Rust rauc crate 0.1.0 has 109 downloads as of 2026-09-27 and is not a usable integration path."
read:     2026-09-27
decay:    1 year, or on any release
unblocks: The update architecture behind doc/IDEA.md:92-99, and whether the target board's bootloader can supply the boot counter the guarantee depends on
---

# How does RAUC guarantee that a failed update cannot leave the unit unbootable?

The guarantee is an **absence of a positive mark**, not the presence of an
error. The sequence is:

1. Two rootfs slots, A and B, on eMMC or a UBI volume.
2. RAUC writes the new slot, marks it pending, reboots.
3. The running system reports itself good through RAUC's D-Bus interface.
4. If that call never arrives — panic, kernel oops, power cut mid-write — the
   **boot counter expires and the bootloader reverts to the other slot**.

Step 4 is what makes it robust. A design triggered by an error code has to
enumerate the failure modes; a design triggered by the absence of a
confirmation has to enumerate only the ways a success can be signalled, and
those are all under the system's control. RAUC's own documentation states the
property as "an update may be interrupted at any point without breaking the
running system".

**The weakest link is not RAUC — it is the bootloader.** Steps 1, 2 and 4 are
the bootloader keeping a boot count and choosing the other slot when it expires:
U-Boot's `bootcount` and `altbootcmd`, or barebox's `bootchooser`. RAUC
documents support for grub, barebox, u-boot and EFI. **If the target board's
bootloader cannot count boot attempts, the A/B guarantee is void** and the
system degrades to "usually recoverable", which is a weaker property than
`doc/IDEA.md:98-99` requires. This was not verified for any specific SBC and is
the thing to check before the update design is committed to.

**Integration is D-Bus, not the Rust crate.** RAUC is explicitly "NOT a
full-blown updating application or GUI" — it is a client that a service drives
over D-Bus, or by invoking the CLI. The `rauc` Rust crate at version 0.1.0 has
**109 downloads** and is not a real integration path; do not plan around it.

**Implication:** the update architecture is available off the shelf and is
GPL-compatible, being LGPL-2.1, which the project's GPL-3.0-or-later licence
absorbs. Two constraints follow that are not obvious from the feature list: the
bootloader must implement a boot counter, and the writable overlay must not
carry state that needs to survive a rollback, or the two mechanisms will
disagree. The LGPL obligation is dynamic linking or relinkable object code, not
a problem in a GPL work.

This is the mechanism `doc/IDEA.md:92-99` is written against, and it stands in
direct contrast to `doc/findings/tesla-software-rollback.md`, where the
manufacturer's own update path left the vehicle unable to start and required a
dealer to recover. The mechanism is available; the question this project has to
answer is only whether the bootloader cooperates.

**Reverses if:** the target board's bootloader turns out to lack a boot counter,
which is checkable in an hour by reading its U-Boot or barebox configuration
and deliberately interrupting an update. If it does lack one, the guarantee
becomes "the running system is never broken", because RAUC never writes the slot
it is running from — a weaker but still useful property, and one that may be
sufficient for `doc/IDEA.md` without a bootloader change.

## Revisit log

- 2026-09-27 — initial finding, `[A]` from RAUC documentation and `COPYING`, both read
