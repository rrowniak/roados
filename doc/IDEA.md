# The idea

This project is written mostly in Rust and aims to provide an infotainment
implementation for various types of vehicles, including:

- cars,
- old cars,
- trucks,
- tractors and other machines,
- motorcycles, e-bikes.

Electric vehicles are a special case, and they are included here as well.

Supporting all of these is not a goal in itself. The goal is a system that is
configurable and easy to extend, ideally by the community: the seam between the
core and a given vehicle is meant to be a contract that others can build
against. The first vehicle is a classic car — a Volkswagen Passat B5.5, model
year 2003. It is the first user of the system, not the shape the system is
built around.

## What it is

### Features

**Audio and media**

- audio output to the car, integrated correctly — zones, source priority, ducking,
- FM radio; DAB+ where licensing permits,
- local and USB media playback,
- calls and audio from a connected phone, with no application of ours involved.

**Vehicle data**

- speed, engine speed, coolant temperature,
- battery voltage, fuel level, outside temperature,
- current and average consumption, estimated range,
- any further signal the vehicle's own bus offers, shown as it becomes available.

**Diagnostics**

The vehicle's diagnostic protocols are open and documented, which is what makes
this possible at all — and is being closed off in newer cars.

- fault codes, read across the whole vehicle,
- fault reset, per module, with the codes exported first. Clearing is
  per-module and whole-memory, with no partial erase, so the export is not
  optional,
- measurement blocks, read on demand,
- recording of measurements over time.

**Comfort**

- heated seats and heated steering wheel, driven by the interface, with the
  vehicle keeping its own physical switch,
- cabin temperature and other climate properties, including setting the target
  temperature. The unit has no documented digital interface, so this is either
  done by driving the unit's own controls or by reverse-engineering the module,
  and both routes are in scope. The resulting actuator position is readable,
  so driving the controls can be closed-loop,
- automatic headlights, from the light sensor or from the camera.

**Information and assistance — warning only, never control**

- rear view on the main display, triggered by the car's own reverse signal,
- a second display in the driver's line of sight, where the vehicle leaves room,
- a trip computer — consumption, range, temperature history,
- rain, glare and tunnel detection,
- traffic sign recognition, reading speed limits. The closest open work is
  trained on US signs and sustains about 1.8 recognitions per second on an
  already-loaded CPU, so metric recognition is ours to build and license,
- car-following warning,
- blind spot warning. The vehicle has no blind-spot sensor and does not report
  one, so this needs side sensing added. Production systems read the signal
  off the bus instead of computing it, and we have no bus to read.

**Navigation**

- offline turn-by-turn navigation,
- POI search, including fuel stations,
- online routing, traffic and road events, when a connection is available.

### Rules that constrain features

- reading the vehicle is unconstrained: any bus may be tapped for anything it
  offers,
- where the vehicle cannot provide a capability, adding the hardware to provide
  it is in scope. This list states goals, not a survey of what the car already
  has,
- no feature may degrade the car's baseline ability to start, drive and stop,
- the interface is never the only way to operate something a driver needs,
- what a vehicle cannot do is shown as absent, never estimated.

### Non-functional goals

- UI responsiveness: Tesla-level or better. Tesla publishes no frame rate, no
  input latency and no boot time, so this is a bar rather than a figure we can
  be measured against — we set our own numbers and measure them,
- cold boot to usable, and resume from suspend, fast enough to feel like
  switching something on rather than starting something,
- every feature works with no network and no account; anything that needs a
  connection is additive, and the interface never blocks on it,
- an update can always be completed or rolled back; a failed one cannot leave
  the unit unbootable,
- no telemetry — nothing about the driver, the car's location or its data
  leaves the vehicle unless the driver asks for it,
- the owner can update it, and can recover it, without us,
- never store camera footage. The camera is a sensor; recognition is done in
  memory and discarded. Event recording is a separate question with its own
  legal constraints and is not part of this scope.

### Not planned

- actuating braking or steering, by any part of this system,
- throttle-influencing cruise assist — not excluded, and not planned either;
  nothing in the system is built in anticipation of it,
- music streaming services — these require a commercial partner agreement,
- HD Radio — per-unit receiver royalties are unresolved,
- a third-party application store,
- storing or publishing driver footage,
- implementing backends for the other vehicle classes listed above. That is
  what the extension seam is for.

## The architecture

The system runs on Linux, and the leading language is Rust. C and C++ are
used where they are the right tool and no better option exists, which
currently means firmware on the vehicle interface gateway.

The target architectures are aarch64 and x86_64. The user interface is
written from scratch over OpenGL ES 3.1, including the widget layer, layout,
motion and theming. The GPU family is the filter that decides which hardware
is usable: Vivante is out because its open driver stops at OpenGL ES 2.0, and
NVIDIA Tegra is out because reaching a useful OpenGL ES level requires a
proprietary driver. The survivors — Mali, Broadcom VideoCore, Intel, AMD,
Adreno and PowerVR — all reach OpenGL ES 3.1 or better with open drivers and
no NDA.

The target device is not yet chosen, so the application layer is kept free
of target-specific dependencies and the target-specific parts sit behind
documented interfaces. The platform is a dependency, not a product: what is
published is a set of services and a user interface against documented
interfaces, not a distribution.

### The units

The minimum deployment is one Linux device running the core and the user
interface, with the vehicle interface on the gateway. A more distributed
deployment gives the user interface a device of its own and the head up
display one of its own, with the units connected over Ethernet.

The vehicle interface gateway is a microcontroller, separate from any Linux
compute. It speaks the vehicle's own protocols — CAN, LIN, K-line — and drives
the small electrical loads the interface needs, such as seat heating. It is
the only part of the system wired to the car.

Keeping the gateway off the Linux board buys four things. The vehicle's
electrical environment stays off the compute supply. CAN scheduling is bounded
without real-time Linux. The compute unit is swappable without touching the
vehicle interface. And the vehicle's protocols terminate in one place, which
also puts its serial diagnostic protocol on a microcontroller where existing
C implementations are usable rather than in Rust, where there are none.

The link from the core to the gateway is a direct point-to-point connection,
USB preferred over Ethernet. USB resets the device on disconnect, needs no
switch and no IP stack, and already requires physical access.

### Failure

The core is supervised rather than made redundant. Two compute units would
double the power drawn and the heat to be removed, in a sealed enclosure and
on a battery that has to survive being parked, to cover a failure whose
consequence is that the head unit is dead while the car keeps driving. Faster
recovery is the cheaper answer.

The user interface never blocks on the core. It renders from its own last
known state and shows the data as stale when it stops being updated, so core
failure costs freshness rather than function. Staleness is detectable quickly:
the vehicle's own messages repeat every 10 to 25 ms, so a gap of more than a
few message periods means a dead core.

The display is owned by as small a process as possible. Whoever holds the DRM
master owns the display, so if that process dies the screen goes black until
another takes it. Media decoding, camera and recognition run behind it, in
processes that can crash and be restarted without taking the display with them.

Watchdogs are layered to match the failure they cover. A process fault is
restarted by the supervisor, which is fast. A kernel fault needs a hardware
watchdog and a board reset, which is slow. The gateway carries its own
watchdog, independent of Linux.

### Actuation

The gateway is an actuator and not only a reader. This is a deliberate and
bounded exception: it drives comfort loads — heated seats, heated steering
wheel — and nothing that affects the vehicle's baseline ability to start, drive
or stop. The vehicle's own physical switch stays authoritative where the
vehicle has one.

Actuation introduces a hazard that reading does not. A load left energised
when the command source is gone is the failure to design against, so every
actuated output fails safe on loss of command rather than holding its last
state.

The gateway is a second update domain alongside the Linux unit, so the
protocol between core and gateway is versioned, and a mismatch is detected
rather than assumed away.

## The golden standard and reference point

Tesla's infotainment is the UX ambition. It is not a technical reference
point, and the survey behind that judgement is specific about why:

- it publishes no frame rate, no input latency and no boot time,
- software cannot be rolled back — the manual states that reverting to a
  previous version is not possible,
- the owner interface exposes no fault codes, no measurement values and no
  per-module reset. It reports symptoms and maintenance intervals, and stops
  there.

The last two are not ambitions we chose over the standard. They are available
here and not there, because this vehicle's diagnostic and update protocols are
open and documented and the newer ones are not.
