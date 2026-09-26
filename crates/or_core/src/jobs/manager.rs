use super::{JobId, JobKind, JobState};
use std::{
    collections::{HashMap, VecDeque},
    error::Error,
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Condvar, Mutex, MutexGuard},
    thread::{self, JoinHandle},
};

/// Explicit bounded configuration for a [`JobManager`].
///
/// Every value must be non-zero; there is no unbounded default and no
/// CPU-count-derived product policy in this foundation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JobManagerConfig {
    max_workers: usize,
    queue_capacity: usize,
    max_records: usize,
}

impl JobManagerConfig {
    /// Validates and constructs a bounded configuration.
    pub fn new(
        max_workers: usize,
        queue_capacity: usize,
        max_records: usize,
    ) -> Result<Self, JobManagerConfigError> {
        if max_workers == 0 || queue_capacity == 0 || max_records == 0 {
            return Err(JobManagerConfigError);
        }
        Ok(Self {
            max_workers,
            queue_capacity,
            max_records,
        })
    }

    pub const fn max_workers(self) -> usize {
        self.max_workers
    }

    pub const fn queue_capacity(self) -> usize {
        self.queue_capacity
    }

    pub const fn max_records(self) -> usize {
        self.max_records
    }
}

/// A [`JobManagerConfig`] value was zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JobManagerConfigError;

impl fmt::Display for JobManagerConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("job manager configuration values must be non-zero")
    }
}

impl Error for JobManagerConfigError {}

/// Failure returned by a job body. Carries no payload into manager state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct JobFailure;

impl JobFailure {
    pub const fn new() -> Self {
        Self
    }
}

/// Cooperative cancellation context handed to a running job body.
#[derive(Clone)]
pub struct JobContext {
    cancelled: Arc<std::sync::atomic::AtomicBool>,
}

impl JobContext {
    /// Returns whether cancellation has been requested for this job.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Read-only snapshot of one tracked job.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JobSnapshot {
    pub id: JobId,
    pub kind: JobKind,
    pub state: JobState,
    pub sequence: u64,
}

/// Non-blocking submission failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JobSubmitError {
    /// The bounded pending queue is full. No worker or thread was created.
    QueueFull,
    /// All tracked record slots hold non-terminal jobs that cannot be evicted.
    RecordCapacityExceeded,
    /// The manager has been shut down and accepts no new work.
    Shutdown,
}

/// Cancelling an unknown job identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JobCancelError {
    NotFound,
}

/// Deterministic outcome of a cancel request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JobCancelOutcome {
    /// A queued job was cancelled before execution; its body will not run.
    CancelledQueued,
    /// A running job was signalled and becomes cancelled when it acknowledges.
    CancellationRequested,
    /// The job already finished; its terminal state is unchanged.
    AlreadyTerminal(JobState),
}

type JobBody = dyn FnOnce(&JobContext) -> Result<(), JobFailure> + Send + 'static;

struct JobRecord {
    kind: JobKind,
    state: JobState,
    sequence: u64,
    cancel: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Default)]
struct SharedState {
    records: HashMap<JobId, JobRecord>,
    pending: VecDeque<JobId>,
    tasks: HashMap<JobId, Box<JobBody>>,
    shutdown: bool,
    next_sequence: u64,
}

struct Shared {
    state: Mutex<SharedState>,
    available: Condvar,
    config: JobManagerConfig,
}

/// A bounded background control-plane job manager.
///
/// The manager owns exactly `max_workers` worker threads, a bounded pending
/// queue, and a bounded set of tracked job records. It never mutates canonical
/// project state, never increments `ProjectRevision`, and is not a realtime
/// media scheduler.
pub struct JobManager {
    shared: Arc<Shared>,
    workers: Mutex<Vec<JoinHandle<()>>>,
}

impl JobManager {
    /// Starts a manager with exactly `config.max_workers()` worker threads.
    ///
    /// The configuration is already validated, so the number of spawned threads
    /// is bounded by a caller-chosen value.
    pub fn new(config: JobManagerConfig) -> Self {
        let shared = Arc::new(Shared {
            state: Mutex::new(SharedState::default()),
            available: Condvar::new(),
            config,
        });

        let mut workers = Vec::with_capacity(config.max_workers());
        for index in 0..config.max_workers() {
            let shared = Arc::clone(&shared);
            let handle = thread::Builder::new()
                .name(format!("or-job-worker-{index}"))
                .spawn(move || worker_loop(shared))
                .expect("failed to spawn a bounded job worker thread");
            workers.push(handle);
        }

        Self {
            shared,
            workers: Mutex::new(workers),
        }
    }

    /// Submits one task. Returns immediately with a backpressure error when the
    /// pending queue is full; it never blocks and never spawns a worker.
    pub fn submit<F>(&self, kind: JobKind, body: F) -> Result<JobId, JobSubmitError>
    where
        F: FnOnce(&JobContext) -> Result<(), JobFailure> + Send + 'static,
    {
        let id = JobId::generate();
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let config = self.shared.config;

        let mut state = lock(&self.shared);
        if state.shutdown {
            return Err(JobSubmitError::Shutdown);
        }
        if state.pending.len() >= config.queue_capacity() {
            return Err(JobSubmitError::QueueFull);
        }
        if state.records.len() >= config.max_records() {
            let needed = state.records.len() - config.max_records() + 1;
            if count_terminal(&state.records) < needed {
                return Err(JobSubmitError::RecordCapacityExceeded);
            }
            evict_oldest_terminal(&mut state.records, needed);
        }

        let sequence = state.next_sequence;
        state.next_sequence += 1;
        state.records.insert(
            id,
            JobRecord {
                kind,
                state: JobState::Queued,
                sequence,
                cancel,
            },
        );
        state.tasks.insert(id, Box::new(body));
        state.pending.push_back(id);
        drop(state);

        self.shared.available.notify_all();
        Ok(id)
    }

    /// Reads one tracked job snapshot, or `None` when it is not tracked
    /// (unknown or already reclaimed).
    pub fn snapshot(&self, id: JobId) -> Option<JobSnapshot> {
        let state = lock(&self.shared);
        state.records.get(&id).map(|record| JobSnapshot {
            id,
            kind: record.kind,
            state: record.state,
            sequence: record.sequence,
        })
    }

    /// Number of currently tracked job records.
    pub fn record_count(&self) -> usize {
        lock(&self.shared).records.len()
    }

    /// Requests cooperative cancellation.
    pub fn cancel(&self, id: JobId) -> Result<JobCancelOutcome, JobCancelError> {
        let mut state = lock(&self.shared);
        let current = state
            .records
            .get(&id)
            .map(|record| record.state)
            .ok_or(JobCancelError::NotFound)?;

        match current {
            JobState::Queued => {
                state.pending.retain(|queued| *queued != id);
                state.tasks.remove(&id);
                if let Some(record) = state.records.get_mut(&id) {
                    record.state = JobState::Cancelled;
                    record
                        .cancel
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                }
                Ok(JobCancelOutcome::CancelledQueued)
            }
            JobState::Running => {
                if let Some(record) = state.records.get_mut(&id) {
                    record
                        .cancel
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                }
                Ok(JobCancelOutcome::CancellationRequested)
            }
            terminal => Ok(JobCancelOutcome::AlreadyTerminal(terminal)),
        }
    }

    /// Stops accepting work, skips queued jobs, signals running jobs, and joins
    /// every worker thread. Safe to call more than once.
    pub fn shutdown(&self) {
        {
            let mut state = lock(&self.shared);
            state.shutdown = true;
            let pending: Vec<JobId> = state.pending.drain(..).collect();
            for id in pending {
                state.tasks.remove(&id);
                if let Some(record) = state.records.get_mut(&id) {
                    if record.state == JobState::Queued {
                        record.state = JobState::Cancelled;
                        record
                            .cancel
                            .store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            }
            for record in state.records.values() {
                if record.state == JobState::Running {
                    record
                        .cancel
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }
            self.shared.available.notify_all();
        }
        self.join_workers();
    }

    fn join_workers(&self) {
        let handles = std::mem::take(&mut *lock_workers(&self.workers));
        for handle in handles {
            let _ = handle.join();
        }
    }
}

impl Drop for JobManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn worker_loop(shared: Arc<Shared>) {
    loop {
        let work = {
            let mut state = lock(&shared);
            while !state.shutdown && state.pending.is_empty() {
                state = shared
                    .available
                    .wait(state)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
            if state.shutdown {
                return;
            }
            let Some(id) = state.pending.pop_front() else {
                continue;
            };
            let Some(record) = state.records.get_mut(&id) else {
                continue;
            };
            if record.state != JobState::Queued {
                continue;
            }
            record.state = JobState::Running;
            let cancel = Arc::clone(&record.cancel);
            let body = state.tasks.remove(&id);
            body.map(|body| (id, cancel, body))
        };

        let Some((id, cancel, body)) = work else {
            continue;
        };
        let context = JobContext {
            cancelled: Arc::clone(&cancel),
        };
        let outcome = catch_unwind(AssertUnwindSafe(|| body(&context)));
        let was_cancelled = cancel.load(std::sync::atomic::Ordering::SeqCst);
        let final_state = match (outcome, was_cancelled) {
            (_, true) => JobState::Cancelled,
            (Ok(Ok(())), false) => JobState::Succeeded,
            (Ok(Err(_)) | Err(_), false) => JobState::Failed,
        };
        if let Some(record) = lock(&shared).records.get_mut(&id) {
            record.state = final_state;
        }
    }
}

fn lock(shared: &Shared) -> MutexGuard<'_, SharedState> {
    shared
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn lock_workers(workers: &Mutex<Vec<JoinHandle<()>>>) -> MutexGuard<'_, Vec<JoinHandle<()>>> {
    workers
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn is_terminal(state: JobState) -> bool {
    matches!(
        state,
        JobState::Succeeded | JobState::Failed | JobState::Cancelled
    )
}

fn count_terminal(records: &HashMap<JobId, JobRecord>) -> usize {
    records
        .values()
        .filter(|record| is_terminal(record.state))
        .count()
}

fn evict_oldest_terminal(records: &mut HashMap<JobId, JobRecord>, needed: usize) {
    for _ in 0..needed {
        let oldest = records
            .iter()
            .filter(|(_, record)| is_terminal(record.state))
            .min_by_key(|(_, record)| record.sequence)
            .map(|(id, _)| *id);
        match oldest {
            Some(id) => {
                records.remove(&id);
            }
            None => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
        time::{Duration, Instant},
    };

    struct Gate {
        released: Mutex<bool>,
        condvar: Condvar,
    }

    impl Gate {
        fn new() -> Self {
            Self {
                released: Mutex::new(false),
                condvar: Condvar::new(),
            }
        }

        fn wait(&self) {
            let mut guard = self.released.lock().unwrap();
            while !*guard {
                guard = self.condvar.wait(guard).unwrap();
            }
        }

        fn release(&self) {
            *self.released.lock().unwrap() = true;
            self.condvar.notify_all();
        }
    }

    fn wait_until(mut predicate: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !predicate() {
            assert!(
                Instant::now() < deadline,
                "condition was not met before the deadlock guard timeout"
            );
            std::thread::yield_now();
        }
    }

    fn state_of(manager: &JobManager, id: JobId) -> JobState {
        manager.snapshot(id).unwrap().state
    }

    fn submit_success(manager: &JobManager) -> JobId {
        manager.submit(JobKind::MediaProbe, |_| Ok(())).unwrap()
    }

    #[test]
    fn worker_pool_never_runs_more_than_the_configured_workers() {
        let manager = JobManager::new(JobManagerConfig::new(2, 8, 16).unwrap());
        let gate = Arc::new(Gate::new());
        let running = Arc::new(AtomicUsize::new(0));
        let completed = Arc::new(AtomicUsize::new(0));

        let mut ids = Vec::new();
        for _ in 0..4 {
            let gate = Arc::clone(&gate);
            let running = Arc::clone(&running);
            let completed = Arc::clone(&completed);
            ids.push(
                manager
                    .submit(JobKind::MediaProbe, move |_| {
                        running.fetch_add(1, Ordering::SeqCst);
                        gate.wait();
                        running.fetch_sub(1, Ordering::SeqCst);
                        completed.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    })
                    .unwrap(),
            );
        }

        wait_until(|| running.load(Ordering::SeqCst) == 2);
        let running_count = ids
            .iter()
            .filter(|id| state_of(&manager, **id) == JobState::Running)
            .count();
        let queued_count = ids
            .iter()
            .filter(|id| state_of(&manager, **id) == JobState::Queued)
            .count();
        assert_eq!(running_count, 2);
        assert_eq!(queued_count, 2);

        gate.release();
        wait_until(|| completed.load(Ordering::SeqCst) == 4);
        manager.shutdown();
    }

    #[test]
    fn submission_returns_queue_full_instead_of_blocking() {
        let manager = JobManager::new(JobManagerConfig::new(1, 1, 10).unwrap());
        let gate = Arc::new(Gate::new());
        let first = {
            let gate = Arc::clone(&gate);
            manager
                .submit(JobKind::MediaProbe, move |_| {
                    gate.wait();
                    Ok(())
                })
                .unwrap()
        };
        wait_until(|| state_of(&manager, first) == JobState::Running);

        let second = submit_success(&manager);
        assert_eq!(state_of(&manager, second), JobState::Queued);
        assert_eq!(
            manager.submit(JobKind::MediaProbe, |_| Ok(())),
            Err(JobSubmitError::QueueFull)
        );

        gate.release();
        manager.shutdown();
    }

    #[test]
    fn queued_job_cancelled_before_execution_never_runs() {
        let manager = JobManager::new(JobManagerConfig::new(1, 4, 10).unwrap());
        let gate = Arc::new(Gate::new());
        let first = {
            let gate = Arc::clone(&gate);
            manager
                .submit(JobKind::MediaProbe, move |_| {
                    gate.wait();
                    Ok(())
                })
                .unwrap()
        };
        wait_until(|| state_of(&manager, first) == JobState::Running);

        let ran = Arc::new(AtomicBool::new(false));
        let second = {
            let ran = Arc::clone(&ran);
            manager
                .submit(JobKind::MediaProbe, move |_| {
                    ran.store(true, Ordering::SeqCst);
                    Ok(())
                })
                .unwrap()
        };
        assert_eq!(
            manager.cancel(second),
            Ok(JobCancelOutcome::CancelledQueued)
        );
        assert_eq!(state_of(&manager, second), JobState::Cancelled);

        gate.release();
        wait_until(|| state_of(&manager, first) == JobState::Succeeded);
        assert!(!ran.load(Ordering::SeqCst));
        manager.shutdown();
    }

    #[test]
    fn running_job_observes_cooperative_cancellation() {
        let manager = JobManager::new(JobManagerConfig::new(1, 4, 10).unwrap());
        let observed = Arc::new(AtomicBool::new(false));
        let id = {
            let observed = Arc::clone(&observed);
            manager
                .submit(JobKind::MediaProbe, move |context| {
                    while !context.is_cancelled() {
                        std::thread::yield_now();
                    }
                    observed.store(true, Ordering::SeqCst);
                    Ok(())
                })
                .unwrap()
        };
        wait_until(|| state_of(&manager, id) == JobState::Running);

        assert_eq!(
            manager.cancel(id),
            Ok(JobCancelOutcome::CancellationRequested)
        );
        wait_until(|| state_of(&manager, id) == JobState::Cancelled);
        assert!(observed.load(Ordering::SeqCst));
        manager.shutdown();
    }

    #[test]
    fn successful_job_reaches_succeeded() {
        let manager = JobManager::new(JobManagerConfig::new(1, 4, 10).unwrap());
        let id = submit_success(&manager);
        wait_until(|| state_of(&manager, id) == JobState::Succeeded);
        manager.shutdown();
    }

    #[test]
    fn failing_job_is_failed_and_worker_continues() {
        let manager = JobManager::new(JobManagerConfig::new(1, 4, 10).unwrap());
        let failing = manager
            .submit(JobKind::MediaProbe, |_| Err(JobFailure::new()))
            .unwrap();
        wait_until(|| state_of(&manager, failing) == JobState::Failed);

        let succeeding = submit_success(&manager);
        wait_until(|| state_of(&manager, succeeding) == JobState::Succeeded);
        manager.shutdown();
    }

    #[test]
    fn panicking_job_is_contained_and_worker_survives() {
        let manager = JobManager::new(JobManagerConfig::new(1, 4, 10).unwrap());
        let panicking = manager
            .submit(JobKind::MediaProbe, |_| panic!("contained job panic"))
            .unwrap();
        wait_until(|| state_of(&manager, panicking) == JobState::Failed);

        let succeeding = submit_success(&manager);
        wait_until(|| state_of(&manager, succeeding) == JobState::Succeeded);
        manager.shutdown();
    }

    #[test]
    fn terminal_records_are_reclaimed_within_the_record_bound() {
        let manager = JobManager::new(JobManagerConfig::new(1, 8, 2).unwrap());
        let first = submit_success(&manager);
        wait_until(|| state_of(&manager, first) == JobState::Succeeded);
        let second = submit_success(&manager);
        wait_until(|| state_of(&manager, second) == JobState::Succeeded);
        assert_eq!(manager.record_count(), 2);

        let third = submit_success(&manager);
        wait_until(|| state_of(&manager, third) == JobState::Succeeded);
        assert_eq!(manager.record_count(), 2);
        assert!(manager.snapshot(first).is_none());
        assert!(manager.snapshot(third).is_some());
        manager.shutdown();
    }

    #[test]
    fn submissions_fail_when_all_records_are_non_terminal() {
        let manager = JobManager::new(JobManagerConfig::new(1, 4, 2).unwrap());
        let gate = Arc::new(Gate::new());
        let running = {
            let gate = Arc::clone(&gate);
            manager
                .submit(JobKind::MediaProbe, move |_| {
                    gate.wait();
                    Ok(())
                })
                .unwrap()
        };
        wait_until(|| state_of(&manager, running) == JobState::Running);

        let queued = submit_success(&manager);
        assert_eq!(state_of(&manager, queued), JobState::Queued);
        assert_eq!(manager.record_count(), 2);
        assert_eq!(
            manager.submit(JobKind::MediaProbe, |_| Ok(())),
            Err(JobSubmitError::RecordCapacityExceeded)
        );

        gate.release();
        manager.shutdown();
    }

    #[test]
    fn snapshots_carry_monotonic_manager_sequence() {
        let manager = JobManager::new(JobManagerConfig::new(2, 8, 16).unwrap());
        let first = submit_success(&manager);
        let second = submit_success(&manager);
        let third = submit_success(&manager);

        let first = manager.snapshot(first).unwrap().sequence;
        let second = manager.snapshot(second).unwrap().sequence;
        let third = manager.snapshot(third).unwrap().sequence;
        assert!(first < second && second < third);
        manager.shutdown();
    }

    #[test]
    fn shutdown_cancels_jobs_and_joins_workers() {
        let manager = JobManager::new(JobManagerConfig::new(2, 4, 8).unwrap());
        let first = manager
            .submit(JobKind::MediaProbe, |context| {
                while !context.is_cancelled() {
                    std::thread::yield_now();
                }
                Ok(())
            })
            .unwrap();
        let second = manager
            .submit(JobKind::MediaProbe, |context| {
                while !context.is_cancelled() {
                    std::thread::yield_now();
                }
                Ok(())
            })
            .unwrap();
        wait_until(|| {
            state_of(&manager, first) == JobState::Running
                && state_of(&manager, second) == JobState::Running
        });

        manager.shutdown();
        assert_eq!(state_of(&manager, first), JobState::Cancelled);
        assert_eq!(state_of(&manager, second), JobState::Cancelled);
        assert_eq!(
            manager.submit(JobKind::MediaProbe, |_| Ok(())),
            Err(JobSubmitError::Shutdown)
        );
    }
}
