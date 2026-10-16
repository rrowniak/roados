# Target platform — decisions

Bench first, car second. The bench proves the stack; the car gets an
industrial panel and carrier, behind the same interfaces.

## Bench (decided 2026-10-16)

- Compute: CM4 wireless, eMMC (4GB/16GB min), on CM4IO carrier.
  CM4 is -20..+85C; the IO board is bench-commercial.
- Display: Waveshare 15.6" (H) 16645, 1920x1080 HDMI + USB-HID touch.
  Driverless, universal; tape bond accepted on bench only.
- Radio: onboard WiFi/BT + external Antenna Kit (`dtparam=ant2`).
- GPS: USB puck (BU-353 / VK172 class), mainline `pl2303/cdc_acm` + gpsd.
- Audio: HDMI audio on bench; USB DAC is the car path, bought now so
  the audio route is already USB.
- Power/cool: 12V/2A barrel, CM4 cooler + PWM fan. Car power
  (buck + ignition-sense shutdown) is a separate, later decision.
- OS/provision: Pi OS Lite 64-bit, pinned release (bench-only, revisable).
  SSH provision + app deploy, no baked image; recovery is reflash + rerun.
  App runs as root under one systemd unit; boot target under 7 s to first
  touch-responsive frame.

## Car (constraints, not parts)

- Panel: industrial open-frame, HDMI/LVDS/eDP + USB touch. Chosen at
  dash-build time; bench HDMI+HID keeps every option open.
- Carrier: custom (wide-Vin, hold-up, ignition sense) — CM4IO does not
  go in the car.
- Display bond must be optical or in-cell; edge-tape panels do not go
  in the car (peel under heat + vibration).

## Swap rule

UI renders logical (1280x720 dp baseline), scaled to physical; geometry,
fonts and touch targets in mm/dp, never pixels. Panel swap is EDID +
touch matrix in the DRM-master process only. Proven by running the bench
panel at 1920x1080, 1280x720 and 1024x600 with no code change.

## Open

Car panel model, carrier design, 12V power design.
