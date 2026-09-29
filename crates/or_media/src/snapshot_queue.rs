use or_runtime::{
    BoundedQueue, CancellationToken, QueueConfigError, QueueReceiveError, QueueSendError,
    RenderSnapshot,
};
use std::sync::{Arc, Mutex, MutexGuard};

/// One queued value tied to the immutable project snapshot that requested it.
#[derive(Debug)]
pub struct SnapshotItem<T> {
    snapshot: RenderSnapshot,
    value: T,
}

impl<T> SnapshotItem<T> {
    pub const fn snapshot(&self) -> RenderSnapshot {
        self.snapshot
    }

    pub const fn value(&self) -> &T {
        &self.value
    }

    pub fn into_value(self) -> T {
        self.value
    }
}

/// A bounded queue that drops stale snapshot output when it is consumed.
pub struct SnapshotQueue<T> {
    queue: Arc<BoundedQueue<SnapshotItem<T>>>,
    active_snapshot: Arc<Mutex<RenderSnapshot>>,
}

impl<T> Clone for SnapshotQueue<T> {
    fn clone(&self) -> Self {
        Self {
            queue: Arc::clone(&self.queue),
            active_snapshot: Arc::clone(&self.active_snapshot),
        }
    }
}

impl<T> SnapshotQueue<T> {
    pub fn new(snapshot: RenderSnapshot, capacity: usize) -> Result<Self, QueueConfigError> {
        Ok(Self {
            queue: Arc::new(BoundedQueue::new(capacity)?),
            active_snapshot: Arc::new(Mutex::new(snapshot)),
        })
    }

    /// Activates a new request. Already queued old values are discarded on read.
    pub fn activate(&self, snapshot: RenderSnapshot) {
        *lock(&self.active_snapshot) = snapshot;
    }

    pub fn is_current(&self, snapshot: RenderSnapshot) -> bool {
        *lock(&self.active_snapshot) == snapshot
    }

    pub fn capacity(&self) -> usize {
        self.queue.capacity()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn close(&self) {
        self.queue.close();
    }

    /// Adds without waiting; a full queue reports backpressure with the value.
    pub fn try_push(
        &self,
        snapshot: RenderSnapshot,
        value: T,
    ) -> Result<(), SnapshotQueueSendError<T>> {
        if !self.is_current(snapshot) {
            return Err(SnapshotQueueSendError::StaleSnapshot(value));
        }
        self.queue
            .try_push(SnapshotItem { snapshot, value })
            .map_err(map_send_error)
    }

    /// Waits for capacity and observes cooperative cancellation.
    pub fn push(
        &self,
        snapshot: RenderSnapshot,
        value: T,
        cancellation: &CancellationToken,
    ) -> Result<(), SnapshotQueueSendError<T>> {
        if !self.is_current(snapshot) {
            return Err(SnapshotQueueSendError::StaleSnapshot(value));
        }
        self.queue
            .push(SnapshotItem { snapshot, value }, cancellation)
            .map_err(map_send_error)
    }

    /// Returns the next current item, draining stale values as they appear.
    pub fn try_pop_current(&self) -> Result<Option<SnapshotItem<T>>, QueueReceiveError> {
        loop {
            let Some(item) = self.queue.try_pop()? else {
                return Ok(None);
            };
            if self.is_current(item.snapshot) {
                return Ok(Some(item));
            }
        }
    }

    /// Waits for current output, discarding stale values and observing cancellation.
    pub fn pop_current(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<SnapshotItem<T>, QueueReceiveError> {
        loop {
            let item = self.queue.pop(cancellation)?;
            if self.is_current(item.snapshot) {
                return Ok(item);
            }
        }
    }
}

fn map_send_error<T>(error: QueueSendError<SnapshotItem<T>>) -> SnapshotQueueSendError<T> {
    match error {
        QueueSendError::Backpressure(item) => SnapshotQueueSendError::Backpressure(item.value),
        QueueSendError::Cancelled(item) => SnapshotQueueSendError::Cancelled(item.value),
        QueueSendError::Closed(item) => SnapshotQueueSendError::Closed(item.value),
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Debug, Eq, PartialEq)]
pub enum SnapshotQueueSendError<T> {
    StaleSnapshot(T),
    Backpressure(T),
    Cancelled(T),
    Closed(T),
}

#[cfg(test)]
mod tests {
    use super::*;
    use or_core::{ProjectDocument, RationalTime, TimeRange};
    use or_runtime::RenderSnapshot;

    fn snapshot(revision: u64) -> RenderSnapshot {
        let project = ProjectDocument::new(format!("queue test {revision}"));
        RenderSnapshot::from_project(
            &project,
            TimeRange::new(RationalTime::ZERO, RationalTime::ZERO).unwrap(),
        )
    }

    #[test]
    fn queue_bounds_values_and_returns_backpressure() {
        let snapshot = snapshot(0);
        let queue = SnapshotQueue::new(snapshot, 1).unwrap();
        queue.try_push(snapshot, 1).unwrap();
        assert_eq!(
            queue.try_push(snapshot, 2),
            Err(SnapshotQueueSendError::Backpressure(2))
        );
        assert_eq!(queue.try_pop_current().unwrap().unwrap().into_value(), 1);
    }

    #[test]
    fn queue_drops_stale_snapshot_output_and_rejects_stale_sends() {
        let old = snapshot(0);
        let new = snapshot(1);
        let queue = SnapshotQueue::new(old, 2).unwrap();
        queue.try_push(old, 1).unwrap();
        queue.activate(new);

        assert!(queue.try_pop_current().unwrap().is_none());
        assert_eq!(
            queue.try_push(old, 2),
            Err(SnapshotQueueSendError::StaleSnapshot(2))
        );
        queue.try_push(new, 3).unwrap();
        assert_eq!(queue.try_pop_current().unwrap().unwrap().into_value(), 3);
    }

    #[test]
    fn queue_cancellation_returns_the_pending_value() {
        let snapshot = snapshot(0);
        let queue = SnapshotQueue::new(snapshot, 1).unwrap();
        queue.try_push(snapshot, 1).unwrap();
        let cancellation = CancellationToken::new();
        cancellation.cancel();

        assert_eq!(
            queue.push(snapshot, 2, &cancellation),
            Err(SnapshotQueueSendError::Cancelled(2))
        );
    }
}
