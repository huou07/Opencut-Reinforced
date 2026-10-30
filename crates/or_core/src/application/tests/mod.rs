pub(super) use crate::application::timeline::{
    ensure_timeline_clip_capacity, find_timeline_track_index,
};
pub(super) use crate::media::{
    AudioStreamMetadata, MediaMetadata, MediaSourceRef, MediaStreamMetadata, VideoStreamMetadata,
};
pub(super) use crate::project_document::{decode_project, encode_project};
pub(super) use crate::timeline::*;
pub(super) use std::num::NonZeroU32;

// Application command and query behaviour, grouped by the owner file it exercises.

const PROJECT_ID: &str = "01234567-89ab-4def-8123-456789abcdef";
const OTHER_PROJECT_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const INSTANCE_ID: &str = "fedcba98-7654-4cba-8fed-cba987654321";
const OTHER_INSTANCE_ID: &str = "11111111-1111-4111-8111-111111111111";
const TRACK_A: &str = "22222222-2222-4222-8222-222222222222";
const TRACK_B: &str = "88888888-8888-4888-8888-888888888888";
const TRACK_C: &str = "99999999-9999-4999-8999-999999999999";
const TRACK_D: &str = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee";
const CLIP_A: &str = "33333333-3333-4333-8333-333333333333";
const CLIP_B: &str = "55555555-5555-4555-8555-555555555555";
const CLIP_C: &str = "66666666-6666-4666-8666-666666666666";
const CLIP_D: &str = "99999999-9999-4999-8999-999999999998";
const CLIP_E: &str = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeef";
const MEDIA_A: &str = "44444444-4444-4444-8444-444444444444";
const MEDIA_B: &str = "77777777-7777-4777-8777-777777777777";
const MEDIA_C: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaab";
const MEDIA_D: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const MARKER_A: &str = "11111111-1111-4111-8111-111111111111";
const MARKER_B: &str = "33333333-3333-4333-8333-333333333333";
const MARKER_C: &str = "55555555-5555-4555-8555-555555555555";

mod clips;
mod ducking;
mod history;
mod markers;
mod media;
mod project;
mod queries;
mod support;
mod tracks;
mod trim;
