use std::time::{Duration, Instant};

pub struct Timer {
    start_time: Instant,
}

impl Timer {
    /// Creates a new timer and starts it.
    pub fn new() -> Self {
        Timer {
            start_time: Instant::now(),
        }
    }

    /// Restarts the timer.
    pub fn restart(&mut self) {
        self.start_time = Instant::now();
    }

    /// Returns the elapsed time since the timer was started (or restarted).
    pub fn elapsed(&self) -> Duration {
        Instant::now().duration_since(self.start_time)
    }
}

#[cfg(test)]
mod test {
    use std::thread;
    use std::time::Duration;

    use crate::timer::Timer;

    #[test]
    fn test_timer() {
        // Make a timer and sleep for a duration
        let mut timer = Timer::new();
        const NUMBER_OF_MILLISECONDS: u64 = 2;
        const FIRST_SLEEP_TIME: Duration = Duration::from_millis(NUMBER_OF_MILLISECONDS);
        thread::sleep(FIRST_SLEEP_TIME);
        // Make sure we sleep for at least that long
        assert!(FIRST_SLEEP_TIME < timer.elapsed());
        // Reset the timer and make sure not too much time has passed
        timer.restart();
        assert!(timer.elapsed() < FIRST_SLEEP_TIME);
        // Try the same test with a longer duration
        const LONGER_SLEEP_TIME: Duration = Duration::from_millis(NUMBER_OF_MILLISECONDS * 2);
        thread::sleep(LONGER_SLEEP_TIME);
        assert!(timer.elapsed() > LONGER_SLEEP_TIME);
    }
}
