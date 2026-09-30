//! Runtime-plane contracts for Phase 7A.
//!
//! This crate contains immutable snapshot identity, frame metadata and
//! ownership, bounded queue primitives, resource budgets, and centralized
//! capability selection. It deliberately has no platform, GPU, media, IPC, or
//! Flutter dependency. Runtime adapters may build on these contracts later;
//! canonical project state remains owned by `or_core`.

use or_core::{ProjectDocument, ProjectId, ProjectRevision, RationalTime, TimeRange};
use std::{
    collections::VecDeque,
    error::Error,
    fmt,
    sync::atomic::{AtomicBool, Ordering},
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::Duration,
};

const QUEUE_CANCELLATION_POLL: Duration = Duration::from_millis(10);
pub const MAX_PROVIDER_ID_BYTES: usize = 128;
pub const SOFTWARE_PROVIDER_ID: &str = "software";

/// An immutable, versioned view of canonical project state for runtime work.
///
/// The snapshot carries only the control-plane identity and exact requested
/// range in 7A. It has no mutable `ProjectDocument`, platform handle, or
/// presentation state. Later evaluators can attach an immutable evaluated
/// payload without changing the revision/time contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderSnapshot {
    project_id: ProjectId,
    project_revision: ProjectRevision,
    requested_range: TimeRange,
}

impl RenderSnapshot {
    /// Creates a snapshot for an exact project revision and time range.
    pub const fn new(
        project_id: ProjectId,
        project_revision: ProjectRevision,
        requested_range: TimeRange,
    ) -> Self {
        Self {
            project_id,
            project_revision,
            requested_range,
        }
    }

    /// Evaluates the current control-plane identity into a runtime snapshot.
    /// No project mutation or revision increment occurs.
    pub fn from_project(document: &ProjectDocument, requested_range: TimeRange) -> Self {
        Self::new(document.id(), document.revision(), requested_range)
    }

    /// Creates a zero-duration snapshot request at one exact rational time.
    pub fn at_time(
        document: &ProjectDocument,
        requested_time: RationalTime,
    ) -> Result<Self, or_core::TimeError> {
        Ok(Self::from_project(
            document,
            TimeRange::new(requested_time, RationalTime::ZERO)?,
        ))
    }

    pub const fn project_id(self) -> ProjectId {
        self.project_id
    }

    pub const fn project_revision(self) -> ProjectRevision {
        self.project_revision
    }

    pub const fn requested_range(self) -> TimeRange {
        self.requested_range
    }

    /// Returns whether this snapshot still names the supplied project version.
    pub fn matches(self, project_id: ProjectId, project_revision: ProjectRevision) -> bool {
        self.project_id == project_id && self.project_revision == project_revision
    }
}

/// Cooperative cancellation shared by runtime producers and consumers.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn check(&self) -> Result<(), CancellationError> {
        if self.is_cancelled() {
            Err(CancellationError)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CancellationError;

impl fmt::Display for CancellationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("runtime operation was cancelled")
    }
}

impl Error for CancellationError {}

/// A fixed-capacity queue. It never allocates beyond its configured item
/// capacity and makes pressure observable to the producer.
pub struct BoundedQueue<T> {
    capacity: usize,
    state: Mutex<QueueState<T>>,
    not_empty: Condvar,
    not_full: Condvar,
}

struct QueueState<T> {
    items: VecDeque<T>,
    closed: bool,
}

impl<T> BoundedQueue<T> {
    pub fn new(capacity: usize) -> Result<Self, QueueConfigError> {
        if capacity == 0 {
            return Err(QueueConfigError::ZeroCapacity);
        }
        Ok(Self {
            capacity,
            state: Mutex::new(QueueState {
                items: VecDeque::new(),
                closed: false,
            }),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
        })
    }

    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        lock(&self.state).items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_closed(&self) -> bool {
        lock(&self.state).closed
    }

    /// Attempts to enqueue without waiting. `Backpressure` returns ownership
    /// of the item so a caller may drop obsolete preview work deliberately.
    pub fn try_push(&self, item: T) -> Result<(), QueueSendError<T>> {
        let mut state = lock(&self.state);
        if state.closed {
            return Err(QueueSendError::Closed(item));
        }
        if state.items.len() == self.capacity {
            return Err(QueueSendError::Backpressure(item));
        }
        state.items.push_back(item);
        self.not_empty.notify_one();
        Ok(())
    }

    /// Enqueues after waiting for capacity, with cooperative cancellation.
    pub fn push(&self, item: T, cancellation: &CancellationToken) -> Result<(), QueueSendError<T>> {
        let mut state = lock(&self.state);
        let mut item = Some(item);
        loop {
            if cancellation.is_cancelled() {
                return Err(QueueSendError::Cancelled(
                    item.take().expect("queue item present"),
                ));
            }
            if state.closed {
                return Err(QueueSendError::Closed(
                    item.take().expect("queue item present"),
                ));
            }
            if state.items.len() < self.capacity {
                state
                    .items
                    .push_back(item.take().expect("queue item present"));
                self.not_empty.notify_one();
                return Ok(());
            }
            state = wait_timeout(&self.not_full, state);
        }
    }

    pub fn try_pop(&self) -> Result<Option<T>, QueueReceiveError> {
        let mut state = lock(&self.state);
        let item = state.items.pop_front();
        if item.is_some() {
            self.not_full.notify_one();
            return Ok(item);
        }
        if state.closed {
            Err(QueueReceiveError::Closed)
        } else {
            Ok(None)
        }
    }

    /// Receives after waiting for an item, with cooperative cancellation.
    /// Closing drains already queued items before returning `Closed`.
    pub fn pop(&self, cancellation: &CancellationToken) -> Result<T, QueueReceiveError> {
        let mut state = lock(&self.state);
        loop {
            if let Some(item) = state.items.pop_front() {
                self.not_full.notify_one();
                return Ok(item);
            }
            if cancellation.is_cancelled() {
                return Err(QueueReceiveError::Cancelled);
            }
            if state.closed {
                return Err(QueueReceiveError::Closed);
            }
            state = wait_timeout(&self.not_empty, state);
        }
    }

    /// Closes the queue. Producers fail immediately; consumers may drain the
    /// existing bounded contents before observing `Closed`.
    pub fn close(&self) {
        let mut state = lock(&self.state);
        state.closed = true;
        self.not_empty.notify_all();
        self.not_full.notify_all();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueueConfigError {
    ZeroCapacity,
}

impl fmt::Display for QueueConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded queue capacity must be positive")
    }
}

impl Error for QueueConfigError {}

#[derive(Debug, PartialEq, Eq)]
pub enum QueueSendError<T> {
    Backpressure(T),
    Closed(T),
    Cancelled(T),
}

impl<T> QueueSendError<T> {
    pub fn is_backpressure(&self) -> bool {
        matches!(self, Self::Backpressure(_))
    }

    pub fn into_item(self) -> T {
        match self {
            Self::Backpressure(item) | Self::Closed(item) | Self::Cancelled(item) => item,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueueReceiveError {
    Closed,
    Cancelled,
}

impl fmt::Display for QueueReceiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Closed => "bounded queue is closed",
            Self::Cancelled => "bounded queue operation was cancelled",
        })
    }
}

impl Error for QueueReceiveError {}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn wait_timeout<'a, T>(condvar: &Condvar, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
    condvar
        .wait_timeout(guard, QUEUE_CANCELLATION_POLL)
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .0
}

/// The metadata needed to describe a decoded or rendered frame.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FrameDescriptor {
    memory_domain: FrameMemoryDomain,
    width: u32,
    height: u32,
    pixel_format: FramePixelFormat,
    color: FrameColorInfo,
    timing: FrameTiming,
    access: FrameAccess,
}

impl FrameDescriptor {
    pub fn new(
        memory_domain: FrameMemoryDomain,
        width: u32,
        height: u32,
        pixel_format: FramePixelFormat,
        color: FrameColorInfo,
        timing: FrameTiming,
        access: FrameAccess,
    ) -> Result<Self, FrameDescriptorError> {
        if width == 0 {
            return Err(FrameDescriptorError::ZeroWidth);
        }
        if height == 0 {
            return Err(FrameDescriptorError::ZeroHeight);
        }
        Ok(Self {
            memory_domain,
            width,
            height,
            pixel_format,
            color,
            timing,
            access,
        })
    }

    pub fn software(
        width: u32,
        height: u32,
        pixel_format: FramePixelFormat,
        timestamp: RationalTime,
    ) -> Result<Self, FrameDescriptorError> {
        Self::new(
            FrameMemoryDomain::Software,
            width,
            height,
            pixel_format,
            FrameColorInfo::default(),
            FrameTiming::at(timestamp),
            FrameAccess::ReadOnly,
        )
    }

    pub fn hardware_surface(
        width: u32,
        height: u32,
        pixel_format: FramePixelFormat,
        timestamp: RationalTime,
    ) -> Result<Self, FrameDescriptorError> {
        Self::new(
            FrameMemoryDomain::HardwareSurface,
            width,
            height,
            pixel_format,
            FrameColorInfo::default(),
            FrameTiming::at(timestamp),
            FrameAccess::ReadOnly,
        )
    }

    pub const fn memory_domain(self) -> FrameMemoryDomain {
        self.memory_domain
    }

    pub const fn width(self) -> u32 {
        self.width
    }

    pub const fn height(self) -> u32 {
        self.height
    }

    pub const fn pixel_format(self) -> FramePixelFormat {
        self.pixel_format
    }

    pub const fn color(self) -> FrameColorInfo {
        self.color
    }

    pub const fn timing(self) -> FrameTiming {
        self.timing
    }

    pub const fn access(self) -> FrameAccess {
        self.access
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameDescriptorError {
    ZeroWidth,
    ZeroHeight,
}

impl fmt::Display for FrameDescriptorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ZeroWidth => "frame width must be positive",
            Self::ZeroHeight => "frame height must be positive",
        })
    }
}

impl Error for FrameDescriptorError {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FrameMemoryDomain {
    Software,
    HardwareSurface,
    ExternalSurface,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FramePixelFormat {
    Rgba8,
    Bgra8,
    Nv12,
    P010,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FrameAccess {
    ReadOnly,
    ReadWrite,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FrameColorInfo {
    color_space: FrameColorSpace,
    range: FrameColorRange,
}

impl FrameColorInfo {
    pub const fn new(color_space: FrameColorSpace, range: FrameColorRange) -> Self {
        Self { color_space, range }
    }

    pub const fn color_space(self) -> FrameColorSpace {
        self.color_space
    }

    pub const fn range(self) -> FrameColorRange {
        self.range
    }
}

impl Default for FrameColorInfo {
    fn default() -> Self {
        Self::new(FrameColorSpace::Unknown, FrameColorRange::Unknown)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FrameColorSpace {
    Unknown,
    Srgb,
    Bt709,
    Bt2020,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FrameColorRange {
    Unknown,
    Full,
    Limited,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FrameTiming {
    timestamp: RationalTime,
    duration: Option<RationalTime>,
}

impl FrameTiming {
    pub fn new(
        timestamp: RationalTime,
        duration: Option<RationalTime>,
    ) -> Result<Self, FrameTimingError> {
        if duration.is_some_and(RationalTime::is_negative) {
            return Err(FrameTimingError::NegativeDuration);
        }
        Ok(Self {
            timestamp,
            duration,
        })
    }

    pub const fn at(timestamp: RationalTime) -> Self {
        Self {
            timestamp,
            duration: None,
        }
    }

    pub const fn timestamp(self) -> RationalTime {
        self.timestamp
    }

    pub const fn duration(self) -> Option<RationalTime> {
        self.duration
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameTimingError {
    NegativeDuration,
}

impl fmt::Display for FrameTimingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("frame duration must be nonnegative")
    }
}

impl Error for FrameTimingError {}

/// Explicit ownership for one runtime frame resource.
///
/// Software leases own their byte buffer. Hardware and external leases carry a
/// runtime-only release callback; no platform handle is represented in this
/// crate or exposed to Dart, IPC, project state, or cache identity.
pub struct FrameLease {
    descriptor: FrameDescriptor,
    storage: Option<FrameLeaseStorage>,
}

enum FrameLeaseStorage {
    Software(Arc<[u8]>),
    Release(Box<dyn FnOnce() + Send + 'static>),
}

impl FrameLease {
    /// Creates an empty lease with a no-op release callback for metadata-only
    /// tests and adapters that own the resource elsewhere.
    pub fn new(descriptor: FrameDescriptor) -> Self {
        Self::with_release(descriptor, || {})
    }

    pub fn from_software(
        descriptor: FrameDescriptor,
        bytes: Vec<u8>,
    ) -> Result<Self, FrameLeaseError> {
        if descriptor.memory_domain() != FrameMemoryDomain::Software {
            return Err(FrameLeaseError::SoftwareStorageRequiresSoftwareDomain);
        }
        Ok(Self {
            descriptor,
            storage: Some(FrameLeaseStorage::Software(Arc::from(
                bytes.into_boxed_slice(),
            ))),
        })
    }

    pub fn with_release<F>(descriptor: FrameDescriptor, release: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            descriptor,
            storage: Some(FrameLeaseStorage::Release(Box::new(release))),
        }
    }

    pub const fn descriptor(&self) -> &FrameDescriptor {
        &self.descriptor
    }

    pub fn software_bytes(&self) -> Option<&[u8]> {
        match self.storage.as_ref() {
            Some(FrameLeaseStorage::Software(bytes)) => Some(bytes),
            Some(FrameLeaseStorage::Release(_)) | None => None,
        }
    }

    fn shared_software_copy(&self) -> Option<Self> {
        match self.storage.as_ref()? {
            FrameLeaseStorage::Software(bytes) => Some(Self {
                descriptor: self.descriptor,
                storage: Some(FrameLeaseStorage::Software(Arc::clone(bytes))),
            }),
            FrameLeaseStorage::Release(_) => None,
        }
    }

    pub fn is_released(&self) -> bool {
        self.storage.is_none()
    }

    /// Releases the resource exactly once. Dropping an unreleased lease also
    /// releases it, so queue cancellation cannot leak a frame.
    pub fn release(mut self) {
        self.release_inner();
    }

    fn release_inner(&mut self) {
        match self.storage.take() {
            Some(FrameLeaseStorage::Software(_)) | None => {}
            Some(FrameLeaseStorage::Release(release)) => release(),
        }
    }
}

impl fmt::Debug for FrameLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FrameLease")
            .field("descriptor", &self.descriptor)
            .field("released", &self.is_released())
            .finish()
    }
}

impl Drop for FrameLease {
    fn drop(&mut self) {
        self.release_inner();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameLeaseError {
    SoftwareStorageRequiresSoftwareDomain,
}

impl fmt::Display for FrameLeaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("software frame storage requires a software frame descriptor")
    }
}

impl Error for FrameLeaseError {}

const VIEWER_MAX_FRAME_BYTES: usize = 256 * 1024 * 1024;
const VIEWER_MAX_IN_FLIGHT: usize = 3;

struct ViewerFrameState {
    generation: Option<u64>,
    latest: Option<FrameLease>,
    in_flight: usize,
}

/// Bounded CPU presentation storage shared by native Flutter texture adapters.
/// It keeps one latest frame and at most three Flutter-owned leases in flight.
pub struct ViewerTextureAdapter {
    state: Arc<Mutex<ViewerFrameState>>,
}

impl Default for ViewerTextureAdapter {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(ViewerFrameState {
                generation: None,
                latest: None,
                in_flight: 0,
            })),
        }
    }
}

impl ViewerTextureAdapter {
    /// Invalidates earlier work when a seek or project revision supersedes it.
    pub fn advance_generation(&self, generation: u64) -> bool {
        let mut state = lock(&self.state);
        if state
            .generation
            .is_some_and(|current| generation <= current)
        {
            return false;
        }
        state.generation = Some(generation);
        state.latest = None;
        true
    }

    /// Publishes RGBA software pixels as a premultiplied BGRA Flutter frame.
    /// Older generations are rejected; the mailbox retains only the newest
    /// frame, while acquired leases remain alive through Flutter's release.
    pub fn publish_rgba(
        &self,
        generation: u64,
        width: u32,
        height: u32,
        timestamp: RationalTime,
        rgba: &[u8],
    ) -> Result<(), ViewerTextureError> {
        let byte_count = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|bytes| *bytes <= VIEWER_MAX_FRAME_BYTES)
            .ok_or(ViewerTextureError::InvalidSize)?;
        if width == 0 || height == 0 || rgba.len() != byte_count {
            return Err(ViewerTextureError::InvalidPixels);
        }

        let mut bgra = Vec::with_capacity(byte_count);
        for pixel in rgba.chunks_exact(4) {
            let alpha = u16::from(pixel[3]);
            let premultiply = |channel: u8| ((u16::from(channel) * alpha + 127) / 255) as u8;
            bgra.extend_from_slice(&[
                premultiply(pixel[2]),
                premultiply(pixel[1]),
                premultiply(pixel[0]),
                pixel[3],
            ]);
        }

        let descriptor =
            FrameDescriptor::software(width, height, FramePixelFormat::Bgra8, timestamp)
                .map_err(|_| ViewerTextureError::InvalidSize)?;
        let frame = FrameLease::from_software(descriptor, bgra)
            .map_err(|_| ViewerTextureError::InvalidPixels)?;
        let mut state = lock(&self.state);
        if state.generation.is_some_and(|current| generation < current) {
            return Err(ViewerTextureError::StaleGeneration);
        }
        state.generation = Some(generation);
        state.latest = Some(frame);
        Ok(())
    }

    /// Acquires the newest frame if Flutter still has room to retain it.
    pub fn acquire_latest(&self) -> Option<ViewerFrameLease> {
        let mut state = lock(&self.state);
        if state.in_flight == VIEWER_MAX_IN_FLIGHT {
            return None;
        }
        let frame = state.latest.as_ref()?.shared_software_copy()?;
        state.in_flight += 1;
        Some(ViewerFrameLease {
            frame,
            state: Arc::clone(&self.state),
        })
    }
}

/// A frame held until Flutter signals that its pixel buffer can be released.
pub struct ViewerFrameLease {
    frame: FrameLease,
    state: Arc<Mutex<ViewerFrameState>>,
}

impl ViewerFrameLease {
    pub const fn descriptor(&self) -> &FrameDescriptor {
        self.frame.descriptor()
    }

    pub fn pixels(&self) -> &[u8] {
        self.frame
            .software_bytes()
            .expect("viewer texture frames own software pixels")
    }
}

impl Drop for ViewerFrameLease {
    fn drop(&mut self) {
        let mut state = lock(&self.state);
        state.in_flight = state.in_flight.saturating_sub(1);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewerTextureError {
    InvalidSize,
    InvalidPixels,
    StaleGeneration,
}

impl fmt::Display for ViewerTextureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidSize => "viewer frame size is invalid or exceeds the limit",
            Self::InvalidPixels => "viewer frame pixels do not match the declared size",
            Self::StaleGeneration => "viewer frame belongs to an obsolete generation",
        })
    }
}

impl Error for ViewerTextureError {}

/// Independent limits for one runtime workload class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BudgetLimits {
    max_in_flight: usize,
    max_bytes: u64,
}

impl BudgetLimits {
    pub const fn new(max_in_flight: usize, max_bytes: u64) -> Result<Self, BudgetConfigError> {
        if max_in_flight == 0 {
            return Err(BudgetConfigError::ZeroInFlight);
        }
        if max_bytes == 0 {
            return Err(BudgetConfigError::ZeroBytes);
        }
        Ok(Self {
            max_in_flight,
            max_bytes,
        })
    }

    pub const fn max_in_flight(self) -> usize {
        self.max_in_flight
    }

    pub const fn max_bytes(self) -> u64 {
        self.max_bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BudgetConfigError {
    ZeroInFlight,
    ZeroBytes,
}

impl fmt::Display for BudgetConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ZeroInFlight => "runtime budget max_in_flight must be positive",
            Self::ZeroBytes => "runtime budget max_bytes must be positive",
        })
    }
}

impl Error for BudgetConfigError {}

#[derive(Clone, Debug)]
pub struct ResourceBudget {
    limits: BudgetLimits,
    state: Arc<Mutex<BudgetState>>,
}

#[derive(Debug, Default)]
struct BudgetState {
    in_flight: usize,
    bytes: u64,
}

impl ResourceBudget {
    pub fn new(limits: BudgetLimits) -> Self {
        Self {
            limits,
            state: Arc::new(Mutex::new(BudgetState::default())),
        }
    }

    pub const fn limits(&self) -> BudgetLimits {
        self.limits
    }

    pub fn in_flight(&self) -> usize {
        lock(&self.state).in_flight
    }

    pub fn bytes_in_use(&self) -> u64 {
        lock(&self.state).bytes
    }

    pub fn try_acquire(&self, bytes: u64) -> Result<BudgetLease, BudgetAcquireError> {
        let mut state = lock(&self.state);
        if state.in_flight >= self.limits.max_in_flight {
            return Err(BudgetAcquireError::InFlightLimit {
                limit: self.limits.max_in_flight,
            });
        }
        if bytes > self.limits.max_bytes.saturating_sub(state.bytes) {
            return Err(BudgetAcquireError::ByteLimit {
                limit: self.limits.max_bytes,
                requested_bytes: bytes,
            });
        }
        let projected = state.bytes + bytes;
        state.in_flight += 1;
        state.bytes = projected;
        Ok(BudgetLease {
            budget: Some(self.clone()),
            bytes,
        })
    }

    fn release(&self, bytes: u64) {
        let mut state = lock(&self.state);
        state.in_flight = state.in_flight.saturating_sub(1);
        state.bytes = state.bytes.saturating_sub(bytes);
    }
}

#[derive(Debug)]
pub struct BudgetLease {
    budget: Option<ResourceBudget>,
    bytes: u64,
}

impl BudgetLease {
    pub fn release(mut self) {
        self.release_inner();
    }

    fn release_inner(&mut self) {
        if let Some(budget) = self.budget.take() {
            budget.release(self.bytes);
        }
    }
}

impl Drop for BudgetLease {
    fn drop(&mut self) {
        self.release_inner();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BudgetAcquireError {
    InFlightLimit { limit: usize },
    ByteLimit { limit: u64, requested_bytes: u64 },
}

impl fmt::Display for BudgetAcquireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InFlightLimit { limit } => {
                write!(
                    formatter,
                    "runtime in-flight budget exceeded (limit {limit})"
                )
            }
            Self::ByteLimit {
                limit,
                requested_bytes,
            } => write!(
                formatter,
                "runtime byte budget exceeded (limit {limit}, request {requested_bytes})"
            ),
        }
    }
}

impl Error for BudgetAcquireError {}

/// Separate budgets prevent decode, audio, and render work from consuming one
/// another's bounded runtime resources.
#[derive(Clone, Debug)]
pub struct RuntimeBudgets {
    render: ResourceBudget,
    audio: ResourceBudget,
    decode: ResourceBudget,
}

impl RuntimeBudgets {
    pub fn new(render: BudgetLimits, audio: BudgetLimits, decode: BudgetLimits) -> Self {
        Self {
            render: ResourceBudget::new(render),
            audio: ResourceBudget::new(audio),
            decode: ResourceBudget::new(decode),
        }
    }

    pub const fn render(&self) -> &ResourceBudget {
        &self.render
    }

    pub const fn audio(&self) -> &ResourceBudget {
        &self.audio
    }

    pub const fn decode(&self) -> &ResourceBudget {
        &self.decode
    }
}

/// A platform-neutral runtime workload. Adapters advertise these capabilities
/// without exposing device handles or scattering platform checks into domain
/// evaluation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RuntimeOperation {
    VideoDecode,
    AudioDecode,
    Render,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimePath {
    Software,
    Hardware,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RuntimeProviderId(String);

impl RuntimeProviderId {
    pub fn new(value: impl Into<String>) -> Result<Self, ProviderIdError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ProviderIdError::Empty);
        }
        if value.len() > MAX_PROVIDER_ID_BYTES {
            return Err(ProviderIdError::TooLong);
        }
        if !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        }) {
            return Err(ProviderIdError::InvalidCharacter);
        }
        Ok(Self(value))
    }

    pub fn software() -> Self {
        Self(SOFTWARE_PROVIDER_ID.to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RuntimeProviderId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderIdError {
    Empty,
    TooLong,
    InvalidCharacter,
}

impl fmt::Display for ProviderIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "runtime provider ID must not be empty",
            Self::TooLong => "runtime provider ID exceeds the configured byte limit",
            Self::InvalidCharacter => {
                "runtime provider ID must contain lowercase ASCII, digits, '.', '_' or '-'"
            }
        })
    }
}

impl Error for ProviderIdError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeCapability {
    operation: RuntimeOperation,
    provider: RuntimeProviderId,
    path: RuntimePath,
    available: bool,
    stable: bool,
    priority: u16,
}

impl RuntimeCapability {
    pub fn new(
        operation: RuntimeOperation,
        provider: RuntimeProviderId,
        path: RuntimePath,
        available: bool,
        stable: bool,
        priority: u16,
    ) -> Self {
        Self {
            operation,
            provider,
            path,
            available,
            stable,
            priority,
        }
    }

    pub fn software(operation: RuntimeOperation) -> Self {
        Self::new(
            operation,
            RuntimeProviderId::software(),
            RuntimePath::Software,
            true,
            true,
            0,
        )
    }

    pub fn hardware(
        operation: RuntimeOperation,
        provider: RuntimeProviderId,
        available: bool,
        stable: bool,
        priority: u16,
    ) -> Self {
        Self::new(
            operation,
            provider,
            RuntimePath::Hardware,
            available,
            stable,
            priority,
        )
    }

    pub const fn operation(&self) -> RuntimeOperation {
        self.operation
    }

    pub fn provider(&self) -> &RuntimeProviderId {
        &self.provider
    }

    pub const fn path(&self) -> RuntimePath {
        self.path
    }

    pub const fn available(&self) -> bool {
        self.available
    }

    pub const fn stable(&self) -> bool {
        self.stable
    }

    pub const fn priority(&self) -> u16 {
        self.priority
    }

    fn usable(&self) -> bool {
        self.available && self.stable
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeSelectionPolicy {
    PreferHardware,
    SoftwareOnly,
    HardwareOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionReason {
    HardwarePreferred,
    SoftwareFallback,
    SoftwareOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderSelection {
    operation: RuntimeOperation,
    provider: RuntimeProviderId,
    path: RuntimePath,
    reason: SelectionReason,
}

impl ProviderSelection {
    pub const fn operation(&self) -> RuntimeOperation {
        self.operation
    }

    pub fn provider(&self) -> &RuntimeProviderId {
        &self.provider
    }

    pub const fn path(&self) -> RuntimePath {
        self.path
    }

    pub const fn reason(&self) -> SelectionReason {
        self.reason
    }

    pub fn used_software_fallback(&self) -> bool {
        self.reason == SelectionReason::SoftwareFallback
    }
}

/// The single selection point for runtime capabilities and providers.
#[derive(Clone, Debug)]
pub struct CapabilityRegistry {
    capabilities: Vec<RuntimeCapability>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            capabilities: vec![
                RuntimeCapability::software(RuntimeOperation::VideoDecode),
                RuntimeCapability::software(RuntimeOperation::AudioDecode),
                RuntimeCapability::software(RuntimeOperation::Render),
            ],
        }
    }

    pub fn empty() -> Self {
        Self {
            capabilities: Vec::new(),
        }
    }

    pub fn register(
        &mut self,
        capability: RuntimeCapability,
    ) -> Result<(), CapabilityRegistryError> {
        if self.capabilities.iter().any(|existing| {
            existing.operation == capability.operation
                && existing.path == capability.path
                && existing.provider == capability.provider
        }) {
            return Err(CapabilityRegistryError::Duplicate);
        }
        self.capabilities.push(capability);
        Ok(())
    }

    pub fn capabilities(&self) -> &[RuntimeCapability] {
        &self.capabilities
    }

    pub fn select(
        &self,
        operation: RuntimeOperation,
        policy: RuntimeSelectionPolicy,
    ) -> Result<ProviderSelection, CapabilitySelectionError> {
        let software = self.best(operation, RuntimePath::Software);
        let hardware = self.best(operation, RuntimePath::Hardware);
        let selection = match policy {
            RuntimeSelectionPolicy::SoftwareOnly => {
                software.map(|capability| (capability, SelectionReason::SoftwareOnly))
            }
            RuntimeSelectionPolicy::PreferHardware => hardware
                .map(|capability| (capability, SelectionReason::HardwarePreferred))
                .or_else(|| {
                    software.map(|capability| (capability, SelectionReason::SoftwareFallback))
                }),
            RuntimeSelectionPolicy::HardwareOnly => {
                hardware.map(|capability| (capability, SelectionReason::HardwarePreferred))
            }
        };

        selection
            .map(|(capability, reason)| ProviderSelection {
                operation,
                provider: capability.provider.clone(),
                path: capability.path,
                reason,
            })
            .ok_or(CapabilitySelectionError { operation, policy })
    }

    fn best(&self, operation: RuntimeOperation, path: RuntimePath) -> Option<&RuntimeCapability> {
        self.capabilities
            .iter()
            .filter(|capability| {
                capability.operation == operation && capability.path == path && capability.usable()
            })
            .min_by_key(|capability| {
                (
                    std::cmp::Reverse(capability.priority),
                    capability.provider.as_str(),
                )
            })
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityRegistryError {
    Duplicate,
}

impl fmt::Display for CapabilityRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("runtime capability provider is already registered")
    }
}

impl Error for CapabilityRegistryError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilitySelectionError {
    pub operation: RuntimeOperation,
    pub policy: RuntimeSelectionPolicy,
}

impl fmt::Display for CapabilitySelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "no usable {:?} provider satisfies {:?}",
            self.operation, self.policy
        )
    }
}

impl Error for CapabilitySelectionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{Barrier, atomic::AtomicUsize},
        thread,
    };

    fn time(numerator: i64, denominator: u32) -> RationalTime {
        RationalTime::new(numerator, denominator).unwrap()
    }

    #[test]
    fn viewer_mailbox_converts_pixels_rejects_stale_frames_and_bounds_leases() {
        let adapter = ViewerTextureAdapter::default();
        assert!(adapter.advance_generation(4));
        assert_eq!(
            adapter.publish_rgba(3, 1, 1, time(1, 24), &[200, 100, 50, 128]),
            Err(ViewerTextureError::StaleGeneration)
        );
        adapter
            .publish_rgba(4, 1, 1, time(1, 24), &[200, 100, 50, 128])
            .unwrap();

        let first = adapter.acquire_latest().unwrap();
        assert_eq!(first.pixels(), &[25, 50, 100, 128]);
        assert_eq!(first.descriptor().pixel_format(), FramePixelFormat::Bgra8);
        let second = adapter.acquire_latest().unwrap();
        let third = adapter.acquire_latest().unwrap();
        assert!(adapter.acquire_latest().is_none());

        assert!(adapter.advance_generation(5));
        assert!(adapter.acquire_latest().is_none());
        drop(first);
        adapter
            .publish_rgba(5, 1, 1, time(2, 24), &[0, 0, 0, 255])
            .unwrap();
        assert!(adapter.acquire_latest().is_some());
        drop((second, third));
    }

    fn provider(value: &str) -> RuntimeProviderId {
        RuntimeProviderId::new(value).unwrap()
    }

    #[test]
    fn snapshots_preserve_exact_range_and_revision_without_project_mutation() {
        let project = ProjectDocument::new("snapshot");
        let before = project.revision();
        let range = TimeRange::new(time(1, 24), time(1001, 24000)).unwrap();
        let snapshot = RenderSnapshot::from_project(&project, range);

        assert_eq!(snapshot.project_id(), project.id());
        assert_eq!(snapshot.project_revision(), before);
        assert_eq!(snapshot.requested_range(), range);
        assert!(snapshot.matches(project.id(), before));
        assert_eq!(project.revision(), before);

        let point = RenderSnapshot::at_time(&project, time(30000, 30000)).unwrap();
        assert_eq!(point.requested_range().start(), time(1, 1));
        assert_eq!(point.requested_range().duration(), RationalTime::ZERO);
    }

    #[test]
    fn frame_descriptor_is_exact_and_lease_releases_once() {
        let timing = FrameTiming::new(time(1001, 24000), Some(time(1, 24))).unwrap();
        let color = FrameColorInfo::new(FrameColorSpace::Bt709, FrameColorRange::Limited);
        let descriptor = FrameDescriptor::new(
            FrameMemoryDomain::HardwareSurface,
            1920,
            1080,
            FramePixelFormat::Nv12,
            color,
            timing,
            FrameAccess::ReadOnly,
        )
        .unwrap();
        let releases = Arc::new(AtomicUsize::new(0));
        let released = Arc::clone(&releases);
        let lease = FrameLease::with_release(descriptor, move || {
            released.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(lease.descriptor().timing(), timing);
        assert_eq!(lease.descriptor().color(), color);
        lease.release();
        assert_eq!(releases.load(Ordering::SeqCst), 1);

        let released = Arc::clone(&releases);
        {
            let _lease = FrameLease::with_release(descriptor, move || {
                released.fetch_add(1, Ordering::SeqCst);
            });
        }
        assert_eq!(releases.load(Ordering::SeqCst), 2);

        let software =
            FrameDescriptor::software(2, 2, FramePixelFormat::Rgba8, time(0, 1)).unwrap();
        let lease = FrameLease::from_software(software, vec![1, 2, 3, 4]).unwrap();
        assert_eq!(lease.software_bytes(), Some(&[1, 2, 3, 4][..]));
        assert!(FrameLease::from_software(descriptor, Vec::new()).is_err());
    }

    #[test]
    fn bounded_queue_reports_backpressure_and_cancellation() {
        let queue = BoundedQueue::new(1).unwrap();
        queue.try_push(1).unwrap();
        assert!(matches!(
            queue.try_push(2),
            Err(QueueSendError::Backpressure(2))
        ));
        assert_eq!(queue.try_pop().unwrap(), Some(1));

        let cancellation = CancellationToken::new();
        cancellation.cancel();
        assert!(matches!(
            queue.push(3, &cancellation),
            Err(QueueSendError::Cancelled(3))
        ));

        let queue = Arc::new(BoundedQueue::<i32>::new(1).unwrap());
        queue.try_push(5).unwrap();
        let started = Arc::new(Barrier::new(2));
        let producer_queue = Arc::clone(&queue);
        let producer_cancellation = CancellationToken::new();
        let producer_token = producer_cancellation.clone();
        let producer_started = Arc::clone(&started);
        let producer = thread::spawn(move || {
            producer_started.wait();
            producer_queue.push(6, &producer_token)
        });
        started.wait();
        producer_cancellation.cancel();
        assert!(matches!(
            producer.join().unwrap(),
            Err(QueueSendError::Cancelled(6))
        ));
        assert_eq!(queue.len(), 1);

        let queue = BoundedQueue::<i32>::new(1).unwrap();
        queue.close();
        assert_eq!(queue.try_pop(), Err(QueueReceiveError::Closed));
        assert!(matches!(queue.try_push(4), Err(QueueSendError::Closed(4))));
    }

    #[test]
    fn queue_waiters_unblock_on_close() {
        let queue = Arc::new(BoundedQueue::<i32>::new(1).unwrap());
        let cancellation = CancellationToken::new();
        let consumer_queue = Arc::clone(&queue);
        let consumer_cancellation = cancellation.clone();
        let consumer = thread::spawn(move || consumer_queue.pop(&consumer_cancellation));
        queue.close();
        assert_eq!(consumer.join().unwrap(), Err(QueueReceiveError::Closed));
    }

    #[test]
    fn separate_runtime_budgets_enforce_bytes_and_release() {
        let limits = BudgetLimits::new(1, 10).unwrap();
        let budgets = RuntimeBudgets::new(limits, limits, limits);
        let lease = budgets.decode().try_acquire(6).unwrap();
        assert_eq!(budgets.decode().in_flight(), 1);
        assert_eq!(budgets.decode().bytes_in_use(), 6);
        assert!(matches!(
            budgets.decode().try_acquire(1),
            Err(BudgetAcquireError::InFlightLimit { limit: 1 })
        ));
        drop(lease);
        assert_eq!(budgets.decode().in_flight(), 0);
        assert_eq!(budgets.decode().bytes_in_use(), 0);
        let _audio = budgets.audio().try_acquire(10).unwrap();
        assert!(matches!(
            budgets.decode().try_acquire(11),
            Err(BudgetAcquireError::ByteLimit {
                limit: 10,
                requested_bytes: 11,
            })
        ));
    }

    #[test]
    fn capability_registry_prefers_stable_hardware_and_has_software_fallback() {
        let mut registry = CapabilityRegistry::new();
        registry
            .register(RuntimeCapability::hardware(
                RuntimeOperation::VideoDecode,
                provider("hardware.fast"),
                true,
                false,
                100,
            ))
            .unwrap();
        registry
            .register(RuntimeCapability::hardware(
                RuntimeOperation::VideoDecode,
                provider("hardware.stable"),
                true,
                true,
                10,
            ))
            .unwrap();

        let selected = registry
            .select(
                RuntimeOperation::VideoDecode,
                RuntimeSelectionPolicy::PreferHardware,
            )
            .unwrap();
        assert_eq!(selected.path(), RuntimePath::Hardware);
        assert_eq!(selected.provider().as_str(), "hardware.stable");
        assert_eq!(selected.reason(), SelectionReason::HardwarePreferred);

        let fallback = CapabilityRegistry::new()
            .select(
                RuntimeOperation::Render,
                RuntimeSelectionPolicy::PreferHardware,
            )
            .unwrap();
        assert_eq!(fallback.path(), RuntimePath::Software);
        assert!(fallback.used_software_fallback());
    }

    #[test]
    fn hardware_only_selection_fails_without_stable_hardware() {
        let registry = CapabilityRegistry::new();
        assert_eq!(
            registry.select(
                RuntimeOperation::AudioDecode,
                RuntimeSelectionPolicy::HardwareOnly,
            ),
            Err(CapabilitySelectionError {
                operation: RuntimeOperation::AudioDecode,
                policy: RuntimeSelectionPolicy::HardwareOnly,
            })
        );
    }
}
