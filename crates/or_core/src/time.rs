use serde::{Deserialize, Deserializer, Serialize};
use std::{cmp::Ordering, error::Error, fmt};

/// An exact amount of seconds stored as a normalized rational number.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct RationalTime {
    numerator: i64,
    denominator: u32,
}

impl RationalTime {
    pub const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };

    /// Constructs and normalizes an exact number of seconds.
    pub fn new(numerator: i64, denominator: u32) -> Result<Self, TimeError> {
        if denominator == 0 {
            return Err(TimeError::ZeroDenominator);
        }

        Self::from_wide(i128::from(numerator), u128::from(denominator))
    }

    /// Converts an integer count of units to seconds without rounding.
    pub fn from_units(units: i64, rate: RationalRate) -> Result<Self, TimeError> {
        let numerator = i128::from(units) * i128::from(rate.denominator);
        Self::from_wide(numerator, u128::from(rate.numerator))
    }

    pub const fn zero() -> Self {
        Self::ZERO
    }

    pub const fn numerator(self) -> i64 {
        self.numerator
    }

    pub const fn denominator(self) -> u32 {
        self.denominator
    }

    pub const fn is_zero(self) -> bool {
        self.numerator == 0
    }

    pub const fn is_negative(self) -> bool {
        self.numerator < 0
    }

    pub const fn is_positive(self) -> bool {
        self.numerator > 0
    }

    pub fn checked_add(self, other: Self) -> Result<Self, TimeError> {
        self.checked_add_signed(other, 1)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, TimeError> {
        self.checked_add_signed(other, -1)
    }

    fn checked_add_signed(self, other: Self, sign: i128) -> Result<Self, TimeError> {
        let self_denominator = u128::from(self.denominator);
        let other_denominator = u128::from(other.denominator);
        let common = gcd(self_denominator, other_denominator);
        let self_scale = other_denominator / common;
        let other_scale = self_denominator / common;

        let numerator = i128::from(self.numerator) * self_scale as i128
            + i128::from(other.numerator) * other_scale as i128 * sign;
        let denominator = self_denominator * self_scale;

        Self::from_wide(numerator, denominator)
    }

    fn from_wide(numerator: i128, denominator: u128) -> Result<Self, TimeError> {
        if denominator == 0 {
            return Err(TimeError::ZeroDenominator);
        }

        let divisor = gcd(numerator.unsigned_abs(), denominator);
        let divisor = i128::try_from(divisor).map_err(|_| TimeError::ArithmeticOverflow)?;
        let numerator =
            i64::try_from(numerator / divisor).map_err(|_| TimeError::ArithmeticOverflow)?;
        let denominator = u32::try_from(denominator / divisor as u128)
            .map_err(|_| TimeError::ArithmeticOverflow)?;

        Ok(Self {
            numerator,
            denominator,
        })
    }
}

impl Default for RationalTime {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Ord for RationalTime {
    fn cmp(&self, other: &Self) -> Ordering {
        (i128::from(self.numerator) * i128::from(other.denominator))
            .cmp(&(i128::from(other.numerator) * i128::from(self.denominator)))
    }
}

impl PartialOrd for RationalTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'de> Deserialize<'de> for RationalTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Repr {
            numerator: i64,
            denominator: u32,
        }

        let value = Repr::deserialize(deserializer)?;
        Self::new(value.numerator, value.denominator).map_err(serde::de::Error::custom)
    }
}

/// An exact positive number of units per second.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct RationalRate {
    numerator: u32,
    denominator: u32,
}

impl RationalRate {
    /// Constructs and normalizes an exact rate.
    pub fn new(numerator: u32, denominator: u32) -> Result<Self, TimeError> {
        if numerator == 0 {
            return Err(TimeError::ZeroRateNumerator);
        }
        if denominator == 0 {
            return Err(TimeError::ZeroRateDenominator);
        }

        let divisor = gcd(u128::from(numerator), u128::from(denominator));
        Ok(Self {
            numerator: (u128::from(numerator) / divisor) as u32,
            denominator: (u128::from(denominator) / divisor) as u32,
        })
    }

    pub const fn numerator(self) -> u32 {
        self.numerator
    }

    pub const fn denominator(self) -> u32 {
        self.denominator
    }
}

impl<'de> Deserialize<'de> for RationalRate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Repr {
            numerator: u32,
            denominator: u32,
        }

        let value = Repr::deserialize(deserializer)?;
        Self::new(value.numerator, value.denominator).map_err(serde::de::Error::custom)
    }
}

/// A start position and nonnegative duration.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct TimeRange {
    start: RationalTime,
    duration: RationalTime,
}

impl TimeRange {
    pub fn new(start: RationalTime, duration: RationalTime) -> Result<Self, TimeError> {
        if duration.is_negative() {
            return Err(TimeError::NegativeDuration);
        }

        Ok(Self { start, duration })
    }

    pub const fn start(self) -> RationalTime {
        self.start
    }

    pub const fn duration(self) -> RationalTime {
        self.duration
    }
}

impl<'de> Deserialize<'de> for TimeRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Repr {
            start: RationalTime,
            duration: RationalTime,
        }

        let value = Repr::deserialize(deserializer)?;
        Self::new(value.start, value.duration).map_err(serde::de::Error::custom)
    }
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeError {
    ZeroDenominator,
    ZeroRateNumerator,
    ZeroRateDenominator,
    ArithmeticOverflow,
    NegativeDuration,
}

impl fmt::Display for TimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ZeroDenominator => "rational denominator must be positive",
            Self::ZeroRateNumerator => "rate numerator must be positive",
            Self::ZeroRateDenominator => "rate denominator must be positive",
            Self::ArithmeticOverflow => "rational time result is outside the supported range",
            Self::NegativeDuration => "time range duration must be nonnegative",
        })
    }
}

impl Error for TimeError {}

#[cfg(test)]
mod tests {
    use super::{RationalRate, RationalTime, TimeError, TimeRange};
    use std::cmp::Ordering;

    fn time(numerator: i64, denominator: u32) -> RationalTime {
        RationalTime::new(numerator, denominator).unwrap()
    }

    fn rate(numerator: u32, denominator: u32) -> RationalRate {
        RationalRate::new(numerator, denominator).unwrap()
    }

    #[test]
    fn rational_values_normalize_and_reject_zero_denominators() {
        assert_eq!(time(1, 2), time(2, 4));
        assert_eq!(time(-2, 4), time(-1, 2));
        assert_eq!(time(0, 123), time(0, 1));
        assert_eq!(time(0, 123).numerator(), 0);
        assert_eq!(time(0, 123).denominator(), 1);
        assert_eq!(RationalTime::new(1, 0), Err(TimeError::ZeroDenominator));
        assert_eq!(RationalRate::new(0, 1), Err(TimeError::ZeroRateNumerator));
        assert_eq!(RationalRate::new(1, 0), Err(TimeError::ZeroRateDenominator));
        assert_eq!(rate(2, 4), rate(1, 2));
    }

    #[test]
    fn video_frame_rates_convert_exactly_to_seconds() {
        assert_eq!(
            RationalTime::from_units(1, rate(24, 1)).unwrap(),
            time(1, 24)
        );
        assert_eq!(
            RationalTime::from_units(1, rate(24_000, 1_001)).unwrap(),
            time(1_001, 24_000)
        );
        assert_eq!(
            RationalTime::from_units(1, rate(30_000, 1_001)).unwrap(),
            time(1_001, 30_000)
        );
        assert_eq!(
            RationalTime::from_units(30_000, rate(30_000, 1_001)).unwrap(),
            time(1_001, 1)
        );
    }

    #[test]
    fn audio_sample_rates_convert_exactly_to_seconds() {
        assert_eq!(
            RationalTime::from_units(48_000, rate(48_000, 1)).unwrap(),
            time(1, 1)
        );
        assert_eq!(
            RationalTime::from_units(24_000, rate(48_000, 1)).unwrap(),
            time(1, 2)
        );
    }

    #[test]
    fn addition_and_subtraction_are_exact_and_checked() {
        assert_eq!(time(1, 2).checked_add(time(1, 3)).unwrap(), time(5, 6));
        assert_eq!(time(1, 1).checked_sub(time(1, 3)).unwrap(), time(2, 3));
        assert_eq!(
            time(i64::MAX, 1).checked_add(time(1, 1)),
            Err(TimeError::ArithmeticOverflow)
        );
    }

    #[test]
    fn ordering_compares_exact_rational_values() {
        assert_eq!(time(1, 3).cmp(&time(1, 2)), Ordering::Less);
        assert_eq!(time(1_001, 30_000).cmp(&time(1, 24)), Ordering::Less);
        assert_eq!(time(-1, 3).cmp(&RationalTime::ZERO), Ordering::Less);
    }

    #[test]
    fn time_range_accepts_nonnegative_duration_and_signed_start() {
        let zero = TimeRange::new(time(-1, 2), RationalTime::ZERO).unwrap();
        assert_eq!(zero.start(), time(-1, 2));
        assert_eq!(zero.duration(), RationalTime::ZERO);

        let positive = TimeRange::new(time(1, 3), time(2, 3)).unwrap();
        assert_eq!(positive.duration(), time(2, 3));
        assert_eq!(
            TimeRange::new(time(0, 1), time(-1, 3)),
            Err(TimeError::NegativeDuration)
        );
    }

    #[test]
    fn serde_rejects_invalid_rationals_and_normalizes_valid_ones() {
        let serialized_time = serde_json::to_string(&time(2, 4)).unwrap();
        assert_eq!(serialized_time, r#"{"numerator":1,"denominator":2}"#);
        assert_eq!(
            serde_json::from_str::<RationalTime>(&serialized_time).unwrap(),
            time(1, 2)
        );

        assert!(
            serde_json::from_str::<RationalTime>(r#"{"numerator":1,"denominator":0}"#).is_err()
        );
        assert_eq!(
            serde_json::from_str::<RationalTime>(r#"{"numerator":2,"denominator":4}"#).unwrap(),
            time(1, 2)
        );

        assert!(
            serde_json::from_str::<RationalRate>(r#"{"numerator":0,"denominator":1}"#).is_err()
        );
        assert!(
            serde_json::from_str::<RationalRate>(r#"{"numerator":1,"denominator":0}"#).is_err()
        );
        assert_eq!(
            serde_json::from_str::<RationalRate>(r#"{"numerator":48000,"denominator":48000}"#)
                .unwrap(),
            rate(1, 1)
        );
        let range = TimeRange::new(time(-1, 3), time(2, 3)).unwrap();
        let serialized_range = serde_json::to_string(&range).unwrap();
        assert_eq!(
            serde_json::from_str::<TimeRange>(&serialized_range).unwrap(),
            range
        );
        assert!(serde_json::from_str::<TimeRange>(
            r#"{"start":{"numerator":0,"denominator":1},"duration":{"numerator":-1,"denominator":1}}"#
        )
        .is_err());
    }
}
