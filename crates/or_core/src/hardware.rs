//! Which encoder an export may use, and why one was picked.
//!
//! This module is pure policy: it never probes FFmpeg, wgpu or the operating
//! system. A caller measures what the machine can actually do (`or_media`
//! opens the codecs, `or_render` enumerates the adapters) and hands the facts
//! here, so the same decision runs in a test, the app, the CLI and an export
//! job — and the choice is made once per job, never inside the frame loop.

use serde::{Deserialize, Serialize};

use crate::ExportQuality;

/// How an export should use this machine.
///
/// There is one answer: the plan is resolved from what this machine actually
/// opened on this boot. The setting keeps a wire name because an export request
/// records it and older settings files carry one, but the tiers that name once
/// had (`recommended`, `maximum_performance`, `balanced`, `compatibility`) are
/// gone — each of them asked a person to answer a question the machine can
/// answer itself, and every one of them still ended in the same probe.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerformanceProfile {
    /// The fastest path that opened here, with the reason recorded.
    #[default]
    Automatic,
}

impl PerformanceProfile {
    pub const ALL: [Self; 1] = [Self::Automatic];

    pub const fn wire_name(self) -> &'static str {
        "automatic"
    }

    /// Parses the wire name the settings file and the export request carry.
    ///
    /// The names of the retired tiers still parse, because a settings file, a
    /// script or an agent may carry one; they all mean the same thing now.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "" => None,
            "automatic" | "recommended" | "maximum_performance" | "balanced"
            | "compatibility" => Some(Self::Automatic),
            _ => None,
        }
    }

    /// The quality an export dialog should start on.
    ///
    /// Quality is an output choice, so it stays in the export dialog and is
    /// never inferred from a machine preference.
    pub const fn default_quality(self) -> ExportQuality {
        ExportQuality::Quality
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
/// change behaviour with a driver update, so they are a second choice when the
/// platform path also opened.
fn is_platform_encoder(name: &str) -> bool {
    matches!(name, "h264_videotoolbox" | "h264_mf" | "h264_vaapi")
}

/// Resolves the H.264 encoder an export job should open.
///
/// An explicit `requested` name wins whenever it is present and opened: a
/// person who chose an encoder should get that encoder. When the request
/// cannot be honoured the plan says why and falls back to the automatic
/// choice, so an export still produces a file instead of failing on a
/// preference.
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
    let chosen = usable
        .iter()
        .copied()
        .find(|candidate| candidate.hardware && is_platform_encoder(&candidate.name))
        .or_else(|| usable.iter().copied().find(|candidate| candidate.hardware))
        .or_else(|| usable.first().copied());

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
    fn the_one_profile_round_trips_and_old_names_still_parse() {
        for profile in PerformanceProfile::ALL {
            assert_eq!(
                PerformanceProfile::parse(profile.wire_name()),
                Some(profile)
            );
        }
        for retired in [
            "recommended",
            "maximum_performance",
            "balanced",
            "compatibility",
        ] {
            assert_eq!(
                PerformanceProfile::parse(retired),
                Some(PerformanceProfile::Automatic),
                "{retired} is a settings file from an older build"
            );
        }
        assert_eq!(PerformanceProfile::parse(" turbo "), None);
    }

    #[test]
    fn quality_is_never_inferred_from_the_machine() {
        use crate::ExportQuality;
        assert_eq!(
            PerformanceProfile::Automatic.default_quality(),
            ExportQuality::Quality
        );
    }

    #[test]
    fn automatic_prefers_the_platform_encoder_over_a_vendor_one() {
        let plan = plan_h264_encoding(PerformanceProfile::Automatic, None, &machine());
        assert_eq!(plan.h264_encoder.as_deref(), Some("h264_mf"));
        assert!(plan.hardware);
        assert_eq!(plan.fallback, None);
    }

    #[test]
    fn automatic_takes_the_first_open_encoder_when_no_platform_one_opened() {
        let candidates = vec![
            candidate("h264_nvenc", true, true),
            candidate("libx264", true, false),
        ];
        let plan = plan_h264_encoding(PerformanceProfile::Automatic, None, &candidates);
        assert_eq!(plan.h264_encoder.as_deref(), Some("h264_nvenc"));
        assert!(plan.hardware);
    }

    #[test]
    fn automatic_falls_back_to_software_when_no_hardware_opened() {
        let candidates = vec![
            candidate("h264_nvenc", false, true),
            candidate("libx264", true, false),
        ];
        let plan = plan_h264_encoding(PerformanceProfile::Automatic, None, &candidates);
        assert_eq!(plan.h264_encoder.as_deref(), Some("libx264"));
        assert!(plan.software);
    }

    #[test]
    fn an_explicit_encoder_wins_over_the_automatic_choice() {
        let plan = plan_h264_encoding(
            PerformanceProfile::Automatic,
            Some("h264_nvenc"),
            &machine(),
        );
        assert_eq!(plan.h264_encoder.as_deref(), Some("h264_nvenc"));
        assert!(plan.hardware);
    }

    #[test]
    fn an_unavailable_request_falls_back_with_a_reason() {
        let plan = plan_h264_encoding(
            PerformanceProfile::Automatic,
            Some("h264_videotoolbox"),
            &machine(),
        );
        assert_eq!(plan.h264_encoder.as_deref(), Some("h264_mf"));
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
        let plan = plan_h264_encoding(PerformanceProfile::Automatic, None, &candidates);
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
        let plan = plan_h264_encoding(PerformanceProfile::Automatic, None, &candidates);
        assert_eq!(plan.h264_encoder, None);
        assert!(!plan.hardware && !plan.software);
        assert!(!plan.notes.is_empty());
    }

    #[test]
    fn the_summary_names_the_path() {
        let plan = plan_h264_encoding(PerformanceProfile::Automatic, None, &machine());
        assert_eq!(plan.summary(), "h264_mf (hardware)");
        let software = plan_h264_encoding(
            PerformanceProfile::Automatic,
            None,
            &[candidate("libx264", true, false)],
        );
        assert_eq!(software.summary(), "libx264 (software)");
    }
}
