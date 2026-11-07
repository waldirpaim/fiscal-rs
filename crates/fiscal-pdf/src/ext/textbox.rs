//! Word wrapping and text boxes (FPDF scripts "WordWrap" by Ron Korving and the ACBr
//! `TextBox`).

use crate::document::Fpdf;

/// Vertical alignment of a text box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VAlign {
    /// Text starts at the top (`'T'`).
    #[default]
    Top,
    /// Text block is centred vertically (`'C'`).
    Center,
    /// Text block ends at the bottom (`'B'`).
    Bottom,
}

impl From<char> for VAlign {
    fn from(value: char) -> Self {
        match value {
            'C' => Self::Center,
            'B' => Self::Bottom,
            _ => Self::Top,
        }
    }
}

/// Horizontal alignment of a text box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HAlign {
    /// Left (`'L'`).
    #[default]
    Left,
    /// Centred (`'C'`).
    Center,
    /// Right (`'R'`).
    Right,
}

impl From<char> for HAlign {
    fn from(value: char) -> Self {
        match value {
            'C' => Self::Center,
            'R' => Self::Right,
            _ => Self::Left,
        }
    }
}

/// Options of [`Fpdf::text_box`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextBoxOpts {
    /// Vertical alignment.
    pub valign: VAlign,
    /// Horizontal alignment.
    pub halign: HAlign,
    /// Draw a thin border around the box.
    pub border: bool,
    /// Wrap the text at the box width (otherwise each line is clipped to it).
    pub word_wrap: bool,
    /// Shrink the font until the text fits the box.
    pub scale: bool,
    /// Extra space between lines, in user units.
    pub line_spacing: f64,
}

impl Default for TextBoxOpts {
    fn default() -> Self {
        Self {
            valign: VAlign::Top,
            halign: HAlign::Left,
            border: true,
            word_wrap: true,
            scale: false,
            line_spacing: 0.0,
        }
    }
}

/// Delphi `Trim`: removes control characters and spaces at both ends.
fn trim(s: &str) -> &str {
    s.trim_matches(|c: char| c <= ' ')
}

/// Delphi `TrimRight`.
fn trim_right(s: &str) -> &str {
    s.trim_end_matches(|c: char| c <= ' ')
}

/// The line separator used inside wrapped text.
const LINE_BREAK: &str = "\n";

/// Splits like the original `Split`: an empty string gives no pieces, and every piece is
/// stripped of its leading control characters and spaces when `trim_left` is set.
fn split<'a>(s: &'a str, delimiter: &str, trim_left: bool) -> Vec<&'a str> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split(delimiter)
        .map(|piece| {
            if trim_left {
                piece.trim_start_matches(|c: char| c <= ' ')
            } else {
                piece
            }
        })
        .collect()
}

/// Normalises the line breaks of user text to `\n`.
fn normalise_breaks(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "")
}

impl Fpdf {
    /// Breaks `text` into lines no wider than `max_width` with the current font and returns
    /// the number of lines. The wrapped text (lines separated by `\n`) replaces `text`.
    /// `indent` shortens the first line.
    pub fn word_wrap(&self, text: &mut String, max_width: f64, indent: f64) -> usize {
        let normalised = normalise_breaks(trim(text));
        *text = normalised.clone();
        if normalised.is_empty() {
            return 0;
        }
        let space = self.get_string_width(" ");
        let mut out = String::new();
        let mut lines = 0;
        // Only the first output line is shortened by the indent.
        let mut limit = max_width - indent;
        let next_line = |limit: &mut f64| *limit = max_width;

        for line in split(&normalised, LINE_BREAK, true) {
            let mut width = 0.0;
            for word in split(line, " ", true) {
                if trim(word).is_empty() {
                    continue;
                }
                let word_width = self.get_string_width(word);
                if word_width > limit {
                    // The word is too long for a line: cut it.
                    for ch in word.chars() {
                        let ch_width = self.get_string_width(&ch.to_string());
                        if width + ch_width <= limit {
                            width += ch_width;
                            out.push(ch);
                        } else {
                            width = ch_width;
                            out.truncate(trim_right(&out).len());
                            out.push_str(LINE_BREAK);
                            out.push(ch);
                            lines += 1;
                            next_line(&mut limit);
                        }
                    }
                } else if width + word_width <= limit {
                    width += word_width + space;
                    out.push_str(word);
                    out.push(' ');
                } else {
                    width = word_width + space;
                    out.truncate(trim_right(&out).len());
                    out.push_str(LINE_BREAK);
                    out.push_str(word);
                    out.push(' ');
                    lines += 1;
                    next_line(&mut limit);
                }
            }
            out.truncate(trim_right(&out).len());
            out.push_str(LINE_BREAK);
            lines += 1;
            next_line(&mut limit);
        }
        out.truncate(trim_right(&out).len());
        *text = out;
        lines
    }

    /// Number of lines `text` needs in a column `width` wide (with the current font).
    pub fn get_num_lines(&self, text: &str, width: f64, indent: f64) -> usize {
        let mut wrapped = trim(text).to_owned();
        self.word_wrap(&mut wrapped, width - 0.2, indent)
    }

    /// Height `text` needs in a column `width` wide, including `line_spacing` between lines.
    pub fn get_string_height(&self, text: &str, width: f64, line_spacing: f64, indent: f64) -> f64 {
        let lines = self.get_num_lines(text, width, indent);
        let spacing = if lines > 1 {
            (lines - 1) as f64 * line_spacing
        } else {
            0.0
        };
        let raw = lines as f64 * self.font_size + spacing;
        crate::fmt::round_to(raw, -2) + 0.5
    }

    /// Prints `text` inside the box `(x, y, width, height)`; returns the height used by the
    /// text. The font is restored to its original size afterwards.
    pub fn text_box(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        text: &str,
        opts: &TextBoxOpts,
    ) -> f64 {
        let max_height = height;
        let old_font_size = self.font_size_pt;
        if width < 0.0 {
            return y;
        }
        let mut text = normalise_breaks(trim(text));
        if opts.border {
            self.rounded_rect(x, y, width, height, 0.8, "", "D");
        }
        let mut inc_y = self.font_size;
        let line_count = if opts.word_wrap && !text.is_empty() {
            while opts.scale
                && self.get_string_height(&text, width, opts.line_spacing, 0.0) > max_height
                && self.shrink_font()
            {
                inc_y = self.font_size;
            }
            self.word_wrap(&mut text, width, 0.0)
        } else {
            split(&text, LINE_BREAK, true).len()
        };
        let block = inc_y * line_count as f64 + (line_count as f64 - 1.0) * opts.line_spacing;
        let lines: Vec<String> = split(&text, LINE_BREAK, true)
            .into_iter()
            .map(str::to_owned)
            .collect();

        let mut y1 = match opts.valign {
            VAlign::Center => y + inc_y + (height - block) / 2.0 - 1.0,
            VAlign::Bottom => (y + height + inc_y) - block - 1.0,
            VAlign::Top => y + inc_y,
        };

        for line in &lines {
            let mut line_text = trim(line).to_owned();
            let mut comp = self.get_string_width(&line_text);
            if comp > width {
                if opts.scale {
                    while comp > width && self.shrink_font() {
                        comp = self.get_string_width(&line_text);
                    }
                } else {
                    while comp > width && !line_text.is_empty() {
                        line_text.pop();
                        comp = self.get_string_width(&line_text);
                    }
                }
            }
            let x1 = match opts.halign {
                HAlign::Center => x + (width - comp) / 2.0,
                HAlign::Right => x + width - (comp + 0.5),
                HAlign::Left => x + 0.5,
            };
            self.text(x1, y1, &line_text);
            if !opts.word_wrap && self.font_size_pt != old_font_size {
                let (family, style) = (self.font_family.clone(), self.font_style.clone());
                self.set_font(&family, &style, old_font_size);
            }
            y1 += inc_y + opts.line_spacing;
            if max_height > 0.0 && y1 > y + max_height {
                break;
            }
        }
        let (family, style) = (self.font_family.clone(), self.font_style.clone());
        self.set_font(&family, &style, old_font_size);
        (y1 - y) - inc_y - opts.line_spacing
    }

    /// Makes the font a little smaller (0.5 pt above 8 pt, 0.1 pt below). Returns `false`
    /// when the font can't shrink any further.
    fn shrink_font(&mut self) -> bool {
        if self.font_size_pt <= 0.15 {
            return false;
        }
        let step = if self.font_size_pt > 8.0 { 0.5 } else { 0.1 };
        let (family, style) = (self.font_family.clone(), self.font_style.clone());
        self.set_font(&family, &style, self.font_size_pt - step);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Orientation, PageFormat, Unit};

    fn pdf() -> Fpdf {
        let mut pdf = Fpdf::new(Orientation::Portrait, Unit::Mm, PageFormat::A4);
        pdf.add_page();
        pdf.set_font("Helvetica", "", 10.0);
        pdf
    }

    #[test]
    fn splits_like_the_original() {
        assert!(split("", "\n", true).is_empty());
        assert_eq!(split("a\n  b\n", "\n", true), ["a", "b", ""]);
        assert_eq!(split("  x", " ", false), ["", "", "x"]);
    }

    #[test]
    fn word_wrap_breaks_on_width() {
        let pdf = pdf();
        let mut text = "um dois tres quatro cinco seis sete oito".to_owned();
        let n = pdf.word_wrap(&mut text, 25.0, 0.0);
        assert!(n >= 3, "{n} lines: {text:?}");
        assert_eq!(text.lines().count(), n);
        for line in text.lines() {
            assert!(pdf.get_string_width(line) <= 25.0, "{line:?}");
        }
        assert_eq!(
            text.replace('\n', " "),
            "um dois tres quatro cinco seis sete oito"
        );
    }

    #[test]
    fn word_wrap_cuts_words_longer_than_the_line() {
        let pdf = pdf();
        let mut text = "Supercalifragilisticexpialidocious".to_owned();
        let n = pdf.word_wrap(&mut text, 20.0, 0.0);
        assert!(n >= 2);
        assert_eq!(text.replace('\n', ""), "Supercalifragilisticexpialidocious");
    }

    #[test]
    fn word_wrap_keeps_explicit_breaks_and_handles_empty() {
        let pdf = pdf();
        let mut text = "a\nb\nc".to_owned();
        assert_eq!(pdf.word_wrap(&mut text, 100.0, 0.0), 3);
        assert_eq!(text, "a\nb\nc");
        let mut empty = "   ".to_owned();
        assert_eq!(pdf.word_wrap(&mut empty, 100.0, 0.0), 0);
        assert_eq!(empty, "");
    }

    #[test]
    fn string_height_grows_with_lines() {
        let pdf = pdf();
        let one = pdf.get_string_height("curto", 50.0, 0.0, 0.0);
        let many = pdf.get_string_height(&"palavra ".repeat(30), 50.0, 0.0, 0.0);
        assert!(many > one * 3.0, "{one} {many}");
        let spaced = pdf.get_string_height("a\nb\nc", 50.0, 1.0, 0.0);
        let plain = pdf.get_string_height("a\nb\nc", 50.0, 0.0, 0.0);
        assert!((spaced - plain - 2.0).abs() < 0.011);
    }

    #[test]
    fn text_box_draws_clips_and_restores_font() {
        let mut pdf = pdf();
        let used = pdf.text_box(
            10.0,
            10.0,
            30.0,
            20.0,
            "texto bem comprido que nao cabe",
            &TextBoxOpts::default(),
        );
        assert!(used > 0.0);
        assert_eq!(pdf.font_size_pt, 10.0);
        // Not wrapped: every line is clipped to the width.
        let used = pdf.text_box(
            10.0,
            40.0,
            10.0,
            10.0,
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            &TextBoxOpts {
                word_wrap: false,
                ..TextBoxOpts::default()
            },
        );
        assert!(used > 0.0);
    }

    #[test]
    fn text_box_scale_shrinks_the_font_to_fit() {
        let mut pdf = pdf();
        pdf.text_box(
            10.0,
            10.0,
            20.0,
            6.0,
            "um texto que precisa ser reduzido para caber nesta caixinha",
            &TextBoxOpts {
                scale: true,
                ..TextBoxOpts::default()
            },
        );
        assert_eq!(pdf.font_size_pt, 10.0, "font restored after scaling");
        assert!(pdf.error().is_none());
    }

    #[test]
    fn text_box_alignment_moves_the_origin() {
        let origin = |opts: TextBoxOpts| {
            let mut p = pdf();
            p.set_compression(false);
            p.text_box(10.0, 10.0, 60.0, 20.0, "ab", &opts);
            let bytes = p.output().expect("pdf");
            let s: String = bytes.iter().map(|b| char::from(*b)).collect();
            let line = s
                .lines()
                .find(|l| l.contains("Td (ab)"))
                .expect("text")
                .to_owned();
            let nums: Vec<f64> = line.split(' ').filter_map(|t| t.parse().ok()).collect();
            (nums[0], nums[1])
        };
        let tl = origin(TextBoxOpts {
            border: false,
            ..TextBoxOpts::default()
        });
        let cr = origin(TextBoxOpts {
            border: false,
            halign: HAlign::Right,
            valign: VAlign::Center,
            ..TextBoxOpts::default()
        });
        assert!(cr.0 > tl.0, "right aligned is further right");
        assert!(cr.1 < tl.1, "vertically centred is lower (smaller PDF y)");
    }
}
