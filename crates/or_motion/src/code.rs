//! Turns a code artifact into a MotionScene.
//!
//! This is the front half of the code-to-illustration direction: a source file
//! becomes a scene of highlighted lines that the normal evaluator and rasteriser then
//! draw. Nothing here executes the sample, and nothing reads anything but the bytes
//! it is handed — a sample is untrusted input, so the tokeniser is a bounded lexical
//! scan with hard limits on size, lines and columns, not a parser and not a compiler.
//!
//! The tokeniser is deliberately small and honest about it: comments, strings,
//! numbers, keywords and call-shaped identifiers for a known language, and plain text
//! for anything else. It has no grammar, no macro expansion and no type information,
//! which is why it is called a highlighter rather than an analyser.

use crate::{
    MOTION_SCENE_FORMAT, MOTION_SCENE_SCHEMA_VERSION, MotionAnimation, MotionCanvas, MotionColor,
    MotionEasing, MotionKeyframe, MotionNode, MotionNodeKind, MotionPoint, MotionProperty,
    MotionScene, MotionSize, MotionTextSpan,
};
use or_core::RationalTime;

/// The most source this ingests.
pub const MAX_CODE_SOURCE_BYTES: usize = 1024 * 1024;
/// The most lines one scene may show. A wall of code is not an illustration.
pub const MAX_CODE_LINES: usize = 400;
/// The most columns one line may show, before it is clipped.
pub const MAX_CODE_COLUMNS: usize = 120;
/// Maximum bytes for a source line before it is clipped.
pub const MAX_CODE_LINE_BYTES: usize = 4096;
/// Keep generated reveal scenes under the MotionScene node and animation limits.
const MAX_CODE_REVEAL_STEPS_PER_LINE: usize = 8;
/// A short opacity transition makes each typewriter step readable without lingering
/// ghost copies of the previous prefix.
const CODE_REVEAL_TRANSITION_MILLISECONDS: u32 = 1;

/// The languages this highlights. Anything else is drawn as plain text, which is a
/// supported outcome rather than a failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeLanguage {
    Rust,
    Python,
    PlainText,
}

impl CodeLanguage {
    /// The language for a file name's extension, defaulting to plain text.
    pub fn from_path(path: &str) -> Self {
        let lowered = path.to_ascii_lowercase();
        if lowered.ends_with(".rs") {
            Self::Rust
        } else if lowered.ends_with(".py") || lowered.ends_with(".pyi") {
            Self::Python
        } else {
            Self::PlainText
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::PlainText => "plain_text",
        }
    }
}

/// The locked colour set a code scene is drawn with. Named roles, not free colours,
/// so a theme is a value rather than a stylesheet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodeTheme {
    pub background: MotionColor,
    pub foreground: MotionColor,
    pub keyword: MotionColor,
    pub string: MotionColor,
    pub comment: MotionColor,
    pub number: MotionColor,
    pub function: MotionColor,
}

impl CodeTheme {
    /// The default dark theme. Colours are chosen for legibility against
    /// `background` rather than for novelty.
    pub const OR_DARK: Self = Self {
        background: MotionColor {
            red: 24,
            green: 26,
            blue: 31,
            alpha: 255,
        },
        foreground: MotionColor {
            red: 220,
            green: 223,
            blue: 228,
            alpha: 255,
        },
        keyword: MotionColor {
            red: 197,
            green: 134,
            blue: 192,
            alpha: 255,
        },
        string: MotionColor {
            red: 152,
            green: 195,
            blue: 121,
            alpha: 255,
        },
        comment: MotionColor {
            red: 128,
            green: 132,
            blue: 140,
            alpha: 255,
        },
        number: MotionColor {
            red: 209,
            green: 154,
            blue: 102,
            alpha: 255,
        },
        function: MotionColor {
            red: 97,
            green: 175,
            blue: 239,
            alpha: 255,
        },
    };
}

/// What a caller chooses about the scene built from a sample.
#[derive(Clone, Debug)]
pub struct CodeSceneOptions {
    pub title: String,
    pub canvas: MotionCanvas,
    pub font_size: f64,
    pub theme: CodeTheme,
    /// How long one line is revealed before the next begins, in milliseconds.
    pub line_milliseconds: u32,
    /// How long the finished frame holds, in milliseconds.
    pub hold_milliseconds: u32,
}

impl Default for CodeSceneOptions {
    fn default() -> Self {
        Self {
            title: "Code".to_owned(),
            canvas: MotionCanvas {
                width: 1920,
                height: 1080,
            },
            font_size: 28.0,
            theme: CodeTheme::OR_DARK,
            line_milliseconds: 400,
            hold_milliseconds: 2000,
        }
    }
}

/// Why a sample could not become a scene.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeSceneError {
    /// The source is larger than [`MAX_CODE_SOURCE_BYTES`].
    SourceTooLarge,
    /// The source has more lines than [`MAX_CODE_LINES`].
    TooManyLines,
    /// The source is not valid UTF-8.
    InvalidUtf8,
}

impl CodeSceneError {
    pub fn code(self) -> &'static str {
        match self {
            Self::SourceTooLarge => "CODE_SOURCE_TOO_LARGE",
            Self::TooManyLines => "CODE_TOO_MANY_LINES",
            Self::InvalidUtf8 => "CODE_INVALID_UTF8",
        }
    }
}

/// What one token is, which is also what colour it takes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeTokenKind {
    Keyword,
    String,
    Comment,
    Number,
    Function,
    Plain,
}

impl CodeTokenKind {
    fn color(self, theme: &CodeTheme) -> MotionColor {
        match self {
            Self::Keyword => theme.keyword,
            Self::String => theme.string,
            Self::Comment => theme.comment,
            Self::Number => theme.number,
            Self::Function => theme.function,
            Self::Plain => theme.foreground,
        }
    }
}

/// One token of one line, as byte offsets into that line's text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodeToken {
    pub start: usize,
    pub end: usize,
    pub kind: CodeTokenKind,
}

/// Python's keywords, as the tokeniser needs them: a word that is a keyword is
/// highlighted wherever it appears, which is what a reader expects from a keyword list
/// rather than from a parser.
const PYTHON_KEYWORDS: [&str; 35] = [
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

const RUST_KEYWORDS: [&str; 51] = [
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while", "yield", "u8", "u16", "u32", "u64", "i32", "i64", "usize",
    "isize", "f32", "f64", "bool", "str",
];

/// Tokenises one line. Bounded by construction: it walks the bytes once and never
/// looks ahead further than the line, so a hostile sample cannot make it loop.
///
/// Multi-line constructs are deliberately not tracked: a block comment that spans
/// lines is highlighted only on the line where it opens and where it closes, and the
/// lines between are plain. That is a stated limitation of a lexical highlighter, not
/// an oversight, and it keeps the scan free of cross-line state.
/// The parts of a language a line tokeniser needs.
///
/// One scanner serves every language; a second copy per language would drift from the
/// first, and the token kinds are the same across them.
struct LanguageSyntax {
    /// The marker that starts a comment running to the end of the line.
    line_comment: &'static str,
    /// Whether `/* ... */` is a comment.
    block_comment: bool,
    /// The quote bytes that start a string literal with backslash escapes.
    string_quotes: &'static [u8],
    /// Whether `'` starts a short plain literal instead of a string (Rust lifetimes
    /// and character literals; Python uses it for strings).
    single_quote_is_plain: bool,
    keywords: &'static [&'static str],
}

fn syntax_for(language: CodeLanguage) -> LanguageSyntax {
    match language {
        CodeLanguage::Rust => LanguageSyntax {
            line_comment: "//",
            block_comment: true,
            string_quotes: b"\"",
            single_quote_is_plain: true,
            keywords: &RUST_KEYWORDS,
        },
        CodeLanguage::Python => LanguageSyntax {
            line_comment: "#",
            block_comment: false,
            // Python's single quotes are strings; its triple quotes open a string that
            // this line-based tokeniser carries to the end of the line, because it has
            // no state between lines and inventing one would change the span contract.
            string_quotes: b"\"'",
            single_quote_is_plain: false,
            keywords: &PYTHON_KEYWORDS,
        },
        CodeLanguage::PlainText => LanguageSyntax {
            line_comment: "",
            block_comment: false,
            string_quotes: b"",
            single_quote_is_plain: false,
            keywords: &[],
        },
    }
}

pub fn tokenize_line(line: &str, language: CodeLanguage) -> Vec<CodeToken> {
    if language == CodeLanguage::PlainText {
        return vec![CodeToken {
            start: 0,
            end: line.len(),
            kind: CodeTokenKind::Plain,
        }];
    }
    let syntax = syntax_for(language);
    let bytes = line.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        // A comment runs to the end of the line.
        // Compared as bytes, never as a string slice: `index` walks byte by byte, so it
        // can sit inside a multi-byte character, and slicing there panics. The hostile
        // input test caught exactly that.
        if !syntax.line_comment.is_empty()
            && bytes.len() >= index + syntax.line_comment.len()
            && &bytes[index..index + syntax.line_comment.len()] == syntax.line_comment.as_bytes()
        {
            tokens.push(CodeToken {
                start: index,
                end: bytes.len(),
                kind: CodeTokenKind::Comment,
            });
            break;
        }
        // A block comment inside one line.
        if syntax.block_comment && byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            let end = line[index + 2..]
                .find("*/")
                .map(|offset| index + 2 + offset + 2)
                .unwrap_or(bytes.len());
            tokens.push(CodeToken {
                start: index,
                end,
                kind: CodeTokenKind::Comment,
            });
            index = end;
            continue;
        }
        // A string literal, with a backslash escape.
        if syntax.string_quotes.contains(&byte) {
            let mut cursor = index + 1;
            while cursor < bytes.len() {
                match bytes[cursor] {
                    b'\\' => cursor += 2,
                    quote if quote == byte => {
                        cursor += 1;
                        break;
                    }
                    _ => cursor += 1,
                }
            }
            let end = cursor.min(bytes.len());
            tokens.push(CodeToken {
                start: index,
                end,
                kind: CodeTokenKind::String,
            });
            index = end;
            continue;
        }
        // A character literal or a lifetime: both are short and plain.
        if syntax.single_quote_is_plain && byte == b'\'' {
            let end = (index + 2).min(bytes.len());
            tokens.push(CodeToken {
                start: index,
                end,
                kind: CodeTokenKind::Plain,
            });
            index = end;
            continue;
        }
        if byte.is_ascii_digit() {
            let mut cursor = index;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric()
                    || bytes[cursor] == b'.'
                    || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            tokens.push(CodeToken {
                start: index,
                end: cursor,
                kind: CodeTokenKind::Number,
            });
            index = cursor;
            continue;
        }
        if byte.is_ascii_alphabetic() || byte == b'_' {
            let mut cursor = index;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            let word = &line[index..cursor];
            let kind = if syntax.keywords.contains(&word) {
                CodeTokenKind::Keyword
            } else if bytes.get(cursor) == Some(&b'(') {
                CodeTokenKind::Function
            } else {
                CodeTokenKind::Plain
            };
            tokens.push(CodeToken {
                start: index,
                end: cursor,
                kind,
            });
            index = cursor;
            continue;
        }
        // Punctuation and whitespace are plain, and adjacent plain runs are merged
        // below so a span list stays small.
        tokens.push(CodeToken {
            start: index,
            end: index + 1,
            kind: CodeTokenKind::Plain,
        });
        index += 1;
    }
    merge_plain(tokens)
}

/// Merges adjacent plain tokens, so a line of punctuation is one span rather than
/// dozens.
fn merge_plain(tokens: Vec<CodeToken>) -> Vec<CodeToken> {
    let mut merged: Vec<CodeToken> = Vec::with_capacity(tokens.len());
    for token in tokens {
        match merged.last_mut() {
            Some(previous)
                if previous.kind == CodeTokenKind::Plain
                    && token.kind == CodeTokenKind::Plain
                    && previous.end == token.start =>
            {
                previous.end = token.end;
            }
            _ => merged.push(token),
        }
    }
    merged
}

/// Builds a scene that draws a sample as highlighted lines.
///
/// The scene is ordinary canonical data: it validates, evaluates and rasterises
/// through the same paths as a hand-authored one. The duration is derived from the
/// line count so a longer sample holds longer, and the lines are placed with the
/// title above them.
pub fn scene_from_source(
    source: &[u8],
    language: CodeLanguage,
    options: &CodeSceneOptions,
) -> Result<MotionScene, CodeSceneError> {
    if source.len() > MAX_CODE_SOURCE_BYTES {
        return Err(CodeSceneError::SourceTooLarge);
    }
    let text = std::str::from_utf8(source).map_err(|_| CodeSceneError::InvalidUtf8)?;
    let lines = text
        .lines()
        .map(|line| clip_columns(line, MAX_CODE_COLUMNS))
        .collect::<Vec<_>>();
    if lines.len() > MAX_CODE_LINES {
        return Err(CodeSceneError::TooManyLines);
    }

    let theme = options.theme;
    let font_size = options.font_size.max(1.0);
    let line_height = font_size * 1.5;
    let margin_x = font_size * 1.5;
    let title_height = font_size * 2.5;
    let line_count = lines.len().max(1) as u32;
    let duration_ms = options
        .hold_milliseconds
        .saturating_add(line_count.saturating_mul(options.line_milliseconds));

    let mut nodes = vec![
        // The background is a node like any other, so it animates and composes with
        // the rest rather than being a special case in the renderer.
        MotionNode {
            node_id: "background".to_owned(),
            kind: MotionNodeKind::Rectangle,
            children: None,
            text: None,
            position: Some(MotionPoint { x: 0.0, y: 0.0 }),
            size: Some(MotionSize {
                width: options.canvas.width as f64,
                height: options.canvas.height as f64,
            }),
            end: None,
            fill: Some(theme.background),
            stroke: None,
            stroke_width: None,
            font_size: None,
            asset_id: None,
            opacity: None,
            rotation_degrees: None,
            spans: None,
        },
        MotionNode {
            node_id: "title".to_owned(),
            kind: MotionNodeKind::Text,
            children: None,
            text: Some(options.title.clone()),
            position: Some(MotionPoint {
                x: margin_x,
                y: margin_x,
            }),
            size: None,
            end: None,
            fill: Some(theme.foreground),
            stroke: None,
            stroke_width: None,
            font_size: Some(font_size * 1.3),
            asset_id: None,
            opacity: None,
            rotation_degrees: None,
            spans: None,
        },
    ];
    let mut animations = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        let tokens = tokenize_line(line, language);
        let character_ends = line
            .char_indices()
            .map(|(start, character)| start + character.len_utf8())
            .collect::<Vec<_>>();
        let duration_slots = usize::try_from(options.line_milliseconds / 2)
            .unwrap_or(1)
            .max(1);
        let step_count = character_ends
            .len()
            .clamp(1, MAX_CODE_REVEAL_STEPS_PER_LINE)
            .min(duration_slots);
        let line_start_ms = (index as u32)
            .saturating_mul(options.line_milliseconds)
            .min(duration_ms);

        for step_index in 0..step_count {
            let prefix_end = if character_ends.is_empty() {
                0
            } else {
                let characters_to_show = ((step_index + 1) * character_ends.len()) / step_count;
                character_ends[characters_to_show.saturating_sub(1)]
            };
            let prefix = &line[..prefix_end];
            let spans = tokens
                .iter()
                .filter(|token| token.kind != CodeTokenKind::Plain && token.start < prefix_end)
                .map(|token| MotionTextSpan {
                    start: token.start,
                    end: token.end.min(prefix_end),
                    color: token.kind.color(&theme),
                })
                .collect::<Vec<_>>();
            let node_id = format!("line-{index}-step-{step_index}");
            nodes.push(MotionNode {
                node_id: node_id.clone(),
                kind: MotionNodeKind::Text,
                children: None,
                text: Some(prefix.to_owned()),
                position: Some(MotionPoint {
                    x: margin_x,
                    y: margin_x + title_height + index as f64 * line_height,
                }),
                size: None,
                end: None,
                fill: Some(theme.foreground),
                stroke: None,
                stroke_width: None,
                font_size: Some(font_size),
                asset_id: None,
                opacity: None,
                rotation_degrees: None,
                spans: if spans.is_empty() { None } else { Some(spans) },
            });

            if character_ends.is_empty() {
                continue;
            }
            let reveal_at_ms = line_reveal_time_ms(
                index,
                step_index,
                step_count,
                options.line_milliseconds,
                duration_ms,
            );
            let next_reveal_at_ms = (step_index + 1 < step_count).then(|| {
                line_reveal_time_ms(
                    index,
                    step_index + 1,
                    step_count,
                    options.line_milliseconds,
                    duration_ms,
                )
            });
            if let Some(animation) =
                code_line_opacity_animation(node_id, line_start_ms, reveal_at_ms, next_reveal_at_ms)
            {
                animations.push(animation);
            }
        }
    }

    Ok(MotionScene {
        format: MOTION_SCENE_FORMAT.to_owned(),
        schema_version: MOTION_SCENE_SCHEMA_VERSION,
        scene_id: scene_id_for(source),
        name: options.title.clone(),
        duration: RationalTime::new(duration_ms as i64, 1000)
            .map_err(|_| CodeSceneError::InvalidUtf8)?,
        canvas: options.canvas,
        assets: Vec::new(),
        nodes,
        animations,
        camera: None,
    })
}

fn line_reveal_time_ms(
    line_index: usize,
    step_index: usize,
    step_count: usize,
    line_milliseconds: u32,
    duration_ms: u32,
) -> u32 {
    let line_start = (line_index as u32)
        .saturating_mul(line_milliseconds)
        .min(duration_ms);
    let step_offset = (u64::from(line_milliseconds) * step_index as u64 / step_count as u64)
        .min(u64::from(u32::MAX)) as u32;
    line_start.saturating_add(step_offset).min(duration_ms)
}

fn code_line_opacity_animation(
    node_id: String,
    line_start_ms: u32,
    reveal_at_ms: u32,
    next_reveal_at_ms: Option<u32>,
) -> Option<MotionAnimation> {
    let visible_from_start = line_start_ms == 0 && reveal_at_ms == 0;
    let mut keyframes = Vec::with_capacity(5);
    if visible_from_start {
        keyframes.push(opacity_keyframe(0, 1.0));
    } else {
        keyframes.push(opacity_keyframe(0, 0.0));
        let hidden_until_ms = reveal_at_ms.saturating_sub(CODE_REVEAL_TRANSITION_MILLISECONDS);
        if hidden_until_ms > 0 {
            keyframes.push(opacity_keyframe(hidden_until_ms, 0.0));
        }
        keyframes.push(opacity_keyframe(reveal_at_ms, 1.0));
    }

    if let Some(next_reveal_at_ms) = next_reveal_at_ms.filter(|next| *next > reveal_at_ms) {
        let transition_start_ms =
            next_reveal_at_ms.saturating_sub(CODE_REVEAL_TRANSITION_MILLISECONDS);
        if transition_start_ms > reveal_at_ms {
            keyframes.push(opacity_keyframe(transition_start_ms, 1.0));
        }
        keyframes.push(opacity_keyframe(next_reveal_at_ms, 0.0));
    }

    (keyframes.len() > 1).then_some(MotionAnimation {
        node_id,
        property: MotionProperty::Opacity,
        keyframes,
        easing: MotionEasing::Linear,
    })
}

fn opacity_keyframe(milliseconds: u32, value: f64) -> MotionKeyframe {
    MotionKeyframe {
        at: RationalTime::new(i64::from(milliseconds), 1000)
            .expect("millisecond times always have a nonzero denominator"),
        value,
    }
}

/// Clips a line to the column limit, on a character boundary, marking the cut so a
/// reader can see the line was clipped rather than assuming it ended.
fn clip_columns(line: &str, columns: usize) -> &str {
    if line.len() <= MAX_CODE_LINE_BYTES && line.chars().count() <= columns {
        return line;
    }
    let mut end = 0;
    for (count, (offset, character)) in line.char_indices().enumerate() {
        if count >= columns.saturating_sub(1) {
            break;
        }
        end = offset + character.len_utf8();
    }
    if end == 0 { "" } else { &line[..end] }
}

/// A stable scene identity derived from the sample, so the same source always
/// produces the same scene id. Deterministic and not random: two hosts rendering the
/// same sample must agree on the identity.
fn scene_id_for(source: &[u8]) -> String {
    // FNV-1a over the bytes, expanded into a UUIDv4-shaped string with the version
    // and variant bits set so it satisfies the format's own validator.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in source {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut second: u64 = hash;
    for byte in source.iter().rev() {
        second ^= *byte as u64;
        second = second.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (hash >> 32) as u32,
        (hash >> 16) as u16,
        (hash & 0x0fff) as u16,
        ((second >> 48) as u16 & 0x3fff) | 0x8000,
        second & 0xffff_ffff_ffff
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{evaluate_scene, inspect_motion_scene, validate_scene};

    const SAMPLE: &str =
        "fn main() {\n    let total = 41 + 1; // answer\n    println!(\"total {}\", total);\n}\n";

    fn scene() -> MotionScene {
        scene_from_source(
            SAMPLE.as_bytes(),
            CodeLanguage::Rust,
            &CodeSceneOptions::default(),
        )
        .unwrap()
    }

    fn full_line_node(scene: &MotionScene, line_index: usize) -> &MotionNode {
        let id_prefix = format!("line-{line_index}-step-");
        scene
            .nodes
            .iter()
            .filter(|node| node.node_id.starts_with(&id_prefix))
            .max_by_key(|node| node.text.as_ref().map_or(0, String::len))
            .expect("the source line has a text node")
    }

    #[test]
    fn a_sample_becomes_a_valid_typewritten_scene() {
        let scene = scene();
        assert!(
            validate_scene(&scene).ok,
            "{:?}",
            validate_scene(&scene).errors
        );
        let inspection = inspect_motion_scene(&scene);
        // Background, title, and up to eight snapshots for each of four source lines.
        assert_eq!(inspection.node_count, 27);
        assert_eq!(inspection.canvas_width, 1920);
        assert_eq!(inspection.duration_milliseconds, 2000 + 4 * 400);
        assert_eq!(scene.nodes[1].text.as_deref(), Some("Code"));
        assert_eq!(
            full_line_node(&scene, 0).text.as_deref(),
            Some("fn main() {")
        );
        assert_eq!(scene.animations.len(), 25);
    }

    #[test]
    fn code_lines_reveal_in_deterministic_steps_over_their_line_interval() {
        let options = CodeSceneOptions {
            hold_milliseconds: 0,
            ..CodeSceneOptions::default()
        };
        let scene = scene_from_source(b"abcdefgh", CodeLanguage::PlainText, &options).unwrap();
        let visible_prefix = |milliseconds| {
            evaluate_scene(&scene, RationalTime::new(milliseconds, 1000).unwrap())
                .unwrap()
                .instructions
                .into_iter()
                .find(|instruction| {
                    instruction.node_id.starts_with("line-0-step-") && instruction.opacity > 0.99
                })
                .and_then(|instruction| instruction.text)
                .expect("one typewriter snapshot is fully visible")
        };

        assert_eq!(visible_prefix(0), "a");
        assert_eq!(visible_prefix(100), "abc");
        assert_eq!(visible_prefix(399), "abcdefgh");
    }

    #[test]
    fn the_maximum_source_stays_within_motion_scene_node_and_animation_limits() {
        let line = "x".repeat(MAX_CODE_COLUMNS);
        let source = (0..MAX_CODE_LINES)
            .map(|_| line.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let scene = scene_from_source(
            source.as_bytes(),
            CodeLanguage::PlainText,
            &CodeSceneOptions::default(),
        )
        .unwrap();

        assert_eq!(
            scene.nodes.len(),
            2 + MAX_CODE_LINES * MAX_CODE_REVEAL_STEPS_PER_LINE
        );
        assert_eq!(
            scene.animations.len(),
            MAX_CODE_LINES * MAX_CODE_REVEAL_STEPS_PER_LINE
        );
        assert!(scene.nodes.len() <= crate::MAX_MOTION_NODES);
        assert!(scene.animations.len() <= crate::MAX_MOTION_ANIMATIONS);
        assert!(
            scene
                .animations
                .iter()
                .all(|animation| animation.keyframes.len() <= crate::MAX_MOTION_KEYFRAMES)
        );
    }

    #[test]
    fn the_same_sample_always_produces_the_same_scene_identity() {
        // Deterministic, not random: two hosts rendering the same sample must agree.
        assert_eq!(scene().scene_id, scene().scene_id);
        let other = scene_from_source(
            b"fn other() {}",
            CodeLanguage::Rust,
            &CodeSceneOptions::default(),
        )
        .unwrap();
        assert_ne!(scene().scene_id, other.scene_id);
        // The identity satisfies the format's own scene_id rule, which is what the
        // validator checks.
        let mut scene_value = scene();
        assert!(validate_scene(&scene_value).ok);
        scene_value.scene_id = "not-a-uuid".to_owned();
        assert!(!validate_scene(&scene_value).ok);
    }

    #[test]
    fn keywords_strings_comments_numbers_and_calls_get_their_own_spans() {
        let scene = scene();
        let line = |index: usize| full_line_node(&scene, index).clone();
        // `fn main() {`: `fn` is a keyword, `main` is a call shape.
        let first = line(0);
        let spans = first.spans.clone().unwrap();
        assert_eq!(spans[0].color, CodeTheme::OR_DARK.keyword);
        assert_eq!(
            &first.text.as_ref().unwrap()[spans[0].start..spans[0].end],
            "fn"
        );
        // `let total = 41 + 1; // answer`: keyword, number and a trailing comment.
        let second = line(1);
        let text = second.text.clone().unwrap();
        let kinds = second
            .spans
            .clone()
            .unwrap()
            .iter()
            .map(|span| {
                let color = span.color;
                let value = &text[span.start..span.end];
                (value.to_owned(), color)
            })
            .collect::<Vec<_>>();
        assert!(
            kinds
                .iter()
                .any(|(value, color)| value == "let" && *color == CodeTheme::OR_DARK.keyword)
        );
        assert!(
            kinds
                .iter()
                .any(|(value, color)| value == "41" && *color == CodeTheme::OR_DARK.number)
        );
        assert!(
            kinds
                .iter()
                .any(|(value, color)| value.starts_with("//")
                    && *color == CodeTheme::OR_DARK.comment)
        );
        // The string on the third line is a string, not plain.
        let third = line(2);
        assert!(
            third
                .spans
                .as_ref()
                .unwrap()
                .iter()
                .any(|span| span.color == CodeTheme::OR_DARK.string)
        );
    }

    #[test]
    fn spans_are_ordered_non_overlapping_and_inside_the_line() {
        let scene = scene();
        for node in scene.nodes.iter().skip(2) {
            let text = node.text.as_ref().unwrap();
            let mut previous_end = 0;
            for span in node.spans.as_deref().unwrap_or_default() {
                assert!(span.start < span.end, "{span:?}");
                assert!(span.end <= text.len(), "{span:?} {text:?}");
                assert!(
                    span.start >= previous_end,
                    "spans must not overlap: {span:?}"
                );
                previous_end = span.end;
                // Every boundary is on a character boundary, so the slice is valid.
                assert!(text.is_char_boundary(span.start) && text.is_char_boundary(span.end));
            }
        }
    }

    #[test]
    fn plain_text_is_one_span_and_never_a_guess() {
        let tokens = tokenize_line("fn main() { // not rust here }", CodeLanguage::PlainText);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, CodeTokenKind::Plain);
        assert_eq!(tokens[0].end, "fn main() { // not rust here }".len());
    }

    #[test]
    fn python_source_tokenises_its_own_syntax() {
        assert_eq!(CodeLanguage::from_path("tool.py"), CodeLanguage::Python);
        assert_eq!(CodeLanguage::from_path("tool.pyi"), CodeLanguage::Python);
        assert_eq!(CodeLanguage::from_path("tool.js"), CodeLanguage::PlainText);

        // A `#` comment runs to the end of the line, and `//` is not a comment.
        let comment = tokenize_line("total = 1  # running total", CodeLanguage::Python);
        let last = comment.last().unwrap();
        assert_eq!(last.kind, CodeTokenKind::Comment);
        assert_eq!(
            &"total = 1  # running total"[last.start..last.end],
            "# running total"
        );
        let not_comment = tokenize_line("total = 1 // 2", CodeLanguage::Python);
        assert!(
            not_comment
                .iter()
                .all(|token| token.kind != CodeTokenKind::Comment)
        );

        // Single and double quotes are both strings, with escapes.
        let single = tokenize_line("name = 'or\\'s'", CodeLanguage::Python);
        assert!(
            single
                .iter()
                .any(|token| token.kind == CodeTokenKind::String)
        );
        let double = tokenize_line("name = \"or\"", CodeLanguage::Python);
        assert!(
            double
                .iter()
                .any(|token| token.kind == CodeTokenKind::String)
        );

        // Keywords and a defined name, which is what a reader looks for first.
        let defined = tokenize_line("def render(scene):", CodeLanguage::Python);
        assert!(defined.iter().any(|token| {
            token.kind == CodeTokenKind::Keyword
                && &"def render(scene):"[token.start..token.end] == "def"
        }));
        assert!(defined.iter().any(|token| {
            token.kind == CodeTokenKind::Function
                && &"def render(scene):"[token.start..token.end] == "render"
        }));
        // A Rust keyword is not a Python keyword.
        let rust_only = tokenize_line("let impl = 1", CodeLanguage::Python);
        assert!(
            rust_only
                .iter()
                .all(|token| token.kind != CodeTokenKind::Keyword)
        );

        // A triple quote is a string to the end of the line; the tokeniser has no state
        // between lines and does not pretend to.
        let docstring = tokenize_line("\"\"\"A module docstring.\"\"\"", CodeLanguage::Python);
        assert_eq!(docstring[0].kind, CodeTokenKind::String);
    }

    #[test]
    fn python_source_builds_a_scene_with_its_own_highlighting() {
        let source = b"def total(values):\n    return sum(values)  # add them\n";
        let scene = scene_from_source(source, CodeLanguage::Python, &CodeSceneOptions::default())
            .expect("python source builds a scene");
        // Read the full source line snapshots rather than concatenating the reveal
        // prefixes that are also stored in the scene.
        let source_line_count = std::str::from_utf8(source).unwrap().lines().count();
        let text = (0..source_line_count)
            .filter_map(|index| full_line_node(&scene, index).text.clone())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            text.contains("def"),
            "the source text is in the scene: {text}"
        );
        assert!(text.contains("# add them"), "the comment is in the scene");
    }

    #[test]
    fn a_hostile_sample_cannot_make_the_tokeniser_loop_or_the_scene_unbounded() {
        // Unterminated constructs, a huge line and non-ASCII text are all handled.
        for sample in [
            "fn main() { let s = \"unterminated",
            "/* unterminated block",
            "'a",
            "0x",
            "let 日本 = 1; // コメント",
        ] {
            let tokens = tokenize_line(sample, CodeLanguage::Rust);
            if let Some(last) = tokens.last() {
                assert_eq!(last.end, sample.len(), "tokens must cover the whole line");
            }
            // One token per byte is the worst case, and nothing may exceed it.
            assert!(tokens.len() <= sample.len().max(1));
        }
        let too_many = "line\n".repeat(MAX_CODE_LINES + 1);
        assert_eq!(
            scene_from_source(
                too_many.as_bytes(),
                CodeLanguage::Rust,
                &CodeSceneOptions::default()
            )
            .unwrap_err(),
            CodeSceneError::TooManyLines
        );
        let too_large = vec![b'a'; MAX_CODE_SOURCE_BYTES + 1];
        assert_eq!(
            scene_from_source(&too_large, CodeLanguage::Rust, &CodeSceneOptions::default())
                .unwrap_err(),
            CodeSceneError::SourceTooLarge
        );
        assert_eq!(
            scene_from_source(
                &[0xff, 0xfe],
                CodeLanguage::Rust,
                &CodeSceneOptions::default()
            )
            .unwrap_err(),
            CodeSceneError::InvalidUtf8
        );
        assert_eq!(
            CodeSceneError::SourceTooLarge.code(),
            "CODE_SOURCE_TOO_LARGE"
        );
    }

    #[test]
    fn a_long_line_is_clipped_rather_than_widening_the_scene() {
        let long = "x".repeat(MAX_CODE_COLUMNS + 50);
        let scene = scene_from_source(
            long.as_bytes(),
            CodeLanguage::Rust,
            &CodeSceneOptions::default(),
        )
        .unwrap();
        let text = full_line_node(&scene, 0).text.clone().unwrap();
        assert!(
            text.chars().count() <= MAX_CODE_COLUMNS,
            "{}",
            text.chars().count()
        );
    }

    #[test]
    fn the_language_comes_from_the_file_name_and_defaults_to_plain_text() {
        assert_eq!(CodeLanguage::from_path("src/main.rs"), CodeLanguage::Rust);
        assert_eq!(
            CodeLanguage::from_path("NOTES.TXT"),
            CodeLanguage::PlainText
        );
        assert_eq!(
            CodeLanguage::from_path("no-extension"),
            CodeLanguage::PlainText
        );
        assert_eq!(CodeLanguage::Rust.name(), "rust");
    }

    #[test]
    fn a_sample_of_no_lines_still_produces_a_valid_scene() {
        let scene =
            scene_from_source(b"", CodeLanguage::Rust, &CodeSceneOptions::default()).unwrap();
        assert!(validate_scene(&scene).ok);
        // The background and the title, and no line nodes.
        assert_eq!(scene.nodes.len(), 2);
        assert_eq!(scene.duration, RationalTime::new(2000 + 400, 1000).unwrap());
    }
}
