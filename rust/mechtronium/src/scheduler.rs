use std::time::Instant;

struct Scheduler {}

impl Scheduler {
    pub fn now() -> Instant {
        let now = Instant::now();
        return now;
    }
}
