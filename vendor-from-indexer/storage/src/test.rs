//! Testing helpers

use std::time::Instant;

/// Helper for printing time elapsed between chunks of code.
pub(crate) struct Timer {
    prefix: String,
    start: Instant,
    last: Instant,
}
impl Timer {
    /// Create a new Timer, initializing both start and last to the current
    /// instant
    ///
    /// The prefix is added to `Self::delta` msgs.
    pub(crate) fn new<S: AsRef<str>>(prefix: S) -> Self {
        let now = Instant::now();
        Timer {
            prefix: prefix.as_ref().to_string(),
            start: now,
            last: now,
        }
    }

    /// Print "&lt;self.prefix&gt;: &lt;time since last delta&gt;/&lt;time since start&gt;: &lt;msg&gt;".
    ///
    /// Returns the time since the last call to `delta`, or since
    /// construction if this is the first call to `delta`.
    pub(crate) fn delta<S: AsRef<str>>(&mut self, msg: S) -> f32 {
        let now = Instant::now();
        let duration_since_start = now.duration_since(self.start).as_secs_f32();
        let duration_since_last = now.duration_since(self.last).as_secs_f32();
        self.last = now;
        println!(
            "{}: {:.2?}/{:.2?}: {}",
            self.prefix,
            duration_since_last,
            duration_since_start,
            msg.as_ref(),
        );
        duration_since_last
    }
}
