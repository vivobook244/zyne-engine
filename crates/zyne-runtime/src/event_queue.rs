use std::collections::VecDeque;

use zyne_platform::PlatformEvent;

/// FIFO queue used by the runtime to defer platform event processing.
///
/// The queue is single-threaded by design. The platform event loop produces
/// events and the runtime decides when to drain and process them.
#[derive(Debug, Default)]
pub struct EventQueue {
    events: VecDeque<PlatformEvent>,
}

impl EventQueue {
    /// Creates an empty event queue.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Adds an event to the back of the queue.
    pub fn push(&mut self, event: PlatformEvent) { self.events.push_back(event); }

    /// Removes and returns the oldest queued event.
    pub fn pop(&mut self) -> Option<PlatformEvent> { self.events.pop_front() }

    /// Removes and returns all queued events in arrival order.
    pub fn drain(&mut self) -> Vec<PlatformEvent> { self.events.drain(..).collect() }

    /// Returns the number of queued events.
    #[must_use]
    pub fn len(&self) -> usize { self.events.len() }

    /// Returns whether the queue contains no events.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.events.is_empty() }

    /// Discards all queued events.
    pub fn clear(&mut self) { self.events.clear(); }
}

#[cfg(test)]
mod tests {
    use super::EventQueue;
    use zyne_platform::{PlatformEvent, WindowEvent};

    #[test]
    fn events_are_popped_in_fifo_order() {
        let mut queue = EventQueue::new();
        queue.push(PlatformEvent::Resumed);
        queue.push(PlatformEvent::Window(WindowEvent::Focused(true)));
        queue.push(PlatformEvent::Suspended);
        assert_eq!(queue.pop(), Some(PlatformEvent::Resumed));
        assert_eq!(queue.pop(), Some(PlatformEvent::Window(WindowEvent::Focused(true))));
        assert_eq!(queue.pop(), Some(PlatformEvent::Suspended));
        assert!(queue.is_empty());
    }

    #[test]
    fn drain_returns_events_in_order_and_empties_queue() {
        let mut queue = EventQueue::new();
        queue.push(PlatformEvent::Resumed);
        queue.push(PlatformEvent::Suspended);
        assert_eq!(queue.drain(), vec![PlatformEvent::Resumed, PlatformEvent::Suspended]);
        assert!(queue.is_empty());
    }

    #[test]
    fn clear_discards_pending_events() {
        let mut queue = EventQueue::new();
        queue.push(PlatformEvent::Resumed);
        queue.clear();
        assert_eq!(queue.len(), 0);
        assert!(queue.pop().is_none());
    }
}
