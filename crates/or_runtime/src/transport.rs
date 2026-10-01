use or_core::{RationalRate, RationalTime, TimeError};
use std::{error::Error, fmt, time::Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlaybackSnapshot {
    pub position: RationalTime,
    pub presented_time: Option<RationalTime>,
    pub frame_rate: Option<RationalRate>,
    pub content_end: Option<RationalTime>,
    pub playing: bool,
    pub generation: u64,
    pub frame_sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreviewFrameRequest {
    pub time: RationalTime,
    pub frame_index: Option<u64>,
    pub generation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewFrameStep {
    Previous,
    Next,
}

#[derive(Debug, Eq, PartialEq)]
pub enum PreviewTransportError {
    NegativeTime,
    FrameRateUnavailable,
    GenerationOverflow,
    ClockOverflow,
    Time(TimeError),
}

impl fmt::Display for PreviewTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeTime => formatter.write_str("preview time must be nonnegative"),
            Self::FrameRateUnavailable => {
                formatter.write_str("set a sequence frame rate before playing or stepping")
            }
            Self::GenerationOverflow => formatter.write_str("preview generation overflowed"),
            Self::ClockOverflow => formatter.write_str("preview clock exceeded exact time bounds"),
            Self::Time(error) => error.fmt(formatter),
        }
    }
}

impl Error for PreviewTransportError {}

impl From<TimeError> for PreviewTransportError {
    fn from(error: TimeError) -> Self {
        Self::Time(error)
    }
}

struct PlaybackAnchor {
    instant: Instant,
    time: RationalTime,
}

/// Runtime-only exact seek and sequence-lattice playback state.
pub struct PreviewTransport {
    state: PlaybackSnapshot,
    anchor: Option<PlaybackAnchor>,
    requested_frame: Option<u64>,
}

impl PreviewTransport {
    pub fn new(frame_rate: Option<RationalRate>, content_end: Option<RationalTime>) -> Self {
        Self {
            state: PlaybackSnapshot {
                position: RationalTime::ZERO,
                presented_time: None,
                frame_rate,
                content_end,
                playing: false,
                generation: 0,
                frame_sequence: 0,
            },
            anchor: None,
            requested_frame: None,
        }
    }

    /// Replaces the timeline timing and invalidates output for the old snapshot.
    pub fn configure(
        &mut self,
        frame_rate: Option<RationalRate>,
        content_end: Option<RationalTime>,
    ) -> Result<PlaybackSnapshot, PreviewTransportError> {
        let generation = self.next_generation()?;
        self.state = PlaybackSnapshot {
            position: RationalTime::ZERO,
            presented_time: None,
            frame_rate,
            content_end,
            playing: false,
            generation,
            frame_sequence: 0,
        };
        self.anchor = None;
        self.requested_frame = None;
        Ok(self.state)
    }

    pub const fn snapshot(&self) -> PlaybackSnapshot {
        self.state
    }

    /// Preserves the requested rational time and does not snap to a frame.
    pub fn seek(&mut self, time: RationalTime) -> Result<PlaybackSnapshot, PreviewTransportError> {
        if time.is_negative() {
            return Err(PreviewTransportError::NegativeTime);
        }
        let generation = self.next_generation()?;
        self.state.position = time;
        self.state.presented_time = None;
        self.state.playing = false;
        self.state.generation = generation;
        self.anchor = None;
        self.requested_frame = None;
        Ok(self.state)
    }

    pub fn step(
        &mut self,
        direction: PreviewFrameStep,
    ) -> Result<Option<PreviewFrameRequest>, PreviewTransportError> {
        let rate = self
            .state
            .frame_rate
            .ok_or(PreviewTransportError::FrameRateUnavailable)?;
        let Some(content_end) = self.state.content_end else {
            return Ok(None);
        };
        let frame_index = match direction {
            PreviewFrameStep::Next => {
                if self.state.position >= content_end {
                    return Ok(None);
                }
                let index = rate
                    .frame_index_floor(self.state.position)?
                    .checked_add(1)
                    .ok_or(TimeError::ArithmeticOverflow)?;
                let time = rate.frame_time(index)?;
                if time >= content_end {
                    return Ok(None);
                }
                index
            }
            PreviewFrameStep::Previous => {
                let Some(last_index) = rate.frame_index_ceil(content_end)?.checked_sub(1) else {
                    return Ok(None);
                };
                rate.frame_index_ceil(self.state.position)?
                    .saturating_sub(1)
                    .min(last_index)
            }
        };
        let time = rate.frame_time(frame_index)?;
        let generation = self.next_generation()?;
        self.state.position = time;
        self.state.presented_time = None;
        self.state.playing = false;
        self.state.generation = generation;
        self.anchor = None;
        self.requested_frame = Some(frame_index);
        Ok(Some(PreviewFrameRequest {
            time,
            frame_index: Some(frame_index),
            generation,
        }))
    }

    /// Starts monotonic-clock playback. An unset rate remains a typed error.
    pub fn play(&mut self, now: Instant) -> Result<PlaybackSnapshot, PreviewTransportError> {
        if self.state.frame_rate.is_none() {
            return Err(PreviewTransportError::FrameRateUnavailable);
        }
        if self
            .state
            .content_end
            .is_none_or(|content_end| self.state.position >= content_end)
        {
            self.state.playing = false;
            self.anchor = None;
            return Ok(self.state);
        }
        self.state.generation = self.next_generation()?;
        self.state.playing = true;
        self.anchor = Some(PlaybackAnchor {
            instant: now,
            time: self.state.position,
        });
        self.requested_frame = None;
        Ok(self.state)
    }

    /// Stops playback at its exact monotonic-clock position.
    pub fn pause(&mut self, now: Instant) -> Result<PlaybackSnapshot, PreviewTransportError> {
        if !self.state.playing {
            return Ok(self.state);
        }
        let position = self.clock_position(now)?;
        self.state.position = self
            .state
            .content_end
            .filter(|end| position >= *end)
            .unwrap_or(position);
        self.state.playing = false;
        self.anchor = None;
        Ok(self.state)
    }

    /// Returns at most one newly due output-lattice frame; callers may drop late work.
    pub fn tick(
        &mut self,
        now: Instant,
    ) -> Result<Option<PreviewFrameRequest>, PreviewTransportError> {
        if !self.state.playing {
            return Ok(None);
        }
        let rate = self
            .state
            .frame_rate
            .ok_or(PreviewTransportError::FrameRateUnavailable)?;
        let position = self.clock_position(now)?;
        let at_end = self.state.content_end.is_some_and(|end| position >= end);
        let content_end = self.state.content_end;
        let frame_index = if at_end {
            let Some(end) = content_end else {
                self.state.playing = false;
                self.anchor = None;
                return Ok(None);
            };
            self.state.position = end;
            self.state.playing = false;
            self.anchor = None;
            let Some(last) = rate.frame_index_ceil(end)?.checked_sub(1) else {
                return Ok(None);
            };
            last
        } else {
            self.state.position = position;
            rate.frame_index_floor(position)?
        };
        if self.requested_frame == Some(frame_index) {
            return Ok(None);
        }
        let time = rate.frame_time(frame_index)?;
        if content_end.is_some_and(|end| time >= end) {
            return Ok(None);
        }
        self.requested_frame = Some(frame_index);
        Ok(Some(PreviewFrameRequest {
            time,
            frame_index: Some(frame_index),
            generation: self.state.generation,
        }))
    }

    pub fn record_presented(&mut self, generation: u64, time: RationalTime) -> bool {
        if generation != self.state.generation {
            return false;
        }
        self.state.presented_time = Some(time);
        self.state.frame_sequence = self.state.frame_sequence.saturating_add(1);
        true
    }

    fn clock_position(&self, now: Instant) -> Result<RationalTime, PreviewTransportError> {
        let Some(anchor) = &self.anchor else {
            return Ok(self.state.position);
        };
        let nanoseconds = i64::try_from(now.saturating_duration_since(anchor.instant).as_nanos())
            .map_err(|_| PreviewTransportError::ClockOverflow)?;
        let elapsed = RationalTime::new(nanoseconds, 1_000_000_000)?;
        Ok(anchor.time.checked_add(elapsed)?)
    }

    fn next_generation(&self) -> Result<u64, PreviewTransportError> {
        self.state
            .generation
            .checked_add(1)
            .ok_or(PreviewTransportError::GenerationOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(numerator: i64, denominator: u32) -> RationalTime {
        RationalTime::new(numerator, denominator).unwrap()
    }

    fn rate() -> RationalRate {
        RationalRate::new(24, 1).unwrap()
    }

    #[test]
    fn seek_preserves_exact_time_without_snapping() {
        let mut transport = PreviewTransport::new(Some(rate()), Some(time(2, 1)));
        let requested = time(1001, 1000);

        let result = transport.seek(requested).unwrap();

        assert_eq!(result.position, requested);
        assert!(!result.playing);
        assert_eq!(result.presented_time, None);
    }

    #[test]
    fn frame_step_uses_the_global_lattice_and_half_open_content_end() {
        let mut transport = PreviewTransport::new(Some(rate()), Some(time(1, 1)));
        transport.seek(time(1, 24)).unwrap();
        let next = transport.step(PreviewFrameStep::Next).unwrap().unwrap();
        assert_eq!(next.time, time(2, 24));
        transport.seek(time(1, 1)).unwrap();
        let previous = transport.step(PreviewFrameStep::Previous).unwrap().unwrap();
        assert_eq!(previous.time, time(23, 24));
        assert!(transport.step(PreviewFrameStep::Next).unwrap().is_none());
    }

    #[test]
    fn previous_at_zero_clamps_to_first_frame_and_empty_content_has_no_steps() {
        let mut transport = PreviewTransport::new(Some(rate()), Some(time(1, 1)));
        assert_eq!(
            transport
                .step(PreviewFrameStep::Previous)
                .unwrap()
                .unwrap()
                .time,
            RationalTime::ZERO
        );
        let mut empty = PreviewTransport::new(Some(rate()), None);
        assert!(empty.step(PreviewFrameStep::Next).unwrap().is_none());
    }

    #[test]
    fn play_requires_rate_and_ticks_exact_lattice_frames_from_monotonic_time() {
        let start = Instant::now();
        let mut transport = PreviewTransport::new(None, Some(time(2, 1)));
        assert_eq!(
            transport.play(start),
            Err(PreviewTransportError::FrameRateUnavailable)
        );

        let mut transport = PreviewTransport::new(Some(rate()), Some(time(2, 1)));
        transport.play(start).unwrap();
        assert_eq!(
            transport.tick(start).unwrap().unwrap().time,
            RationalTime::ZERO
        );
        assert!(
            transport
                .tick(start + std::time::Duration::from_millis(10))
                .unwrap()
                .is_none()
        );
        assert_eq!(
            transport
                .tick(start + std::time::Duration::from_millis(50))
                .unwrap()
                .unwrap()
                .time,
            time(1, 24)
        );
    }

    #[test]
    fn frame_completion_rejects_stale_generation() {
        let mut transport = PreviewTransport::new(Some(rate()), Some(time(2, 1)));
        let old = transport.snapshot().generation;
        transport.seek(time(1, 3)).unwrap();
        assert!(!transport.record_presented(old, time(0, 1)));
        assert!(transport.snapshot().presented_time.is_none());
    }
}
