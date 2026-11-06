//! What this machine can do, in the shape the Performance settings need.
//!
//! Every entry point here probes once and returns a decision plus its reasons.
//! Nothing in this module runs per frame: the export path calls it before a
//! file is opened, and the settings panel calls it when the person opens it.

use or_core::{
    DecoderCandidate, EncoderCandidate, ExportEncodingPlan, ExportQuality, PerformanceProfile,
    plan_h264_encoding,
};
use or_render::RenderAdapterInfo;

use super::views::ProjectBridgeError;

/// One H.264 encoder this build carries and what the open probe found.
#[derive(Clone, Debug)]
pub struct EncoderProbeView {
    pub name: String,
    /// The linked FFmpeg carries it.
    pub present: bool,
    /// This machine opened it just now.
    pub opens: bool,
    pub hardware: bool,
    pub note: Option<String>,
}

/// One decoder this build carries.
#[derive(Clone, Debug)]
pub struct DecoderProbeView {
    pub name: String,
    pub present: bool,
}

/// The encoder an export job will open, and why.
#[derive(Clone, Debug)]
pub struct EncodingPlanView {
    pub profile: String,
    /// "quality" or "speed": what the profile spends by default.
    pub quality: String,
    pub h264_encoder: Option<String>,
    pub hardware: bool,
    pub software: bool,
    pub fallback: Option<String>,
    pub notes: Vec<String>,
    pub summary: String,
}

/// The machine facts the performance decision reads.
#[derive(Clone, Debug)]
pub struct PerformanceReportView {
    pub cpu_arch: String,
    pub logical_cores: u32,
    pub gpu_name: String,
    pub gpu_backend: String,
    pub gpu_device_type: String,
    pub gpu_driver: String,
    pub gpu_driver_info: String,
    /// Native backends this build can open, e.g. `["metal"]`.
    pub gpu_compiled_backends: Vec<String>,
    /// Set when the render device could not be opened for the report.
    pub gpu_error: Option<String>,
    pub h264_encoders: Vec<EncoderProbeView>,
    pub decoders: Vec<DecoderProbeView>,
    /// One plan per profile, in the order the profile picker shows them.
    pub plans: Vec<EncodingPlanView>,
}

/// Reports this machine's execution-relevant facts and the plans they imply.
///
/// A missing GPU is reported through `gpu_error` rather than failing the whole
/// report: the media facts still decide a usable export on a machine whose
/// preview is broken.
pub fn performance_report() -> PerformanceReportView {
    let candidates = or_media::probe_h264_encoders();
    let plans = PerformanceProfile::ALL
        .iter()
        .map(|profile| {
            EncodingPlanView::from_plan(&plan_h264_encoding(*profile, None, &candidates))
        })
        .collect();
    let (gpu_name, gpu_backend, gpu_device_type, gpu_driver, gpu_driver_info, gpu_error) =
        match crate::preview::render_adapter_info() {
            Ok(adapter) => adapter_columns(&adapter),
            Err(message) => (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                Some(message),
            ),
        };

    PerformanceReportView {
        cpu_arch: std::env::consts::ARCH.to_owned(),
        logical_cores: std::thread::available_parallelism()
            .map_or(1, |count| u32::try_from(count.get()).unwrap_or(u32::MAX)),
        gpu_name,
        gpu_backend,
        gpu_device_type,
        gpu_driver,
        gpu_driver_info,
        gpu_compiled_backends: or_render::RenderDevice::compiled_backend_names()
            .into_iter()
            .map(str::to_owned)
            .collect(),
        gpu_error,
        h264_encoders: candidates.iter().map(EncoderProbeView::from).collect(),
        decoders: or_media::probe_decoders()
            .into_iter()
            .map(DecoderProbeView::from)
            .collect(),
        plans,
    }
}

/// Resolves the plan for one profile, honouring an encoder the person chose.
///
/// The settings panel and the export dialog both ask this instead of repeating
/// the policy in Dart.
pub fn encoding_plan(
    profile: String,
    preferred_h264_encoder: Option<String>,
) -> Result<EncodingPlanView, ProjectBridgeError> {
    let profile = PerformanceProfile::parse(&profile).ok_or_else(|| ProjectBridgeError {
        code: "INVALID_PERFORMANCE_PROFILE".to_owned(),
        message: format!("unknown performance profile {profile:?}"),
    })?;
    let candidates = or_media::probe_h264_encoders();
    Ok(EncodingPlanView::from_plan(&plan_h264_encoding(
        profile,
        preferred_h264_encoder.as_deref(),
        &candidates,
    )))
}

impl From<&EncoderCandidate> for EncoderProbeView {
    fn from(candidate: &EncoderCandidate) -> Self {
        Self {
            name: candidate.name.clone(),
            present: candidate.present,
            opens: candidate.opens,
            hardware: candidate.hardware,
            note: candidate.note.clone(),
        }
    }
}

impl From<DecoderCandidate> for DecoderProbeView {
    fn from(candidate: DecoderCandidate) -> Self {
        Self {
            name: candidate.name,
            present: candidate.present,
        }
    }
}

impl EncodingPlanView {
    fn from_plan(plan: &ExportEncodingPlan) -> Self {
        Self {
            profile: plan.profile.wire_name().to_owned(),
            quality: match plan.profile.default_quality() {
                ExportQuality::Quality => "quality".to_owned(),
                ExportQuality::Speed => "speed".to_owned(),
            },
            h264_encoder: plan.h264_encoder.clone(),
            hardware: plan.hardware,
            software: plan.software,
            fallback: plan.fallback.clone(),
            notes: plan.notes.clone(),
            summary: plan.summary(),
        }
    }
}

fn adapter_columns(
    adapter: &RenderAdapterInfo,
) -> (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
) {
    (
        adapter.name().to_owned(),
        adapter.backend_name().to_owned(),
        adapter.device_type_name().to_owned(),
        adapter.driver().to_owned(),
        adapter.driver_info().to_owned(),
        None,
    )
}

