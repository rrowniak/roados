---
question: How is the reception time of a received CAN frame obtained on Linux SocketCAN, and does SO_TIMESTAMPING work on a CAN_RAW socket?
answer: The documented mechanism is the SIOCGSTAMP ioctl, giving 1 us resolution and set automatically when the frame is received, which socketcan exposes as read_frame_with_timestamp(). Whether SO_TIMESTAMPING produces ancillary cmsgs on a CAN_RAW socket is unverified and contradicted by the source — raw_rcv() contains no timestamping call, and socketcan's own documentation asserts the opposite. Treat SIOCGSTAMP and SIOCGSTAMPNS as the production path and do not build on SO_TIMESTAMPING.
tag:      [C]
support:  2
evidence: "kernel.org/doc/html/latest/networking/can.html, How to use SocketCAN, SIOCGSTAMP with 1 us resolution set automatically at reception [A]; net/can/raw.c:1078, .gettstamp = sock_gettstamp in raw_ops [A]; net/can/raw.c:129-213, raw_rcv() filters, clones and calls sock_queue_rcv_skb_reason() with no timestamping call [A]; drivers/net/can/dev/dev.c:460-464, SOF_TIMESTAMPING_RX_HARDWARE | SOF_TIMESTAMPING_RAW_HARDWARE [A]; socketcan src/socket.rs:237-262 and src/timestamp.rs:146 expose the ioctl paths [A]. Fetches that failed and are part of the result: raw.githubusercontent.com/torvalds/linux/master/net/core/sock.c and include/linux/net_sock.h both returned HTTP 404, so the generic receive path emitting the cmsg could not be located on current mainline."
read:     2026-09-27
decay:    1 month
unblocks: Which timestamp source the core process ages vehicle data with, and whether the integration target is a 1 us ioctl or an ancillary cmsg
---

# How is the reception time of a received CAN frame obtained on Linux SocketCAN, and does SO_TIMESTAMPING work on a CAN_RAW socket?

Two mechanisms are documented and one is contradicted. The kernel's SocketCAN
documentation states that after a successful read the reception time can be
obtained with `ioctl(s, SIOCGSTAMP, &tv)`, at **1 µs resolution**, and that the
timestamp is set automatically when the frame is received. The CAN raw socket
wires this up — `.gettstamp = sock_gettstamp` in `raw_ops` — and CAN drivers
that support it advertise `SOF_TIMESTAMPING_RX_HARDWARE` and
`SOF_TIMESTAMPING_RAW_HARDWARE`, which is how `read_frame_with_hw_timestamp()`
is meant to be fed. `socketcan` 4.0.0 exposes all three paths:
`read_frame_with_timestamp()` (microseconds),
`read_frame_with_hw_timestamp()`, and `read_frame_with_timestamps()` returning
`CanTimestamps`.

The claim that does not survive inspection is the ancillary-message route.
`socketcan`'s documentation asserts that `SO_TIMESTAMPING` works on CAN_RAW and
that `SOF_TIMESTAMPING_OPT_CMSG` is required on non-IP sockets, "which includes
CAN raw". But `raw_rcv()` — the function every received frame passes through —
contains **no timestamping call of any kind**. It filters by error class,
clones the skb, and calls `sock_queue_rcv_skb_reason()`. The generic socket
receive path that would emit `SCM_TIMESTAMPING` is not in `raw_rcv`, and I
could not find `sock_recv_ts_and_drops` in the current mainline
`net/core/sock.c`; the receive path has been refactored. Two fetches failed and
that failure is part of the result rather than a footnote: both
`net/core/sock.c` and `include/linux/net_sock.h` returned HTTP 404 from the
raw mirror I used, so the cmsg path is unlocated rather than disproved.

That contradiction is why this finding is tagged `[C]` and not `[A]`. The
ioctl half is solid at `[A]`; the cmsg half is an unresolved conflict between a
library's documentation and the source, and the schema requires the tag to sit
at the lowest level any source in the answer supports.

**Implication:** the core process should age vehicle data from the
`SIOCGSTAMP`/`SIOCGSTAMPNS` ioctl, which the kernel documents and `socketcan`
exposes, and must not be built around `SO_TIMESTAMPING` cmsgs until the
contradiction is settled. Hardware timestamping is controller-dependent and is
not something to assume on a USB-CAN adapter.

**Reverses if:** a `vcan0` test settles it in about an hour. Load `vcan0`, open
a `CAN_RAW` socket, set
`SO_TIMESTAMPING | SOF_TIMESTAMPING_OPT_CMSG | SOF_TIMESTAMPING_RX_SOFTWARE`,
`recvmsg()` with a control-message buffer, and print the cmsg. If
`SCM_TIMESTAMPNS` arrives, the `socketcan` claim holds and this finding is
superseded. Alternatively, reading the current `net/core/sock.c` receive path
locates or refutes the emitting code without any hardware.

## Revisit log

- 2026-09-27 — initial finding, `[A]` for the ioctl path from kernel docs and `net/can/raw.c:1078`; `[C]` retained for the cmsg question after a failed fetch of `net/core/sock.c`
