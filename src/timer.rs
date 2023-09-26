use std::time::{Duration, Instant};

pub struct Timer {
    start_time: Instant,
}

impl Timer {
    // Creates a new timer and starts it.
    pub fn new() -> Self {
        Timer {
            start_time: Instant::now(),
        }
    }

    // Restarts the timer.
    pub fn restart(&mut self) {
        self.start_time = Instant::now();
    }

    // Returns the elapsed time since the timer was started (or restarted).
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
        const FIRST_SLEEP_TIME: Duration = Duration::from_millis(2);
        thread::sleep(FIRST_SLEEP_TIME);
        // Make sure we sleep for at least that long
        assert!(FIRST_SLEEP_TIME < timer.elapsed());
        // Reset the timer and make sure not too much time has passed
        timer.restart();
        assert!(timer.elapsed() < FIRST_SLEEP_TIME);
        // Try the same test with a longer duration
        const SECOND_SLEEP_TIME: Duration =
            Duration::from_millis((FIRST_SLEEP_TIME.as_millis() * 2) as u64);
        thread::sleep(SECOND_SLEEP_TIME);
        assert!(timer.elapsed() > SECOND_SLEEP_TIME);
    }
}
