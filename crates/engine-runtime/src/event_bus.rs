use crate::{EngineError, EngineErrorCode, EngineEvent, EngineSnapshot, Result, lock};
use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex, Weak},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionError {
    Timeout,
    Closed,
    /// This subscriber alone fell behind. Retained events can still be read;
    /// call runtime.snapshot() to resynchronize immediately.
    Lagged {
        missed: u64,
    },
}
impl std::fmt::Display for SubscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SubscriptionError {}

struct Mailbox {
    events: VecDeque<EngineEvent>,
    missed: u64,
    closed: bool,
}
struct Listener {
    mailbox: Mutex<Mailbox>,
    ready: Condvar,
    capacity: usize,
}

/// Every subscription has its own bounded mailbox, not a shared work queue.
/// Overflow drops the oldest event and reports Lagged before the next event.
pub struct Subscription {
    listener: Arc<Listener>,
}
impl Subscription {
    pub fn recv(&self) -> std::result::Result<EngineEvent, SubscriptionError> {
        self.receive(None)
    }
    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> std::result::Result<EngineEvent, SubscriptionError> {
        self.receive(Some(timeout))
    }
    fn receive(
        &self,
        timeout: Option<Duration>,
    ) -> std::result::Result<EngineEvent, SubscriptionError> {
        let start = Instant::now();
        let mut mailbox = lock(&self.listener.mailbox);
        loop {
            if mailbox.missed != 0 {
                return Err(SubscriptionError::Lagged {
                    missed: std::mem::take(&mut mailbox.missed),
                });
            }
            if let Some(event) = mailbox.events.pop_front() {
                return Ok(event);
            }
            if mailbox.closed {
                return Err(SubscriptionError::Closed);
            }
            mailbox = if let Some(timeout) = timeout {
                let remaining = timeout.saturating_sub(start.elapsed());
                if remaining.is_zero() {
                    return Err(SubscriptionError::Timeout);
                }
                self.listener
                    .ready
                    .wait_timeout(mailbox, remaining)
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .0
            } else {
                self.listener
                    .ready
                    .wait(mailbox)
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
            };
        }
    }
}

struct BusState {
    snapshot: EngineSnapshot,
    listeners: Vec<Weak<Listener>>,
    closed: bool,
}
pub(crate) struct EventBus {
    state: Mutex<BusState>,
    capacity: usize,
    max_subscribers: usize,
}
impl EventBus {
    pub fn new(snapshot: EngineSnapshot, capacity: usize, max_subscribers: usize) -> Self {
        Self {
            state: Mutex::new(BusState {
                snapshot,
                listeners: Vec::new(),
                closed: false,
            }),
            capacity,
            max_subscribers,
        }
    }
    pub fn snapshot(&self) -> EngineSnapshot {
        lock(&self.state).snapshot.clone()
    }
    pub fn subscribe(&self) -> Result<Subscription> {
        let mut state = lock(&self.state);
        if state.closed {
            return Err(EngineError::new(
                EngineErrorCode::RuntimeClosed,
                "runtime has shut down",
            ));
        }
        state
            .listeners
            .retain(|listener| listener.strong_count() != 0);
        if state.listeners.len() >= self.max_subscribers {
            return Err(EngineError::new(
                EngineErrorCode::SubscriberLimit,
                "maximum number of listeners reached",
            ));
        }
        let listener = Arc::new(Listener {
            mailbox: Mutex::new(Mailbox {
                events: VecDeque::from([EngineEvent::Snapshot {
                    snapshot: Box::new(state.snapshot.clone()),
                }]),
                missed: 0,
                closed: false,
            }),
            ready: Condvar::new(),
            capacity: self.capacity,
        });
        state.listeners.push(Arc::downgrade(&listener));
        Ok(Subscription { listener })
    }
    pub fn update(&self, snapshot: &mut EngineSnapshot) {
        let mut state = lock(&self.state);
        snapshot.revision = state.snapshot.revision.saturating_add(1);
        state.snapshot = snapshot.clone();
        Self::broadcast(
            &mut state,
            EngineEvent::Snapshot {
                snapshot: Box::new(snapshot.clone()),
            },
        );
    }
    pub fn error(&self, error: EngineError) {
        Self::broadcast(&mut lock(&self.state), EngineEvent::Error { error });
    }
    pub fn transcript_partial(&self, segment: crate::TranscriptSegment) {
        Self::broadcast(
            &mut lock(&self.state),
            EngineEvent::TranscriptPartial { segment },
        );
    }
    pub fn transcript_update(&self, update: crate::TranscriptUpdate) {
        Self::broadcast(
            &mut lock(&self.state),
            EngineEvent::TranscriptUpdate { update },
        );
    }
    pub fn transcript_final(&self, segment: crate::TranscriptSegment) {
        Self::broadcast(
            &mut lock(&self.state),
            EngineEvent::TranscriptFinal { segment },
        );
    }
    pub fn transcription_error(&self, error: EngineError) {
        Self::broadcast(
            &mut lock(&self.state),
            EngineEvent::TranscriptionError { error },
        );
    }
    fn broadcast(state: &mut BusState, event: EngineEvent) {
        state.listeners.retain(|weak| {
            let Some(listener) = weak.upgrade() else {
                return false;
            };
            let mut mailbox = lock(&listener.mailbox);
            if mailbox.events.len() == listener.capacity {
                mailbox.events.pop_front();
                mailbox.missed = mailbox.missed.saturating_add(1);
            }
            mailbox.events.push_back(event.clone());
            listener.ready.notify_one();
            true
        });
    }
    pub fn close(&self) {
        let mut state = lock(&self.state);
        if matches!(
            state.snapshot.status,
            crate::EngineStatus::Starting
                | crate::EngineStatus::Recording
                | crate::EngineStatus::Stopping
        ) {
            // Unexpected worker exit must not leave clients showing Recording.
            let error = EngineError::new(
                EngineErrorCode::WorkerFailed,
                "worker exited before session shutdown completed",
            );
            state.snapshot.status = crate::EngineStatus::Error;
            state.snapshot.microphone.active = false;
            state.snapshot.system_audio.active = false;
            state.snapshot.last_error = Some(error.clone());
            state.snapshot.revision = state.snapshot.revision.saturating_add(1);
            Self::broadcast(&mut state, EngineEvent::Error { error });
            let snapshot = Box::new(state.snapshot.clone());
            Self::broadcast(&mut state, EngineEvent::Snapshot { snapshot });
        }
        state.closed = true;
        for listener in state.listeners.iter().filter_map(Weak::upgrade) {
            lock(&listener.mailbox).closed = true;
            listener.ready.notify_all();
        }
        state.listeners.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closure_wakes_waiter_and_marks_unexpected_exit() {
        let bus = EventBus::new(EngineSnapshot::default(), 4, 2);
        let listener = bus.subscribe().unwrap();
        listener.recv().unwrap();
        let waiter = std::thread::spawn(move || listener.recv_timeout(Duration::from_secs(1)));
        bus.close();
        assert_eq!(waiter.join().unwrap(), Err(SubscriptionError::Closed));

        let bus = EventBus::new(
            EngineSnapshot {
                status: crate::EngineStatus::Recording,
                ..Default::default()
            },
            4,
            2,
        );
        bus.close();
        assert_eq!(bus.snapshot().status, crate::EngineStatus::Error);
        assert_eq!(
            bus.snapshot().last_error.unwrap().code,
            EngineErrorCode::WorkerFailed
        );
    }
    #[test]
    fn broadcast_initial_snapshot_and_disconnect() {
        let bus = EventBus::new(EngineSnapshot::default(), 4, 2);
        let a = bus.subscribe().unwrap();
        let b = bus.subscribe().unwrap();
        assert_eq!(a.recv().unwrap(), b.recv().unwrap());
        assert!(matches!(
            bus.subscribe(),
            Err(EngineError {
                code: EngineErrorCode::SubscriberLimit,
                ..
            })
        ));
        let mut state = bus.snapshot();
        state.microphone.enabled = true;
        bus.update(&mut state);
        assert_eq!(a.recv().unwrap(), b.recv().unwrap());
        assert_eq!(bus.snapshot(), state);
        drop(b);
        assert!(bus.subscribe().is_ok());
        bus.close();
        assert_eq!(a.recv(), Err(SubscriptionError::Closed));
    }
    #[test]
    fn slow_listener_cannot_block_or_steal_from_fast_listener() {
        let bus = EventBus::new(EngineSnapshot::default(), 2, 2);
        let slow = bus.subscribe().unwrap();
        let fast = bus.subscribe().unwrap();
        fast.recv().unwrap();
        let mut state = bus.snapshot();
        for _ in 0..10 {
            bus.update(&mut state);
            assert_eq!(
                fast.recv().unwrap(),
                EngineEvent::Snapshot {
                    snapshot: Box::new(state.clone())
                }
            );
        }
        assert_eq!(slow.recv(), Err(SubscriptionError::Lagged { missed: 9 }));
        slow.recv().unwrap();
        assert_eq!(
            slow.recv().unwrap(),
            EngineEvent::Snapshot {
                snapshot: Box::new(state)
            }
        );
        assert_eq!(
            slow.recv_timeout(Duration::ZERO),
            Err(SubscriptionError::Timeout)
        );
    }
}
