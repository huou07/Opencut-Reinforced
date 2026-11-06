//! Which encoder an export may use, and why one was picked.
//!
//! This module is pure policy: it never probes FFmpeg, wgpu or the operating
//! system. A caller measures what the machine can actually do (`or_media`
//! opens the codecs, `or_render` enumerates the adapters) and hands the facts
//! here, so the same decision runs in a test, the app, the CLI and an export
//! job — and the choice is made once per job, never inside the frame loop.

use serde::{Deserialize, Serialize};

use crate::ExportQuality;

/// How much hardware acceleration an export should use.
///
/// The profile is a preference, not a switch: every profile still requires an
/// encoder that opened on this machine, and every profile records why it moved
/// to a different one. `Compatibility` is the only one that refuses hardware on
/// purpose, so a machine with a driver that misbehaves has a way out that does
/// not depend on the person knowing encoder names.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerformanceProfile {
    /// Use whatever this machine does best, with a measured fallback.
    #[default]
    Recommended,
    /// Spend the machine's encode hardware even when a safer path exists.
    /// The dialog starts on `Speed` so the choice is visible in the file too.
    MaximumPerformance,
    /// Prefer the operating system's own hardware encoder over vendor ones.
    /// Vendor encoders are faster but vary more between driver versions.
    Balanced,
    /// Software encoders only, for a driver that crashes or produces
    /// unplayable files.
    Compatibility,
}

impl PerformanceProfile {
    pub const ALL: [Self; 4] = [
        Self::Recommended,
        Self::MaximumPerformance,
        Self::Balanced,
        Self::Compatibility,
    ];

    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Recommended => "recommended",
            Self::MaximumPerformance => "maximum_performance",
            Self::Balanced => "balanced",
            Self::Compatibility => "compatibility",
        }
    }

    /// Parses the wire name the settings file and the export request carry.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|profile| profile.wire_name() == value.trim())
    }

    /// The quality an export dialog should start on for this profile.
    ///
    /// `MaximumPerformance` means "make the fastest file this machine can",
    /// so the dialog starts on `Speed`; the lossless profile ignores quality
    /// and the dialog hides the choice there anyway.
    pub const fn default_quality(self) -> ExportQuality {
        match self {
            Self::MaximumPerformance => ExportQuality::Speed,
            _ => ExportQuality::Quality,
        }
    }
}

/// One decoder the linked FFmpeg carries.
///
/// OR decodes in software today, so this answers "can this machine read the
/// format at all", not "is a hardware decoder available".
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecoderCandidate {
    pub name: String,
    pub present: bool,
}

/// One encoder the build carries, and whether this machine opened it.
///
/// `present` is compile-time (the linked FFmpeg has the encoder); `opens` is
/// the runtime probe the export itself performs, so a profile never selects an
/// encoder that would fail on its first frame.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EncoderCandidate {
    pub name: String,
    pub present: bool,
    pub opens: bool,
    /// True when the encoder reads GPU surfaces rather than host memory.
    pub hardware: bool,
    /// Why a present encoder did not open, phrased for a person.
    pub note: Option<String>,
}

/// The encoder an export job will open, and the reasons around it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExportEncodingPlan {
    pub profile: PerformanceProfile,
    /// The encoder name to open, when the build carries any usable H.264 one.
    pub h264_encoder: Option<String>,
    pub hardware: bool,
    pub software: bool,
    /// Set when an explicit request could not be honoured.
    pub fallback: Option<String>,
    /// What the person should know about this machine, in reading order.
    pub notes: Vec<String>,
}

impl ExportEncodingPlan {
    /// A one-line explanation for a status surface or an export receipt.
    pub fn summary(&self) -> String {
        match &self.h264_encoder {
            Some(encoder) if self.hardware => format!("{encoder} (hardware)"),
            Some(encoder) if self.software => format!("{encoder} (software)"),
            Some(encoder) => encoder.clone(),
            None => "no usable H.264 encoder".to_owned(),
        }
    }
}

/// Whether the operating system itself ships this encoder.
///
/// VideoToolbox, Media Foundation and VAAPI are the platform's own path;
/// NVENC, AMF and OpenH264 come from a vendor or a bundled library and can
/// change behaviour with a driver update, which is the trade `Balanced` avoids.
fn is_platform_encoder(name: &str) -> bool {
    matches!(name, "h264_videotoolbox" | "h264_mf" | "h264_vaapi")
}

/// Resolves the H.264 encoder an export job should open.
///
/// An explicit `requested` name wins whenever it is present and opened: a
/// person who chose an encoder should get that encoder, and a profile is a
/// default rather than a veto. When the request cannot be honoured the plan
/// says why and falls back to the profile's automatic choice, so an export
/// still produces a file instead of failing on a preference.
pub fn plan_h264_encoding(
    profile: PerformanceProfile,
    requested: Option<&str>,
    candidates: &[EncoderCandidate],
) -> ExportEncodingPlan {
    let mut notes = Vec::new();
    let mut fallback = None;

    if let Some(name) = requested.map(str::trim).filter(|name| !name.is_empty()) {
        match candidates.iter().find(|candidate| candidate.name == name) {
            None => {
                fallback = Some(format!(
                    "this build does not carry the {name} encoder; choosing automatically"
                ));
            }
            Some(candidate) if candidate.present && candidate.opens => {
                if profile == PerformanceProfile::Compatibility && candidate.hardware {
                    notes.push(
                        "You chose a hardware encoder while the Compatibility profile is on; \
                         the explicit choice is kept."
                            .to_owned(),
                    );
                }
                return ExportEncodingPlan {
                    profile,
                    h264_encoder: Some(candidate.name.clone()),
                    hardware: candidate.hardware,
                    software: !candidate.hardware,
                    fallback,
                    notes,
                };
            }
            Some(candidate) => {
                fallback = Some(candidate.note.clone().unwrap_or_else(|| {
                    format!("{name} did not open on this machine; choosing automatically")
                }));
            }
        }
    }

    let usable: Vec<&EncoderCandidate> = candidates
        .iter()
        .filter(|candidate| candidate.present && candidate.opens)
        .collect();
    let chosen = match profile {
        PerformanceProfile::Compatibility => usable
            .iter()
            .copied()
            .find(|candidate| !candidate.hardware)
            .or_else(|| usable.first().copied()),
        PerformanceProfile::Balanced => usable
            .iter()
            .copied()
            .find(|candidate| candidate.hardware && is_platform_encoder(&candidate.name))
            .or_else(|| usable.iter().copied().find(|candidate| !candidate.hardware)),
        PerformanceProfile::Recommended | PerformanceProfile::MaximumPerformance => {
            usable.first().copied()
        }
    };

    let Some(chosen) = chosen else {
        let reason = "no H.264 encoder opened on this machine".to_owned();
        notes.push(reason);
        return ExportEncodingPlan {
            profile,
            h264_encoder: None,
            hardware: false,
            software: false,
            fallback,
            notes,
        };
    };

    if profile == PerformanceProfile::Compatibility && chosen.hardware {
        notes.push(
            "No software H.264 encoder opened, so this export uses the only encoder that did."
                .to_owned(),
        );
    }
    for candidate in candidates {
        if candidate.present
            && !candidate.opens
            && let Some(note) = &candidate.note
        {
            notes.push(format!("{}: {note}", candidate.name));
        }
    }

    ExportEncodingPlan {
        profile,
        h264_encoder: Some(chosen.name.clone()),
        hardware: chosen.hardware,
        software: !chosen.hardware,
        fallback,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::{EncoderCandidate, PerformanceProfile, plan_h264_encoding};

    fn candidate(name: &str, opens: bool, hardware: bool) -> EncoderCandidate {
        EncoderCandidate {
            name: name.to_owned(),
            present: true,
            opens,
            hardware,
            note: (!opens).then(|| format!("{name} did not open")),
        }
    }

    fn machine() -> Vec<EncoderCandidate> {
        vec![
            candidate("h264_nvenc", true, true),
            candidate("h264_mf", true, true),
            candidate("libx264", true, false),
        ]
    }

    #[test]
    fn profiles_round_trip_their_wire_names() {
        for profile in PerformanceProfile::ALL {
            assert_eq!(
                PerformanceProfile::parse(profile.wire_name()),
                Some(profile)
            );
        }
        assert_eq!(PerformanceProfile::parse(" turbo "), None);
    }

    #[test]
    fn only_maximum_performance_starts_on_speed() {
        use crate::ExportQuality;
        assert_eq!(
            PerformanceProfile::MaximumPerformance.default_quality(),
            ExportQuality::Speed
        );
        for profile in PerformanceProfile::ALL
            .into_iter()
            .filter(|profile| *profile != PerformanceProfile::MaximumPerformance)
        {
            assert_eq!(profile.default_quality(), ExportQuality::Quality);
        }
    }

    #[test]
    fn recommended_takes_the_first_open_encoder() {
        let plan = plan_h264_encoding(PerformanceProfile::Recommended, None, &machine());
        assert_eq!(plan.h264_encoder.as_deref(), Some("h264_nvenc"));
        assert!(plan.hardware);
        assert_eq!(plan.fallback, None);
    }

    #[test]
    fn balanced_prefers_the_platform_encoder_over_a_vendor_one() {
        let plan = plan_h264_encoding(PerformanceProfile::Balanced, None, &machine());
        assert_eq!(plan.h264_encoder.as_deref(), Some("h264_mf"));
    }

    #[test]
    fn compatibility_refuses_hardware() {
        let plan = plan_h264_encoding(PerformanceProfile::Compatibility, None, &machine());
        assert_eq!(plan.h264_encoder.as_deref(), Some("libx264"));
        assert!(plan.software);
    }

    #[test]
    fn an_explicit_encoder_wins_over_the_profile() {
        let plan = plan_h264_encoding(
            PerformanceProfile::Compatibility,
            Some("h264_nvenc"),
            &machine(),
        );
        assert_eq!(plan.h264_encoder.as_deref(), Some("h264_nvenc"));
        assert!(plan.hardware);
    }

    #[test]
    fn an_unavailable_request_falls_back_with_a_reason() {
        let plan = plan_h264_encoding(
            PerformanceProfile::Recommended,
            Some("h264_videotoolbox"),
            &machine(),
        );
        assert_eq!(plan.h264_encoder.as_deref(), Some("h264_nvenc"));
        assert!(
            plan.fallback
                .as_deref()
                .is_some_and(|reason| reason.contains("h264_videotoolbox"))
        );
    }

    #[test]
    fn an_encoder_that_does_not_open_is_skipped_and_explained() {
        let candidates = vec![
            EncoderCandidate {
                name: "h264_nvenc".to_owned(),
                present: true,
                opens: false,
                hardware: true,
                note: Some("no CUDA device".to_owned()),
            },
            candidate("libx264", true, false),
        ];
        let plan = plan_h264_encoding(PerformanceProfile::Recommended, None, &candidates);
        assert_eq!(plan.h264_encoder.as_deref(), Some("libx264"));
        assert!(
            plan.notes
                .iter()
                .any(|note| note.contains("h264_nvenc") && note.contains("no CUDA device"))
        );
    }

    #[test]
    fn no_usable_encoder_is_a_plan_not_a_panic() {
        let candidates = vec![candidate("h264_mf", false, true)];
        let plan = plan_h264_encoding(PerformanceProfile::Recommended, None, &candidates);
        assert_eq!(plan.h264_encoder, None);
        assert!(!plan.hardware && !plan.software);
        assert!(!plan.notes.is_empty());
    }

    #[test]
    fn compatibility_with_only_hardware_available_says_so() {
        let candidates = vec![candidate("h264_mf", true, true)];
        let plan = plan_h264_encoding(PerformanceProfile::Compatibility, None, &candidates);
        assert_eq!(plan.h264_encoder.as_deref(), Some("h264_mf"));
        assert!(
            plan.notes
                .iter()
                .any(|note| note.contains("only encoder that did"))
        );
    }

    #[test]
    fn the_summary_names_the_path() {
        let plan = plan_h264_encoding(PerformanceProfile::Recommended, None, &machine());
        assert_eq!(plan.summary(), "h264_nvenc (hardware)");
        let software = plan_h264_encoding(PerformanceProfile::Compatibility, None, &machine());
        assert_eq!(software.summary(), "libx264 (software)");
    }
}
