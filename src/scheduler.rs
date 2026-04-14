use cosmic::iced::Subscription;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum SchedulerEvent {
    Tick(Instant),
}

pub fn timer_subscription(paused: bool) -> Subscription<SchedulerEvent> {
    if paused {
        return Subscription::none();
    }

    cosmic::iced::time::every(Duration::from_secs(1)).map(SchedulerEvent::Tick)
}
