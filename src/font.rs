/*
 *  Copyright 2021 QuantumBadger
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use std::collections::VecDeque;
use std::convert::TryInto;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::Peekable;
use std::ops::Deref;
use std::rc::Rc;
use std::slice::Iter;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::vec::IntoIter;

use rusttype::Scale;
use unicode_normalization::UnicodeNormalization;

use crate::dimen::Vector2;
use crate::error::{BacktraceError, ErrorMessage};
use crate::shape::Rectangle;

static FONT_ID_GENERATOR: AtomicUsize = AtomicUsize::new(10000);

/// Type returned by the [FormattedGlyph::user_index()] function.
///
/// The `user_index` field allows you to determine which output glyph
/// corresponds to which input codepoint.
pub type UserGlyphIndex = u32;

/// An internal identifier for a font. Each font which is loaded receives a
/// unique ID.
pub type FontId = usize;

/// A struct representing a Unicode codepoint, for the purposes of text layout.
/// The `user_index` field allows you to determine which output glyph
/// corresponds to which input codepoint.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Codepoint
{
    user_index: UserGlyphIndex,
    codepoint: char
}

impl Codepoint
{
    /// The Unicode codepoint for a zero width space. You may use this to denote
    /// places where it would be appropriate to insert a line break when
    /// wrapping.
    pub const ZERO_WIDTH_SPACE: char = '\u{200B}';

    /// Instantiates a new `Codepoint`. The value provided for `user_index` will
    /// be present in the corresponding `FormattedGlyph` object returned
    /// during layout.
    #[inline]
    #[must_use]
    pub fn new(user_index: UserGlyphIndex, codepoint: char) -> Self
    {
        Codepoint {
            user_index,
            codepoint
        }
    }

    fn from_unindexed_codepoints(unindexed_codepoints: &[char]) -> Vec<Self>
    {
        let mut codepoints = Vec::new();
        codepoints.reserve(unindexed_codepoints.len());

        for (i, codepoint) in unindexed_codepoints.iter().enumerate() {
            codepoints.push(Codepoint::new(i.try_into().unwrap(), *codepoint));
        }

        codepoints
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
struct RenderableWord
{
    codepoints: Vec<Codepoint>,
    is_whitespace: bool
}

impl RenderableWord
{
    fn starting_from_codepoint_location(mut self, location: usize) -> Self
    {
        self.codepoints.drain(0..location);

        RenderableWord {
            codepoints: self.codepoints,
            is_whitespace: self.is_whitespace
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
enum Word
{
    Renderable(RenderableWord),
    Newline
}

impl Word
{
    fn split_words(codepoints: &[Codepoint]) -> Vec<Word>
    {
        let mut reader = codepoints.iter().peekable();

        let mut result = Vec::new();

        while let Some(first_token) = reader.next() {
            match first_token.codepoint {
                Codepoint::ZERO_WIDTH_SPACE | '\r' => {
                    // Do nothing here, just ignore it
                }

                '\n' => result.push(Word::Newline),

                ' ' | '\t' => {
                    result.push(Word::Renderable(RenderableWord {
                        codepoints: vec![first_token.clone()],
                        is_whitespace: true
                    }));
                }

                _ => {
                    // Non-whitespace word

                    let mut word_codepoints = Vec::new();
                    word_codepoints.reserve(16);
                    word_codepoints.push(first_token.clone());

                    while let Some(next) = reader.peek() {
                        match next.codepoint {
                            ' ' | '\t' | '\r' | '\n' | Codepoint::ZERO_WIDTH_SPACE => {
                                break
                            }
                            _ => word_codepoints.push(reader.next().unwrap().clone())
                        }
                    }

                    result.push(Word::Renderable(RenderableWord {
                        codepoints: word_codepoints,
                        is_whitespace: false
                    }));
                }
            }
        }

        result
    }
}

/// A struct representing a glyph in a font.
pub struct FontGlyph
{
    glyph: rusttype::Glyph<'static>,
    font: Font
}

struct WordsIterator
{
    words: Peekable<IntoIter<Word>>,
    pending: VecDeque<Word>
}

impl WordsIterator
{
    fn from(words: Vec<Word>) -> Self
    {
        WordsIterator {
            words: words.into_iter().peekable(),
            pending: VecDeque::new()
        }
    }

    #[inline]
    #[must_use]
    fn has_next(&self) -> bool
    {
        self.words.len() > 0 || !self.pending.is_empty()
    }

    #[inline]
    #[must_use]
    fn peek(&mut self) -> Option<&Word>
    {
        if let Some(word) = self.pending.front() {
            return Some(word);
        }

        if let Some(word) = self.words.peek() {
            return Some(word);
        }

        None
    }

    #[inline]
    fn next(&mut self) -> Option<Word>
    {
        if let Some(word) = self.pending.pop_front() {
            return Some(word);
        }

        if let Some(word) = self.words.next() {
            return Some(word);
        }

        None
    }

    #[inline]
    fn add_pending(&mut self, word: Word)
    {
        self.pending.push_back(word);
    }
}

#[derive(Clone, Debug)]
struct LineLayoutMetrics
{
    x_pos: f32,
    max_ascent: f32,
    min_descent: f32,
    max_line_gap: f32,
    last_glyph_id: Option<rusttype::GlyphId>,
    last_font_id: Option<FontId>
}

impl LineLayoutMetrics
{
    fn new() -> Self
    {
        LineLayoutMetrics {
            x_pos: 0.0,
            max_ascent: 0.0,
            min_descent: 0.0,
            max_line_gap: 0.0,
            last_glyph_id: None,
            last_font_id: None
        }
    }

    #[inline]
    #[must_use]
    fn height(&self) -> f32
    {
        self.max_ascent - self.min_descent
    }

    fn update_and_get_render_pos_x(
        &mut self,
        glyph: &rusttype::ScaledGlyph,
        font_id: FontId,
        scale: &rusttype::Scale,
        options: &TextOptions
    ) -> f32
    {
        if let Some(last_glyph_id) = self.last_glyph_id {
            if self.last_font_id == Some(font_id) {
                self.x_pos +=
                    glyph.font().pair_kerning(*scale, last_glyph_id, glyph.id());
            }

            self.x_pos += options.tracking;
        }

        if self.last_font_id != Some(font_id) {
            let v_metrics = glyph.font().v_metrics(*scale);

            self.max_ascent = crate::numeric::max(self.max_ascent, v_metrics.ascent);
            self.min_descent = crate::numeric::min(self.min_descent, v_metrics.descent);
            self.max_line_gap =
                crate::numeric::max(self.max_line_gap, v_metrics.line_gap);
        }

        let advance_width = glyph.h_metrics().advance_width;

        let glyph_x_pos_start = self.x_pos;
        self.x_pos += advance_width;

        self.last_font_id = Some(font_id);
        self.last_glyph_id = Some(glyph.id());

        glyph_x_pos_start
    }
}

enum WordLayoutResult
{
    Success(LineLayoutMetrics),
    PartialWord(LineLayoutMetrics),
    NotEnoughSpace
}

impl WordLayoutResult
{
    fn get_metrics(&self) -> Option<&LineLayoutMetrics>
    {
        match self {
            WordLayoutResult::Success(metrics) => Some(metrics),
            WordLayoutResult::PartialWord(metrics) => Some(metrics),
            WordLayoutResult::NotEnoughSpace => None
        }
    }

    fn end_of_line(&self) -> bool
    {
        match self {
            WordLayoutResult::Success(_) => false,
            WordLayoutResult::PartialWord(_) => true,
            WordLayoutResult::NotEnoughSpace => true
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn try_layout_word_internal<T: TextLayout>(
    layout_helper: &T,
    word: RenderableWord,
    remaining_words: &mut WordsIterator,
    scale: &rusttype::Scale,
    options: &TextOptions,
    pos_y_baseline: f32,
    first_word_on_line: bool,
    previous_metrics: &LineLayoutMetrics,
    output: &mut Vec<FormattedGlyph>
) -> WordLayoutResult
{
    let mut new_word_metrics = previous_metrics.clone();
    let pos_x_max = options.wrap_words_after_width;

    let mut glyphs = Vec::new();

    for (
        i,
        Codepoint {
            user_index,
            codepoint: c
        }
    ) in word.codepoints.iter().enumerate()
    {
        // We can't modify the actual values until we're sure we can render this glyph
        let mut new_glyph_metrics = new_word_metrics.clone();

        let glyph = match layout_helper.lookup_glyph_for_codepoint(*c) {
            None => {
                match layout_helper
                    .lookup_glyph_for_codepoint('□')
                    .or_else(|| layout_helper.lookup_glyph_for_codepoint('?'))
                {
                    None => continue,
                    Some(glyph) => glyph
                }
            }
            Some(glyph) => glyph
        };

        let scaled_glyph = glyph.glyph.scaled(*scale);

        let glyph_x_pos_start = new_glyph_metrics.update_and_get_render_pos_x(
            &scaled_glyph,
            glyph.font.id(),
            scale,
            options
        );

        let formatted_glyph = FormattedGlyph {
            user_index: *user_index,
            glyph: scaled_glyph.positioned(rusttype::point(glyph_x_pos_start, 0.0)),
            font_id: glyph.font.id()
        };

        if let Some(pos_x_max) = pos_x_max {
            if new_glyph_metrics.x_pos > pos_x_max {
                if first_word_on_line {
                    if i == 0 {
                        // First glyph in word, we should render it even though it goes
                        // over the boundary
                        glyphs.push(formatted_glyph);
                        new_word_metrics = new_glyph_metrics;

                        // If there are more codepoints, we need to split the word
                        if word.codepoints.len() > 1 {
                            remaining_words.add_pending(Word::Renderable(
                                word.starting_from_codepoint_location(i + 1)
                            ));
                        }
                    } else {
                        remaining_words.add_pending(Word::Renderable(
                            word.starting_from_codepoint_location(i)
                        ));
                    }

                    glyphs.iter_mut().for_each(|glyph| {
                        glyph.reposition_y(pos_y_baseline + new_word_metrics.max_ascent);
                    });

                    output.append(&mut glyphs);
                    return WordLayoutResult::PartialWord(new_word_metrics);
                } else {
                    remaining_words.add_pending(Word::Renderable(word));
                    return WordLayoutResult::NotEnoughSpace;
                }
            }
        }

        glyphs.push(formatted_glyph);
        new_word_metrics = new_glyph_metrics;
    }

    glyphs.iter_mut().for_each(|glyph| {
        glyph.reposition_y(pos_y_baseline + new_word_metrics.max_ascent);
    });

    output.append(&mut glyphs);

    WordLayoutResult::Success(new_word_metrics)
}

fn layout_line_internal<T: TextLayout>(
    layout_helper: &T,
    words: &mut WordsIterator,
    scale: &rusttype::Scale,
    options: &TextOptions,
    pos_y_baseline: f32
) -> FormattedTextLine
{
    let mut line_metrics = LineLayoutMetrics::new();
    let mut glyphs = Vec::new();

    let mut first_word_on_line = true;

    // Skip whitespace
    while let Some(Word::Renderable(word)) = words.peek() {
        if word.is_whitespace {
            words.next().unwrap();
        } else {
            break;
        }
    }

    while let Some(Word::Renderable(word)) = words.next() {
        let result = try_layout_word_internal(
            layout_helper,
            word,
            words,
            scale,
            options,
            pos_y_baseline,
            first_word_on_line,
            &line_metrics,
            &mut glyphs
        );

        if let Some(metrics) = result.get_metrics() {
            line_metrics = metrics.clone();
        }

        if result.end_of_line() {
            break;
        }

        first_word_on_line = false;
    }

    if glyphs.is_empty() {
        let empty_metrics = layout_helper.empty_line_vertical_metrics(scale.y);
        line_metrics.max_ascent = empty_metrics.ascent;
        line_metrics.min_descent = empty_metrics.descent;
        line_metrics.max_line_gap = empty_metrics.line_gap;
    }

    FormattedTextLine {
        glyphs,
        baseline_vertical_position: pos_y_baseline,
        width: line_metrics.x_pos,
        height: line_metrics.height(),
        ascent: line_metrics.max_ascent,
        descent: line_metrics.min_descent,
        line_gap: line_metrics.max_line_gap
    }
}

fn layout_multiple_lines_internal<T: TextLayout>(
    layout_helper: &T,
    codepoints: &[Codepoint],
    scale: f32,
    options: TextOptions
) -> Rc<FormattedTextBlock>
{
    let scale = rusttype::Scale::uniform(scale);

    let mut iterator = WordsIterator::from(Word::split_words(codepoints));

    let mut pos_y = 0.0;
    let mut lines = Vec::new();

    let mut width = 0.0;

    while iterator.has_next() {
        let mut line =
            layout_line_internal(layout_helper, &mut iterator, &scale, &options, pos_y);

        if let Some(max_width) = options.wrap_words_after_width {
            match options.alignment {
                TextAlignment::Left => {}
                TextAlignment::Center => {
                    line.add_offset_x((max_width - line.width) / 2.0)
                }
                TextAlignment::Right => line.add_offset_x(max_width - line.width)
            }
        }

        pos_y += line.height * options.line_spacing_multiplier;

        if iterator.has_next() {
            pos_y += line.line_gap * options.line_spacing_multiplier;
        }

        width = crate::numeric::max(width, line.width);

        lines.push(Rc::new(line));
    }

    Rc::new(FormattedTextBlock {
        lines,
        width,
        height: pos_y
    })
}

/// The vertical metrics of a line of text.
#[derive(Debug, Clone, PartialEq)]
pub struct LineVerticalMetrics
{
    /// The ascent of the line in pixels.
    ascent: f32,
    /// The descent of the line in pixels.
    descent: f32,
    /// The gap between this line and the next line, in pixels.
    line_gap: f32
}

impl LineVerticalMetrics
{
    /// The height of the line in pixels.
    pub fn height(&self) -> f32
    {
        self.ascent - self.descent
    }
}

/// Objects implementing this trait are able to lay out text, ready for
/// rendering.
pub trait TextLayout
{
    /// Returns the glyph corresponding to the provided codepoint. If the glyph
    /// cannot be found, `None` is returned.
    fn lookup_glyph_for_codepoint(&self, codepoint: char) -> Option<FontGlyph>;

    /// Lays out a block of text with the specified scale and options. The
    /// result may be passed to `Graphics2D::draw_text`.
    ///
    /// As the string undergoes normalization before being laid out, the
    /// `user_index` of each `FormattedGlyph` is undefined. To gain control
    /// over the `user_index` field, consider using
    /// either `layout_text_line_from_codepoints()` or
    /// `layout_text_line_from_unindexed_codepoints()`.
    #[inline]
    #[must_use]
    fn layout_text(
        &self,
        text: &str,
        scale: f32,
        options: TextOptions
    ) -> Rc<FormattedTextBlock>
    where
        Self: Sized
    {
        let codepoints: Vec<char> = text.nfc().collect();
        self.layout_text_from_unindexed_codepoints(codepoints.as_slice(), scale, options)
    }

    /// Lays out a block of text with the specified scale and options. The
    /// result may be passed to `Graphics2D::draw_text`.
    ///
    /// The `user_index` field of each `FormattedGlyph` will be set to the
    /// location of the input codepoint in `unindexed_codepoints`, starting
    /// from zero.
    #[inline]
    #[must_use]
    fn layout_text_from_unindexed_codepoints(
        &self,
        unindexed_codepoints: &[char],
        scale: f32,
        options: TextOptions
    ) -> Rc<FormattedTextBlock>
    where
        Self: Sized
    {
        self.layout_text_from_codepoints(
            Codepoint::from_unindexed_codepoints(unindexed_codepoints).as_slice(),
            scale,
            options
        )
    }

    /// Lays out a block of text with the specified scale and options. The
    /// result may be passed to `Graphics2D::draw_text`.
    ///
    /// The `user_index` field of each `FormattedGlyph` will be set to the
    /// `user_index` of the corresponding `Codepoint`.
    #[must_use]
    fn layout_text_from_codepoints(
        &self,
        codepoints: &[Codepoint],
        scale: f32,
        options: TextOptions
    ) -> Rc<FormattedTextBlock>
    where
        Self: Sized
    {
        layout_multiple_lines_internal(self, codepoints, scale, options)
    }

    /// The default metrics of a line which contains no characters.
    #[must_use]
    fn empty_line_vertical_metrics(&self, scale: f32) -> LineVerticalMetrics;
}

/// A struct representing a font.
#[repr(transparent)]
#[derive(Clone)]
pub struct Font
{
    data: Rc<FontImpl>
}

#[derive(Debug)]
struct FontImpl
{
    id: usize,
    font: rusttype::Font<'static>
}

impl Font
{
    /// Constructs a new font from the specified bytes.
    ///
    /// The font may be in TrueType or OpenType format. Support for OpenType
    /// fonts may be limited.
    pub fn new(bytes: &[u8]) -> Result<Font, BacktraceError<ErrorMessage>>
    {
        let font = rusttype::Font::try_from_vec(bytes.to_vec())
            .ok_or_else(|| ErrorMessage::msg("Failed to load font"))?;

        Ok(Font {
            data: Rc::new(FontImpl {
                id: FONT_ID_GENERATOR.fetch_add(1, Ordering::SeqCst),
                font
            })
        })
    }

    #[inline]
    fn id(&self) -> usize
    {
        self.data.id
    }

    #[inline]
    fn font(&self) -> &rusttype::Font<'static>
    {
        &self.data.font
    }
}

impl TextLayout for FontFamily
{
    fn lookup_glyph_for_codepoint(&self, codepoint: char) -> Option<FontGlyph>
    {
        for font in &*self.fonts {
            if let Some(glyph) = font.lookup_glyph_for_codepoint(codepoint) {
                return Some(glyph);
            }
        }

        None
    }

    fn empty_line_vertical_metrics(&self, scale: f32) -> LineVerticalMetrics
    {
        match Rc::deref(&self.fonts).first() {
            None => LineVerticalMetrics {
                ascent: 0.0,
                descent: 0.0,
                line_gap: 0.0
            },
            Some(font) => {
                let metrics = font.data.font.v_metrics(Scale::uniform(scale));
                LineVerticalMetrics {
                    ascent: metrics.ascent,
                    descent: metrics.descent,
                    line_gap: metrics.line_gap
                }
            }
        }
    }
}

impl TextLayout for Font
{
    fn lookup_glyph_for_codepoint(&self, codepoint: char) -> Option<FontGlyph>
    {
        let glyph = self.font().glyph(codepoint);

        if glyph.id().0 == 0 {
            None
        } else {
            Some(FontGlyph {
                glyph,
                font: self.clone()
            })
        }
    }

    fn empty_line_vertical_metrics(&self, scale: f32) -> LineVerticalMetrics
    {
        let metrics = self.data.font.v_metrics(Scale::uniform(scale));
        LineVerticalMetrics {
            ascent: metrics.ascent,
            descent: metrics.descent,
            line_gap: metrics.line_gap
        }
    }
}

impl PartialEq for Font
{
    #[inline]
    fn eq(&self, other: &Self) -> bool
    {
        self.id() == other.id()
    }
}

impl Eq for Font {}

impl Hash for Font
{
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H)
    {
        self.id().hash(state);
    }
}

impl Debug for Font
{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result
    {
        f.debug_struct("Font").field("id", &self.id()).finish()
    }
}

/// A collection of fonts, in decreasing order of priority. When laying out
/// text, if a codepoint cannot be found in the first font in the list, the
/// subsequent fonts will also be searched.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontFamily
{
    fonts: Rc<Vec<Font>>
}

impl FontFamily
{
    /// Instantiates a new font family, containing the specified fonts in
    /// decreasing order of priority.
    #[must_use]
    pub fn new(fonts: Vec<Font>) -> Self
    {
        FontFamily {
            fonts: Rc::new(fonts)
        }
    }
}

/// The horizontal alignment of a block of text. This can be set when calling
/// `TextOptions::with_wrap_words_after_width`.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum TextAlignment
{
    /// Align the text to the left.
    Left,
    /// Center the text in the maximum width.
    Center,
    /// Align the text to the rightmost point within the maximum width.
    Right
}

/// A series of options for specifying how text should be laid out.
pub struct TextOptions
{
    tracking: f32,
    wrap_words_after_width: Option<f32>,
    alignment: TextAlignment,
    line_spacing_multiplier: f32
}

impl TextOptions
{
    /// Instantiates a new `TextOptions` with the default settings.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        TextOptions {
            tracking: 0.0,
            wrap_words_after_width: None,
            alignment: TextAlignment::Left,
            line_spacing_multiplier: 1.0
        }
    }

    /// Sets the tracking of the font. This is the amount of extra space (in
    /// pixels) to put between each character.
    ///
    /// The default is `0.0`.
    #[inline]
    #[must_use]
    pub fn with_tracking(mut self, tracking: f32) -> Self
    {
        self.tracking = tracking;
        self
    }

    /// Limits the width of the text block to the specified pixel value,
    /// wrapping words to a new line if they exceed that limit.
    ///
    /// This function also sets the alignment, within the specified width.
    ///
    /// The default is to not wrap text.
    #[inline]
    #[must_use]
    pub fn with_wrap_to_width(
        mut self,
        wrap_words_after_width_px: f32,
        alignment: TextAlignment
    ) -> Self
    {
        self.wrap_words_after_width = Some(wrap_words_after_width_px);
        self.alignment = alignment;
        self
    }

    /// Sets the amount of space between each line of text. The gap between the
    /// baseline of each line of text is multiplied by this value.
    ///
    /// The default is `1.0`.
    #[inline]
    #[must_use]
    pub fn with_line_spacing_multiplier(mut self, line_spacing_multiplier: f32) -> Self
    {
        self.line_spacing_multiplier = line_spacing_multiplier;
        self
    }
}

impl Default for TextOptions
{
    fn default() -> Self
    {
        Self::new()
    }
}

/// Represents a glyph which has been laid out as part of a line of text.
#[derive(Clone)]
pub struct FormattedGlyph
{
    glyph: rusttype::PositionedGlyph<'static>,
    font_id: FontId,
    user_index: UserGlyphIndex
}

impl FormattedGlyph
{
    #[inline]
    #[must_use]
    pub(crate) fn glyph(&self) -> &rusttype::PositionedGlyph<'static>
    {
        &self.glyph
    }

    /// The identifier of the font which was used to render this glyph.
    #[inline]
    #[must_use]
    pub fn font_id(&self) -> FontId
    {
        self.font_id
    }

    /// The `user_index` of the corresponding `Codepoint`. This allows you to
    /// identify which input `Codepoint` corresponds to the output
    /// `FormattedGlyph`.
    #[inline]
    #[must_use]
    pub fn user_index(&self) -> UserGlyphIndex
    {
        self.user_index
    }

    #[inline]
    fn reposition_y(&mut self, y_pos: f32)
    {
        let existing_pos = self.glyph.position();
        self.glyph
            .set_position(rusttype::point(existing_pos.x, y_pos));
    }

    #[inline]
    fn add_offset_x(&mut self, offset_x: f32)
    {
        let existing_pos = self.glyph.position();
        self.glyph
            .set_position(rusttype::point(existing_pos.x + offset_x, existing_pos.y));
    }
}

/// Represents a block of text which has been laid out.
pub struct FormattedTextBlock
{
    lines: Vec<Rc<FormattedTextLine>>,
    width: f32,
    height: f32
}

impl FormattedTextBlock
{
    /// Iterate over the lines of text in this block.
    #[inline]
    #[must_use]
    pub fn iter_lines(&self) -> Iter<'_, Rc<FormattedTextLine>>
    {
        self.lines.iter()
    }

    /// The width (in pixels) of this text block.
    #[inline]
    #[must_use]
    pub fn width(&self) -> f32
    {
        self.width
    }

    /// The height (in pixels) of this text block.
    #[inline]
    #[must_use]
    pub fn height(&self) -> f32
    {
        self.height
    }

    /// The size (in pixels) of this text block.
    #[inline]
    #[must_use]
    pub fn size(&self) -> Vector2<f32>
    {
        Vector2::new(self.width, self.height)
    }
}

/// Represents a line of text which has been laid out as part of a block.
pub struct FormattedTextLine
{
    glyphs: Vec<FormattedGlyph>,
    baseline_vertical_position: f32,
    width: f32,
    height: f32,
    ascent: f32,
    descent: f32,
    line_gap: f32
}

impl FormattedTextLine
{
    /// Iterate over the glyphs in this line.
    #[inline]
    #[must_use]
    pub fn iter_glyphs(&self) -> Iter<'_, FormattedGlyph>
    {
        self.glyphs.iter()
    }

    /// The width (in pixels) of this text line.
    #[inline]
    #[must_use]
    pub fn width(&self) -> f32
    {
        self.width
    }

    /// The height (in pixels) of this text line. This is equal to the
    /// `ascent()` minus the `descent()`.
    #[inline]
    #[must_use]
    pub fn height(&self) -> f32
    {
        self.height
    }

    /// The ascent (in pixels) of this text line. This is the maximum height of
    /// each glyph above the text baseline.
    #[inline]
    #[must_use]
    pub fn ascent(&self) -> f32
    {
        self.ascent
    }

    /// The descent (in pixels) of this text line. This is the furthest distance
    /// of each glyph below the text baseline.
    ///
    /// This is negative: a value of `-10.0` means the font can descend `10`
    /// pixels below the baseline.
    #[inline]
    #[must_use]
    pub fn descent(&self) -> f32
    {
        self.descent
    }

    /// The recommended gap to put between each line of text, as encoded by the
    /// font authors.
    #[inline]
    #[must_use]
    pub fn line_gap(&self) -> f32
    {
        self.line_gap
    }

    /// The vertical position of this line's baseline within the block of text.
    #[inline]
    #[must_use]
    pub fn baseline_position(&self) -> f32
    {
        self.baseline_vertical_position
    }

    fn add_offset_x(&mut self, offset_x: f32)
    {
        for glyph in self.glyphs.iter_mut() {
            glyph.add_offset_x(offset_x);
        }
    }
}

impl<T: Copy> From<&rusttype::Rect<T>> for Rectangle<T>
{
    #[inline]
    #[must_use]
    fn from(rect: &rusttype::Rect<T>) -> Self
    {
        Rectangle::new(
            Vector2::new(rect.min.x, rect.min.y),
            Vector2::new(rect.max.x, rect.max.y)
        )
    }
}

#[cfg(test)]
mod test
{
    use super::*;

    #[test]
    fn test_word_split_1()
    {
        let codepoints = Codepoint::from_unindexed_codepoints(&['a', 'b', ' ', 'c', 'd']);

        let words = Word::split_words(&codepoints);

        assert_eq!(
            vec![
                Word::Renderable(RenderableWord {
                    codepoints: vec![Codepoint::new(0, 'a'), Codepoint::new(1, 'b')],
                    is_whitespace: false
                }),
                Word::Renderable(RenderableWord {
                    codepoints: vec![Codepoint::new(2, ' ')],
                    is_whitespace: true
                }),
                Word::Renderable(RenderableWord {
                    codepoints: vec![Codepoint::new(3, 'c'), Codepoint::new(4, 'd')],
                    is_whitespace: false
                })
            ],
            words
        )
    }

    #[test]
    fn test_word_split_2()
    {
        let codepoints = Codepoint::from_unindexed_codepoints(&[
            'a', 'b', '\t', ' ', '\n', 'c', 'd', '\n', '\n', ' '
        ]);

        let words = Word::split_words(&codepoints);

        assert_eq!(
            vec![
                Word::Renderable(RenderableWord {
                    codepoints: vec![Codepoint::new(0, 'a'), Codepoint::new(1, 'b')],
                    is_whitespace: false
                }),
                Word::Renderable(RenderableWord {
                    codepoints: vec![Codepoint::new(2, '\t'),],
                    is_whitespace: true
                }),
                Word::Renderable(RenderableWord {
                    codepoints: vec![Codepoint::new(3, ' '),],
                    is_whitespace: true
                }),
                Word::Newline,
                Word::Renderable(RenderableWord {
                    codepoints: vec![Codepoint::new(5, 'c'), Codepoint::new(6, 'd')],
                    is_whitespace: false
                }),
                Word::Newline,
                Word::Newline,
                Word::Renderable(RenderableWord {
                    codepoints: vec![Codepoint::new(9, ' ')],
                    is_whitespace: true
                })
            ],
            words
        )
    }
}
