//! A tiny HTML writer: `<b>`, `<i>`, `<u>`, `<br>` and `<a href="...">`.

use crate::document::Fpdf;
use crate::types::Link;

impl Fpdf {
    /// Writes text with a minimal set of HTML tags in flowing mode: `<b>`, `<i>`, `<u>`
    /// toggle the font style, `<br>` breaks the line and `<a href="url">` adds a link.
    /// Newlines in the source count as spaces.
    pub fn write_html(&mut self, html: &str) {
        let source = html.replace("\r\n", " ").replace('\n', " ");
        let mut rest = source.as_str();
        while !rest.is_empty() {
            let Some(open) = rest.find('<') else {
                self.html_text(rest);
                break;
            };
            self.html_text(&rest[..open]);
            let Some(close) = rest[open..].find('>') else {
                // Unterminated tag: print it as text.
                self.html_text(&rest[open..]);
                break;
            };
            let tag = &rest[open + 1..open + close];
            if let Some(name) = tag.strip_prefix('/') {
                self.html_close_tag(name);
            } else {
                self.html_open_tag(tag);
            }
            rest = &rest[open + close + 1..];
        }
    }

    fn html_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if self.html_href.is_empty() {
            self.write(self.font_size + 0.5, text, None);
        } else {
            let href = self.html_href.clone();
            self.html_put_link(&href, text);
        }
    }

    fn html_open_tag(&mut self, tag: &str) {
        let upper = tag.to_uppercase();
        match upper.as_str() {
            "B" | "I" | "U" => self.html_set_style(&upper, true),
            "BR" => self.ln(5.0),
            _ if upper.starts_with('A') => {
                if let Some(p) = tag.find("href=\"") {
                    let value = &tag[p + 6..];
                    let end = value.find('"').unwrap_or(value.len());
                    self.html_href = value[..end].to_owned();
                }
            }
            _ => {}
        }
    }

    fn html_close_tag(&mut self, tag: &str) {
        let upper = tag.to_uppercase();
        match upper.as_str() {
            "B" | "I" | "U" => self.html_set_style(&upper, false),
            "A" => self.html_href.clear(),
            _ => {}
        }
    }

    fn html_set_style(&mut self, tag: &str, enable: bool) {
        let position = self.html_style.find(tag);
        match (enable, position) {
            (true, None) => self.html_style.push_str(tag),
            (false, Some(p)) => {
                self.html_style.replace_range(p..p + tag.len(), "");
            }
            _ => {}
        }
        let style = self.html_style.clone();
        self.set_font("", &style, 0.0);
    }

    fn html_put_link(&mut self, url: &str, text: &str) {
        self.set_text_color((0, 0, 255));
        self.html_set_style("U", true);
        self.write(5.0, text, Some(&Link::Uri(url.to_owned())));
        self.html_set_style("U", false);
        self.set_text_color(crate::types::Color::BLACK);
    }
}
