//! How fast the demo is running, measured from the loop's own frame deltas.
//!
//! **A still proves what is drawn and never how fast it is drawn.** Task 11's
//! demo rendered at about 4 fps — a `load_char` per character, per label, per
//! frame — and survived three reviews and four captures, because every capture
//! method in this repository is a single still, and a still of a 4 fps
//! application is pixel-identical to a still of a 60 fps one. The entry in
//! `.ai/NEVERAGAIN.md` is the reason this module exists, so the next regression
//! of that kind is a **number** rather than a suspicion.
//!
//! Two numbers come out of a run and they answer different questions:
//!
//! - the **average**, `frames` over the time the run took. It is what a test
//!   runner compares against a baseline and what an operator comparing two
//!   builds wants.
//! - the **worst single frame**, which is what catches a hitch the average hides.
//!   An application that runs at 62 fps for nine seconds and takes 300 ms once
//!   has not had a good run, and its average will not say so.
//!
//! The meter holds **no clock**. Every frame's cost arrives as a `Duration` from
//! the caller — the loop already computes one per frame for the animation clocks —
//! and that is what makes this module testable without a wall clock: the tests
//! drive it with durations they chose, and nothing in them can fail because the
//! machine was busy. `AGENTS.md` forbids time-dependent tests, and a meter that
//! read `Instant::now()` itself could only be tested by timing.

use std::time::Duration;

/// How long a window of frames is, before the rate on screen is reduced.
///
/// Half a second: short enough that a stall is still on screen when it happens,
/// and long enough that one slow frame does not own the number. At 60 fps a
/// 500 ms window is thirty frames, and one 34 ms frame inside one moves it from
/// 62.5 to 60.3 rather than to 29.
pub const RATE_WINDOW: Duration = Duration::from_millis(500);

/// A frame that took longer than this missed a presentation slot.
///
/// Twice the loop's own 16 ms wait, rounded up: a frame longer than this did not
/// fit in two of the loop's slots, so it is a dropped frame rather than a merely
/// slow one. It is the count to read when an average looks fine and something
/// still feels wrong.
pub const LONG_FRAME: Duration = Duration::from_millis(33);

/// The first word of the line this module prints, so a runner can find it in a
/// log full of the demo's own output.
///
/// Stable because `.ai/tools/fps-check.sh` greps for exactly this word: a runner
/// written against one spelling is a runner that silently stops finding the
/// report when the prefix is prettied.
pub const REPORT_PREFIX: &str = "roados-fps";

/// Counts frames and reduces them to rates.
///
/// Ticked once per frame by the demo's `frame`, with the same delta the
/// animation clocks get, so the number describes the frames that were drawn.
#[derive(Debug, Clone, Default)]
pub struct FrameRate {
    /// Every frame this run has counted, with every frame's cost added up.
    frames: u64,
    elapsed: Duration,
    /// The frames and the time since the last window boundary, partial window
    /// included: this is what [`FrameRate::rate`] reads.
    window_frames: u64,
    window_elapsed: Duration,
    /// The longest single frame so far, and how many have run past
    /// [`LONG_FRAME`].
    worst: Duration,
    long_frames: u64,
}

impl FrameRate {
    /// Returns a meter that has counted nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Counts one frame that took `delta`, and returns the rate on screen after
    /// it.
    ///
    /// **The window is closed before the frame is counted, not after.** A window
    /// that were closed afterwards would leave the meter holding no frames at all
    /// for the space of one call, and the number on screen would read 0 fps once
    /// every half second — a flicker that says the application stopped, on a
    /// frame that did not.
    pub fn tick(&mut self, delta: Duration) -> f64 {
        if self.window_elapsed >= RATE_WINDOW {
            self.window_frames = 0;
            self.window_elapsed = Duration::ZERO;
        }
        self.frames += 1;
        self.elapsed += delta;
        self.window_frames += 1;
        self.window_elapsed += delta;
        if delta > self.worst {
            self.worst = delta;
        }
        if delta > LONG_FRAME {
            self.long_frames += 1;
        }
        self.rate()
    }

    /// Returns the rate of the frames since the last window boundary, in frames
    /// per second.
    ///
    /// The **partial window is included**, so this is never a stale number: a
    /// meter that reported the last completed window would keep showing 60 fps
    /// for the half second after a 300 ms stall, which is a rate nothing is
    /// running at. It is also the same number as the run's average before the
    /// first boundary, because until then there is no boundary — so the readout
    /// has something truthful to say from the first frame rather than a zero, and
    /// a zero would be a claim rather than a measurement.
    #[must_use]
    pub fn rate(&self) -> f64 {
        rate(self.window_frames, self.window_elapsed)
    }

    /// Returns the run's average rate in frames per second, or **0.0** if no time
    /// has passed.
    ///
    /// Zero rather than a division by zero and rather than infinity: a run of no
    /// frames has no rate, and a reader of the report needs a number it can parse
    /// rather than one it has to guard.
    #[must_use]
    pub fn average(&self) -> f64 {
        rate(self.frames, self.elapsed)
    }

    /// Returns how many frames the run has counted.
    #[must_use]
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// Returns how long the run has been going, as the frames' own deltas added
    /// up.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Returns the longest single frame so far, or zero if none has been counted.
    #[must_use]
    pub fn worst_frame(&self) -> Duration {
        self.worst
    }

    /// Returns how many frames have taken longer than [`LONG_FRAME`].
    #[must_use]
    pub fn long_frames(&self) -> u64 {
        self.long_frames
    }

    /// Returns the line the demo shows on screen.
    ///
    /// Three numbers, and the worst one among them: the current rate is what a
    /// person watching wants, the average is what they will be asked about, and
    /// the worst frame is the one a rate alone would hide. `worst 34 ms` rather
    /// than a column of fields is the shape the rest of the demo's readouts use —
    /// `50%, determinate`, `off, 3 changes` — and this line is a readout.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "fps {:.0}, avg {:.1}, worst {:.0} ms",
            self.rate(),
            self.average(),
            millis(self.worst_frame())
        )
    }

    /// Returns the line the demo prints when it stops, for a test runner to read.
    ///
    /// **`key=value` pairs after a fixed prefix**, because this line is parsed by
    /// a script rather than read by a person: `grep '^roados-fps '` finds it in a
    /// log full of the demo's other output, and every number can be pulled out by
    /// name rather than by counting spaces.
    ///
    /// Every field is a quantity of the **whole run**, which is why the current
    /// rate is not among them: it is a window's, and at the moment a bounded run
    /// ends that window is partial, so it is a number whose value depends on when
    /// the run was stopped. One decimal on the rates is the honest precision — a
    /// rate is a ratio of two counts, and the digits after that are noise a
    /// comparison would only trip over.
    #[must_use]
    pub fn report(&self) -> String {
        format!(
            "{REPORT_PREFIX} frames={} duration_s={:.3} average_fps={:.1} \
             worst_frame_ms={:.1} long_frames={}",
            self.frames(),
            self.elapsed().as_secs_f64(),
            self.average(),
            millis(self.worst_frame()),
            self.long_frames(),
        )
    }
}

/// Returns `count` frames per `over`, or 0.0 when `over` is no time at all.
///
/// The one place a count becomes a rate. `u32` rather than `u64` because an f64
/// has no precision left above 2^53 frames and the conversion saturates rather
/// than wraps: a run of `u32::MAX` frames takes over two years at 60 fps, and a
/// saturated rate is a number a reader will believe is wrong rather than one that
/// silently reads as plausible.
fn rate(count: u64, over: Duration) -> f64 {
    let seconds = over.as_secs_f64();
    if seconds <= 0.0 {
        return 0.0;
    }
    let frames = f64::from(u32::try_from(count).unwrap_or(u32::MAX));
    frames / seconds
}

/// Returns `duration` in milliseconds, for the numbers a report is read in.
fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a meter that has counted `count` frames of `delta` each.
    ///
    /// The run time a test wants is a duration it wrote down rather than one it
    /// waited for, which is the whole reason [`FrameRate`] takes its deltas from
    /// the caller.
    fn run(count: u64, delta: Duration) -> FrameRate {
        let mut meter = FrameRate::new();
        for _ in 0..count {
            meter.tick(delta);
        }
        meter
    }

    /// Thirty 20 ms frames is 600 ms of run time, 50 frames per second, and two
    /// windows: the first closes at the 25th frame (500 ms) and the second holds
    /// the remaining 5. Every rate in these tests is one of those two numbers.
    const TWENTY_MS: Duration = Duration::from_millis(20);
    const FIFTY: f64 = 50.0;

    #[test]
    fn a_run_of_uniform_frames_reports_their_own_rate() {
        let meter = run(30, TWENTY_MS);
        assert_eq!(meter.frames(), 30);
        assert_eq!(meter.elapsed(), Duration::from_millis(600));
        assert_eq!(meter.average(), FIFTY);
        assert_eq!(meter.rate(), FIFTY, "5 frames of the second window");
        assert_eq!(
            meter.report(),
            "roados-fps frames=30 duration_s=0.600 average_fps=50.0 \
             worst_frame_ms=20.0 long_frames=0"
        );
    }

    #[test]
    fn the_window_rate_is_the_frames_since_the_last_boundary_and_not_the_average() {
        // 25 frames of 20 ms close the first window at 500 ms, and the 26th
        // starts the second — so the rate the readout shows is one frame over
        // 20 ms while the average is 26 over 520 ms, and the two are 50.0 and
        // 50.0 only because this run is uniform. The next test is the one that
        // tells the two apart.
        let meter = run(26, TWENTY_MS);
        assert_eq!(meter.rate(), FIFTY);
        assert_eq!(meter.average(), FIFTY);
        assert_eq!(meter.window_frames, 1, "the second window has one frame");
    }

    #[test]
    fn a_window_that_was_slow_reports_the_slow_rate_and_the_average_remembers_it() {
        // Thirty 10 ms frames, then thirty 40 ms ones: 300 ms then 1200 ms, so
        // 60 frames over 1.5 s is 40 fps for the run. The window that closed last
        // holds only 40 ms frames — 12 of them, 480 ms, because the 13th crossed
        // 500 ms and was counted into it — so the readout says 25 while the
        // average still carries the fast half.
        let mut meter = run(30, Duration::from_millis(10));
        assert_eq!(meter.rate(), 100.0, "300 ms has not reached a window");
        for _ in 0..30 {
            meter.tick(Duration::from_millis(40));
        }
        assert_eq!(meter.average(), 40.0, "60 frames over 1.5 s");
        assert_eq!(
            meter.rate(),
            25.0,
            "the window that has just closed is all 40 ms frames"
        );
        assert!(
            meter.rate() < meter.average(),
            "a readout of the current rate is below the run's average, and it is \
             the average that would hide the stall"
        );
    }

    #[test]
    fn closing_a_window_does_not_read_zero_for_a_frame() {
        // The window is closed before the frame is counted, so the frame that
        // follows a boundary is the first one in the new window and the rate is
        // that frame's. An implementation that closed the window afterwards would
        // return 0.0 here, once every 500 ms, which on screen is a flash of
        // "fps 0".
        let mut meter = run(25, TWENTY_MS);
        assert_eq!(meter.window_elapsed, Duration::from_millis(500));
        let rate = meter.tick(TWENTY_MS);
        assert!(
            rate > 0.0,
            "the 26th frame opens the second window and is inside it, so {rate} \
             cannot be zero"
        );
        assert_eq!(rate, FIFTY);
    }

    #[test]
    fn a_slow_frame_is_the_worst_one_and_is_counted() {
        let mut meter = run(10, TWENTY_MS);
        meter.tick(Duration::from_millis(120));
        assert_eq!(
            meter.worst_frame(),
            Duration::from_millis(120),
            "the longest frame is kept, not the last one and not the mean"
        );
        assert_eq!(
            meter.long_frames(),
            1,
            "and it is the only frame past LONG_FRAME, because 20 ms is not"
        );
    }

    #[test]
    fn a_frame_at_the_long_frame_threshold_is_not_counted_as_a_long_one() {
        // The count says "a frame did not fit in two of the loop's slots", so it
        // is strictly longer than twice the wait: at the threshold itself the
        // frame did fit, and counting it would make the number depend on which
        // side of an inequality a millisecond fell.
        assert_eq!(run(4, LONG_FRAME).long_frames(), 0);
        assert_eq!(
            run(4, LONG_FRAME + Duration::from_millis(1)).long_frames(),
            4
        );
    }

    #[test]
    fn a_run_with_no_time_in_it_has_no_rate() {
        let meter = FrameRate::new();
        assert_eq!(
            meter.average(),
            0.0,
            "no frames and no time is not a division"
        );
        assert_eq!(meter.rate(), 0.0);
        assert_eq!(meter.frames(), 0);
        assert_eq!(meter.elapsed(), Duration::ZERO);
        assert_eq!(meter.worst_frame(), Duration::ZERO);
        assert_eq!(meter.long_frames(), 0);
    }

    #[test]
    fn a_run_of_frames_with_no_time_between_them_reports_no_rate() {
        // A rate is frames over time. Counted without the time it would be
        // infinity, and `summary` would print a word nobody can parse.
        let meter = run(10, Duration::ZERO);
        assert_eq!(meter.average(), 0.0);
        assert_eq!(meter.rate(), 0.0);
        assert_eq!(
            meter.summary(),
            "fps 0, avg 0.0, worst 0 ms",
            "and the worst frame of a run that took no time is no time"
        );
    }

    #[test]
    fn the_summary_names_the_rate_the_average_and_the_worst_frame() {
        assert_eq!(
            run(30, TWENTY_MS).summary(),
            "fps 50, avg 50.0, worst 20 ms"
        );
    }

    #[test]
    fn the_summary_of_a_stalling_run_shows_the_stall() {
        // The pair the readout exists for. 40 frames of 20 ms is 800 ms and 50
        // fps, and the 41st frame takes 200 ms: the window that has just closed
        // holds 16 frames over 500 ms, so the rate on screen reads 32, while the
        // run's average — 41 frames over 1000 ms — still reads 41. The average
        // is the number that would hide the stall and the two others are not.
        let mut meter = run(40, TWENTY_MS);
        meter.tick(Duration::from_millis(200));
        assert_eq!(meter.summary(), "fps 32, avg 41.0, worst 200 ms");
        assert_eq!(meter.long_frames(), 1);
    }
}
