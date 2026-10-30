// Trim and ripple trims, plus slip, roll and slide edits.

use super::super::*;
use super::support::*;
use super::*;
use serde_json::json;

#[test]
fn ripple_trim_pulls_every_later_clip_left() {
    let mut session = session_with_state("ripple-trim", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (4, 1));
    insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (4, 1), (2, 1));
    insert_clip_at(&mut session, CLIP_C, TRACK_A, MEDIA_A, (6, 1), (2, 1));
    let revision_before = session.project_revision();

    ripple_trim_end(&mut session, CLIP_A, (2, 1), true).unwrap();

    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].2, RationalTime::new(2, 1).unwrap());
    assert_eq!(
        spans[1].1,
        RationalTime::new(2, 1).unwrap(),
        "the next clip closes the gap"
    );
    assert_eq!(
        spans[2].1,
        RationalTime::new(4, 1).unwrap(),
        "and the one after it moves by the same delta"
    );
    assert_eq!(spans[1].2, RationalTime::new(2, 1).unwrap());
    assert_eq!(
        session.project_revision().value(),
        revision_before.value() + 1,
        "a ripple trim is one revision"
    );

    execute(&mut session, "history.undo", json!({})).unwrap();
    let restored = clip_spans(&session, TRACK_A);
    assert_eq!(restored[0].2, RationalTime::new(4, 1).unwrap());
    assert_eq!(
        restored[1].1,
        RationalTime::new(4, 1).unwrap(),
        "one undo restores every shifted clip"
    );
    assert_eq!(restored[2].1, RationalTime::new(6, 1).unwrap());
}

#[test]
fn ripple_trim_opens_a_gap_when_the_clip_lengthens() {
    let mut session = session_with_state("ripple-lengthen", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (2, 1));
    insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (2, 1), (2, 1));

    ripple_trim_end(&mut session, CLIP_A, (3, 1), true).unwrap();

    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans[0].2, RationalTime::new(3, 1).unwrap());
    assert_eq!(
        spans[1].1,
        RationalTime::new(3, 1).unwrap(),
        "the later clip moves right with the extended edge"
    );
}

#[test]
fn plain_trim_leaves_later_clips_where_they_are() {
    let mut session = session_with_state("plain-trim", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (4, 1));
    insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (4, 1), (2, 1));

    // No ripple argument at all: the default must keep the old meaning, which
    // is the regression that would silently move every user's later clips.
    execute(
        &mut session,
        "timeline.clip.trim",
        json!({
            "clip_id": CLIP_A,
            "edge": "end",
            "timeline_time": rational_json(2, 1),
        }),
    )
    .unwrap();

    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans[0].2, RationalTime::new(2, 1).unwrap());
    assert_eq!(
        spans[1].1,
        RationalTime::new(4, 1).unwrap(),
        "a plain trim leaves the gap"
    );
}

#[test]
fn a_ripple_trim_to_the_current_edge_is_a_no_op() {
    let mut session = session_with_state("ripple-noop", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (4, 1));
    insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (4, 1), (2, 1));
    let revision_before = session.project_revision();

    // A ripple is still a trim: an edge that does not move changes nothing and,
    // in particular, must not shift the later clips.
    let result = ripple_trim_end(&mut session, CLIP_A, (4, 1), true).unwrap();
    assert!(!result.changed);
    assert_eq!(session.project_revision().value(), revision_before.value());
    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans[0].2, RationalTime::new(4, 1).unwrap());
    assert_eq!(spans[1].1, RationalTime::new(4, 1).unwrap());
}

#[test]
fn a_ripple_trim_outside_the_clip_is_refused_and_changes_nothing() {
    let mut session = session_with_state("ripple-refused", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (2, 1));
    insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (2, 1), (2, 1));
    let revision_before = session.project_revision();

    // An end trim to or before the clip's own start is invalid for any end
    // trim, ripple or not, and the track must be untouched afterwards. (An end
    // beyond the current end is a lengthening and is legal.)
    let error = ripple_trim_end(&mut session, CLIP_A, (0, 1), true).unwrap_err();
    assert_eq!(
        error.code,
        super::super::OperationErrorCode::InvalidArguments
    );
    assert_eq!(session.project_revision().value(), revision_before.value());
    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans[0].2, RationalTime::new(2, 1).unwrap());
    assert_eq!(spans[1].1, RationalTime::new(2, 1).unwrap());
}

#[test]
fn ripple_on_a_start_edge_is_refused_with_the_plain_trim_meaning() {
    let mut session = session_with_state("ripple-start", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (4, 1));

    let error = execute(
        &mut session,
        "timeline.clip.trim",
        json!({
            "clip_id": CLIP_A,
            "edge": "start",
            "timeline_time": rational_json(1, 1),
            "ripple": true,
        }),
    )
    .unwrap_err();

    assert_eq!(
        error.code,
        super::super::OperationErrorCode::InvalidArguments,
        "ripple-in has two meanings; refusing is the honest default"
    );
    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans[0].1, RationalTime::new(0, 1).unwrap());
    assert_eq!(spans[0].2, RationalTime::new(4, 1).unwrap());
}

#[test]
fn slip_moves_the_source_window_and_leaves_the_cut_where_it_was() {
    let mut session = session_with_state("slip", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (1, 1), (1, 1));
    insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (2, 1), (1, 1));
    let revision_before = session.project_revision();
    let spans_before = clip_spans(&session, TRACK_A);

    slip_clip(&mut session, CLIP_A, (1, 4)).unwrap();

    let spans_after = clip_spans(&session, TRACK_A);
    assert_eq!(
        spans_before, spans_after,
        "a slip never moves a clip or its neighbours"
    );
    assert_eq!(
        source_start(&session, TRACK_A, CLIP_A),
        RationalTime::new(1, 4).unwrap(),
        "the source window moved by the delta"
    );
    assert_eq!(
        session.project_revision().value(),
        revision_before.value() + 1,
        "a slip is one revision"
    );

    execute(&mut session, "history.undo", json!({})).unwrap();
    assert_eq!(
        source_start(&session, TRACK_A, CLIP_A),
        RationalTime::new(0, 1).unwrap(),
        "undo restores the original source window"
    );
    assert_eq!(spans_before, clip_spans(&session, TRACK_A));
}

#[test]
fn slip_backwards_takes_earlier_source_material() {
    let mut session = session_with_state("slip-back", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
    // Start an eighth of a second into the source so there is room to go back.
    slip_clip(&mut session, CLIP_A, (1, 8)).unwrap();
    assert_eq!(
        source_start(&session, TRACK_A, CLIP_A),
        RationalTime::new(1, 8).unwrap()
    );

    slip_clip(&mut session, CLIP_A, (-1, 8)).unwrap();
    assert_eq!(
        source_start(&session, TRACK_A, CLIP_A),
        RationalTime::new(0, 1).unwrap(),
        "a negative delta takes earlier material"
    );
}

#[test]
fn a_slip_past_the_start_of_the_source_is_refused() {
    let mut session = session_with_state("slip-refused", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
    let revision_before = session.project_revision();

    // Nothing exists before the start of the media, so the clip must not be
    // left pointing at it.
    let error = slip_clip(&mut session, CLIP_A, (-1, 1)).unwrap_err();
    assert_eq!(
        error.code,
        super::super::OperationErrorCode::InvalidArguments
    );
    assert_eq!(session.project_revision().value(), revision_before.value());
    assert_eq!(
        source_start(&session, TRACK_A, CLIP_A),
        RationalTime::new(0, 1).unwrap()
    );
}

#[test]
fn a_slip_of_zero_is_a_no_op() {
    let mut session = session_with_state("slip-zero", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
    let revision_before = session.project_revision();

    let result = slip_clip(&mut session, CLIP_A, (0, 1)).unwrap();
    assert!(!result.changed);
    assert_eq!(session.project_revision().value(), revision_before.value());
}

#[test]
fn roll_moves_the_boundary_and_both_source_windows() {
    let mut session = session_with_state("roll", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
    insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (1, 1), (1, 1));
    let revision_before = session.project_revision();

    roll_boundary(&mut session, CLIP_A, (1, 4)).unwrap();

    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(
        spans[0].2,
        RationalTime::new(5, 4).unwrap(),
        "the left clip gives up a quarter second"
    );
    assert_eq!(
        spans[1].1,
        RationalTime::new(5, 4).unwrap(),
        "the right clip starts where the boundary moved to"
    );
    assert_eq!(spans[1].2, RationalTime::new(3, 4).unwrap());
    assert_eq!(
        source_start(&session, TRACK_A, CLIP_A),
        RationalTime::new(0, 1).unwrap(),
        "the left clip's source does not move"
    );
    assert_eq!(
        source_start(&session, TRACK_A, CLIP_B),
        RationalTime::new(1, 4).unwrap(),
        "the right clip's source advances with its start"
    );
    assert_eq!(
        session.project_revision().value(),
        revision_before.value() + 1,
        "a roll is one revision"
    );

    execute(&mut session, "history.undo", json!({})).unwrap();
    let restored = clip_spans(&session, TRACK_A);
    assert_eq!(restored[0].2, RationalTime::new(1, 1).unwrap());
    assert_eq!(restored[1].1, RationalTime::new(1, 1).unwrap());
    assert_eq!(
        source_start(&session, TRACK_A, CLIP_B),
        RationalTime::new(0, 1).unwrap(),
        "one undo restores both sides"
    );
}

#[test]
fn roll_needs_two_clips_that_actually_touch() {
    let mut session = session_with_state("roll-gap", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
    // A one-second gap: there is no shared boundary to roll.
    insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (2, 1), (1, 1));

    let error = roll_boundary(&mut session, CLIP_A, (1, 4)).unwrap_err();
    assert_eq!(
        error.code,
        super::super::OperationErrorCode::InvalidArguments
    );
    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans[0].2, RationalTime::new(1, 1).unwrap());
    assert_eq!(spans[1].1, RationalTime::new(2, 1).unwrap());
}

#[test]
fn roll_past_either_clip_is_refused() {
    let mut session = session_with_state("roll-bounds", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
    insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (1, 1), (1, 1));

    // Consuming the whole right clip leaves it no duration.
    let error = roll_boundary(&mut session, CLIP_A, (1, 1)).unwrap_err();
    assert_eq!(
        error.code,
        super::super::OperationErrorCode::InvalidArguments
    );
    // And the same on the other side.
    let error = roll_boundary(&mut session, CLIP_A, (-1, 1)).unwrap_err();
    assert_eq!(
        error.code,
        super::super::OperationErrorCode::InvalidArguments
    );
    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans[0].2, RationalTime::new(1, 1).unwrap());
    assert_eq!(spans[1].2, RationalTime::new(1, 1).unwrap());
}

#[test]
fn a_roll_of_zero_is_a_no_op() {
    let mut session = session_with_state("roll-zero", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
    insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (1, 1), (1, 1));
    let revision_before = session.project_revision();

    let result = roll_boundary(&mut session, CLIP_A, (0, 1)).unwrap();
    assert!(!result.changed);
    assert_eq!(session.project_revision().value(), revision_before.value());
}

#[test]
fn slide_moves_the_clip_and_lets_both_neighbours_absorb_it() {
    let mut session = session_with_state("slide", 0);
    seed_three_adjacent_clips(&mut session);
    let revision_before = session.project_revision();

    slide_clip(&mut session, CLIP_B, (1, 4)).unwrap();

    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(
        spans[0].2,
        RationalTime::new(5, 4).unwrap(),
        "the previous clip grows"
    );
    assert_eq!(
        spans[1].1,
        RationalTime::new(5, 4).unwrap(),
        "the clip moves"
    );
    assert_eq!(
        spans[1].2,
        RationalTime::new(1, 1).unwrap(),
        "and keeps its duration"
    );
    assert_eq!(
        spans[2].1,
        RationalTime::new(9, 4).unwrap(),
        "the next clip starts later"
    );
    assert_eq!(
        spans[2].2,
        RationalTime::new(3, 4).unwrap(),
        "and gives up the same amount"
    );
    assert_eq!(
        source_start(&session, TRACK_A, CLIP_B),
        RationalTime::new(0, 1).unwrap(),
        "the moving clip's source never changes"
    );
    assert_eq!(
        source_start(&session, TRACK_A, CLIP_C),
        RationalTime::new(1, 4).unwrap(),
        "the next clip's source advances with its start"
    );
    assert_eq!(
        session.project_revision().value(),
        revision_before.value() + 1
    );

    execute(&mut session, "history.undo", json!({})).unwrap();
    let restored = clip_spans(&session, TRACK_A);
    assert_eq!(restored[0].2, RationalTime::new(1, 1).unwrap());
    assert_eq!(restored[1].1, RationalTime::new(1, 1).unwrap());
    assert_eq!(restored[2].1, RationalTime::new(2, 1).unwrap());
    assert_eq!(
        source_start(&session, TRACK_A, CLIP_C),
        RationalTime::new(0, 1).unwrap(),
        "one undo restores all three clips"
    );
}

#[test]
fn slide_backwards_is_refused_when_the_next_clip_has_no_earlier_material() {
    let mut session = session_with_state("slide-back-refused", 0);
    seed_three_adjacent_clips(&mut session);

    // The next clip starts at the beginning of its own source: it has nothing
    // earlier to give, so pulling the clip back would reference material that
    // does not exist. That is a refusal, not a trim.
    let error = slide_clip(&mut session, CLIP_B, (-1, 4)).unwrap_err();
    assert_eq!(
        error.code,
        super::super::OperationErrorCode::InvalidArguments
    );
    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans[1].1, RationalTime::new(1, 1).unwrap());
    assert_eq!(spans[2].1, RationalTime::new(2, 1).unwrap());
}

#[test]
fn slide_backwards_pulls_the_clip_earlier_when_the_material_allows_it() {
    let mut session = session_with_state("slide-back", 0);
    seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
    insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
    insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (1, 1), (1, 1));
    // This clip starts a quarter second into its source, so it can give that
    // quarter second back when the slide pulls it earlier.
    insert_clip(
        &mut session,
        CLIP_C,
        TRACK_A,
        MEDIA_A,
        (2, 1),
        (1, 4),
        (1, 1),
    )
    .unwrap();

    slide_clip(&mut session, CLIP_B, (-1, 4)).unwrap();

    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans[0].2, RationalTime::new(3, 4).unwrap());
    assert_eq!(spans[1].1, RationalTime::new(3, 4).unwrap());
    assert_eq!(
        spans[1].2,
        RationalTime::new(1, 1).unwrap(),
        "the moving clip keeps its duration"
    );
    assert_eq!(spans[2].1, RationalTime::new(7, 4).unwrap());
    assert_eq!(spans[2].2, RationalTime::new(5, 4).unwrap());
    assert_eq!(
        source_start(&session, TRACK_A, CLIP_C),
        RationalTime::new(0, 1).unwrap(),
        "the next clip's source moves back with its start"
    );
}

#[test]
fn slide_needs_touching_neighbours_on_both_sides() {
    let mut session = session_with_state("slide-edges", 0);
    seed_three_adjacent_clips(&mut session);
    // The first and last clip of a track have no neighbour to borrow from.
    let error = slide_clip(&mut session, CLIP_A, (1, 4)).unwrap_err();
    assert_eq!(
        error.code,
        super::super::OperationErrorCode::InvalidArguments
    );
    let error = slide_clip(&mut session, CLIP_C, (1, 4)).unwrap_err();
    assert_eq!(
        error.code,
        super::super::OperationErrorCode::InvalidArguments
    );

    // And a gap means there is nothing to borrow either.
    let mut gapped = session_with_state("slide-gap", 0);
    seed_media_and_track(&mut gapped, MEDIA_A, TRACK_A);
    insert_clip_at(&mut gapped, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
    insert_clip_at(&mut gapped, CLIP_B, TRACK_A, MEDIA_A, (2, 1), (1, 1));
    insert_clip_at(&mut gapped, CLIP_C, TRACK_A, MEDIA_A, (3, 1), (1, 1));
    let error = slide_clip(&mut gapped, CLIP_B, (1, 4)).unwrap_err();
    assert_eq!(
        error.code,
        super::super::OperationErrorCode::InvalidArguments
    );
    let spans = clip_spans(&gapped, TRACK_A);
    assert_eq!(spans[1].1, RationalTime::new(2, 1).unwrap());
}

#[test]
fn slide_that_would_consume_a_neighbour_is_refused() {
    let mut session = session_with_state("slide-bounds", 0);
    seed_three_adjacent_clips(&mut session);

    let error = slide_clip(&mut session, CLIP_B, (2, 1)).unwrap_err();
    assert_eq!(
        error.code,
        super::super::OperationErrorCode::InvalidArguments
    );
    let error = slide_clip(&mut session, CLIP_B, (-2, 1)).unwrap_err();
    assert_eq!(
        error.code,
        super::super::OperationErrorCode::InvalidArguments
    );
    let spans = clip_spans(&session, TRACK_A);
    assert_eq!(spans[0].2, RationalTime::new(1, 1).unwrap());
    assert_eq!(spans[1].1, RationalTime::new(1, 1).unwrap());
}

#[test]
fn a_slide_of_zero_is_a_no_op() {
    let mut session = session_with_state("slide-zero", 0);
    seed_three_adjacent_clips(&mut session);
    let revision_before = session.project_revision();

    let result = slide_clip(&mut session, CLIP_B, (0, 1)).unwrap();
    assert!(!result.changed);
    assert_eq!(session.project_revision().value(), revision_before.value());
}
