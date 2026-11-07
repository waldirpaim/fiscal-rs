//! Serialisation of the document into PDF objects (the `_put*` family of FPDF).

use std::collections::BTreeMap;
use std::io::Write as _;

use crate::document::{Fpdf, now_utc, pdf_version_string};
use crate::encoding::{literal_string, text_string_raw};
use crate::fmt::{fixed, float_to_str};
use crate::fonts::to_unicode_cmap;
use crate::image::{ImageInfo, zcompress};
use crate::types::{DateTime, LayoutMode, Link, Orientation, ZoomMode};

impl Fpdf {
    /// Appends a line to the document buffer.
    pub(crate) fn put(&mut self, line: &str) {
        self.put_bytes(line.as_bytes());
    }

    pub(crate) fn put_bytes(&mut self, line: &[u8]) {
        self.buffer.extend_from_slice(line);
        self.buffer.push(b'\n');
    }

    /// Begins a new object; `number` forces the object number (used for the fixed objects 1
    /// and 2).
    pub(crate) fn new_obj(&mut self, number: Option<usize>) {
        let n = match number {
            Some(n) => n,
            None => {
                self.n += 1;
                self.n
            }
        };
        if self.offsets.len() < n {
            self.offsets.resize(n, 0);
        }
        self.offsets[n - 1] = self.buffer.len();
        self.put(&format!("{n} 0 obj"));
    }

    pub(crate) fn put_stream(&mut self, data: &[u8]) {
        let data = self.encrypt_data(data);
        self.put("stream");
        self.put_bytes(&data);
        self.put("endstream");
    }

    /// Encrypts stream or string data for the object being written (identity when the
    /// document is not protected).
    pub(crate) fn encrypt_data(&self, data: &[u8]) -> Vec<u8> {
        #[cfg(feature = "protection")]
        {
            self.encrypt_for_object(data)
        }
        #[cfg(not(feature = "protection"))]
        {
            data.to_vec()
        }
    }

    /// A PDF literal text string for the object being written (encrypted when protected).
    pub(crate) fn pdf_text_string(&self, text: &str) -> Vec<u8> {
        literal_string(&self.encrypt_data(&text_string_raw(text)))
    }

    /// Writes a complete stream object, compressing it when compression is enabled.
    pub(crate) fn put_stream_object(&mut self, data: &[u8]) {
        let (entries, body) = if self.compress {
            ("/Filter /FlateDecode ".to_owned(), zcompress(data))
        } else {
            (String::new(), data.to_vec())
        };
        self.new_obj(None);
        self.put(&format!("<<{entries}/Length {}>>", body.len()));
        self.put_stream(&body);
        self.put("endobj");
    }

    fn put_links(&mut self, page: usize) {
        let count = self.page_links[page - 1].len();
        for i in 0..count {
            let pl = self.page_links[page - 1][i].clone();
            self.new_obj(None);
            let rect = format!(
                "{} {} {} {}",
                fixed(pl.x, 2),
                fixed(pl.y, 2),
                fixed(pl.x + pl.width, 2),
                fixed(pl.y - pl.height, 2)
            );
            let mut s = format!("<</Type /Annot /Subtype /Link /Rect [{rect}] /Border [0 0 0] ")
                .into_bytes();
            match &pl.link {
                Link::Uri(uri) => {
                    s.extend_from_slice(b"/A <</S /URI /URI ");
                    s.extend(self.pdf_text_string(uri));
                    s.extend_from_slice(b">>>>");
                }
                Link::Internal(id) => {
                    if let Some(dest) = self.links.get(*id).copied() {
                        if dest.page > 0 {
                            let dest_info = &self.page_info[dest.page - 1];
                            let height = match dest_info.size {
                                Some((_, h)) => h,
                                None if self.def_orientation == Orientation::Portrait => {
                                    self.def_page_size.h * self.k
                                }
                                None => self.def_page_size.w * self.k,
                            };
                            let _ = write!(
                                s,
                                "/Dest [{} 0 R /XYZ 0 {} null]>>",
                                dest_info.n,
                                fixed(height - dest.y * self.k, 2)
                            );
                        }
                    }
                }
            }
            self.put_bytes(&s);
            self.put("endobj");
        }
    }

    fn put_page(&mut self, page: usize) {
        self.new_obj(None);
        self.put("<</Type /Page");
        self.put("/Parent 1 0 R");
        let info = self.page_info[page - 1].clone();
        if let Some((w, h)) = info.size {
            self.put(&format!(
                "/MediaBox [0 0 {} {}]",
                float_to_str(w),
                float_to_str(h)
            ));
        }
        if let Some(rotation) = info.rotation.filter(|r| *r != 0) {
            self.put(&format!("/Rotate {rotation}"));
        }
        self.put("/Resources 2 0 R");
        if !self.page_links[page - 1].is_empty() {
            let mut s = String::from("/Annots [");
            for pl in &self.page_links[page - 1] {
                s.push_str(&format!("{} 0 R ", pl.n));
            }
            s.push(']');
            self.put(&s);
        }
        if self.with_alpha {
            self.put("/Group <</Type /Group /S /Transparency /CS /DeviceRGB>>");
        }
        self.put(&format!("/Contents {} 0 R>>", self.n + 1));
        self.put("endobj");

        // Page content.
        let mut content = std::mem::take(&mut self.pages[page - 1]);
        if !self.alias_nb_pages.is_empty() {
            content = replace_bytes(
                &content,
                &self.alias_nb_pages,
                self.page.to_string().as_bytes(),
            );
        }
        self.put_stream_object(&content);
        self.put_links(page);
    }

    fn put_pages(&mut self) {
        let nb = self.page;
        let mut n = self.n;
        for i in 0..nb {
            n += 1;
            self.page_info[i].n = n;
            n += 1;
            for j in 0..self.page_links[i].len() {
                n += 1;
                self.page_links[i][j].n = n;
            }
        }
        for i in 1..=nb {
            self.put_page(i);
        }
        // Pages root.
        self.new_obj(Some(1));
        self.put("<</Type /Pages");
        let mut kids = String::from("/Kids [");
        for info in &self.page_info {
            kids.push_str(&format!("{} 0 R ", info.n));
        }
        kids.push(']');
        self.put(&kids);
        self.put(&format!("/Count {nb}"));
        let (w, h) = if self.def_orientation == Orientation::Portrait {
            (self.def_page_size.w, self.def_page_size.h)
        } else {
            (self.def_page_size.h, self.def_page_size.w)
        };
        self.put(&format!(
            "/MediaBox [0 0 {} {}]",
            fixed(w * self.k, 2),
            fixed(h * self.k, 2)
        ));
        self.put(">>");
        self.put("endobj");
    }

    fn put_fonts(&mut self) {
        let mut cmaps: BTreeMap<String, usize> = BTreeMap::new();
        for i in 0..self.used_fonts.len() {
            let key = self.used_fonts[i].font_name.clone();
            let Some(font) = self
                .fonts
                .iter()
                .find(|f| f.font_name.eq_ignore_ascii_case(&key))
                .cloned()
            else {
                continue;
            };
            // ToUnicode CMap, shared between fonts with the same encoding.
            let cmap = to_unicode_cmap(&font.uv1, &font.uv2);
            let mut cmap_key = None;
            if !cmap.is_empty() {
                let k = if font.cp1252 {
                    "cp1252".to_owned()
                } else {
                    font.name.clone()
                };
                if !cmaps.contains_key(&k) {
                    self.put_stream_object(cmap.as_bytes());
                    cmaps.insert(k.clone(), self.n);
                }
                cmap_key = Some(k);
            }
            // Core font object.
            self.used_fonts[i].n = self.n + 1;
            self.new_obj(None);
            self.put("<</Type /Font");
            self.put(&format!("/BaseFont /{}", font.name));
            self.put("/Subtype /Type1");
            if font.name != "Symbol" && font.name != "ZapfDingbats" {
                self.put("/Encoding /WinAnsiEncoding");
            }
            if let Some(n) = cmap_key.and_then(|k| cmaps.get(&k).copied()) {
                self.put(&format!("/ToUnicode {n} 0 R"));
            }
            self.put(">>");
            self.put("endobj");
        }
    }

    fn put_images(&mut self) {
        for i in 0..self.images.len() {
            let mut img = std::mem::take(&mut self.images[i]);
            self.put_image(&mut img);
            img.data = Vec::new();
            img.smask = Vec::new();
            self.images[i] = img;
        }
    }

    fn put_image(&mut self, img: &mut ImageInfo) {
        self.new_obj(None);
        img.n = self.n;
        self.put("<</Type /XObject");
        self.put("/Subtype /Image");
        self.put(&format!("/Width {}", img.w));
        self.put(&format!("/Height {}", img.h));
        if img.cs == "Indexed" {
            self.put(&format!(
                "/ColorSpace [/Indexed /DeviceRGB {} {} 0 R]",
                float_to_str((img.pal.len() / 3) as f64 - 1.0),
                self.n + 1
            ));
        } else {
            self.put(&format!("/ColorSpace /{}", img.cs));
            if img.cs == "DeviceCMYK" {
                self.put("/Decode [1 0 1 0 1 0 1 0]");
            }
        }
        self.put(&format!("/BitsPerComponent {}", img.bpc));
        if !img.f.is_empty() {
            self.put(&format!("/Filter /{}", img.f));
        }
        if !img.dp.is_empty() {
            self.put(&format!("/DecodeParms <<{}>>", img.dp));
        }
        if !img.trns.is_empty() {
            let mut s = String::new();
            for t in &img.trns {
                s.push_str(&format!("{t} {t} "));
            }
            self.put(&format!("/Mask [{s}]"));
        }
        if !img.smask.is_empty() {
            self.put(&format!("/SMask {} 0 R", self.n + 1));
        }
        self.put(&format!("/Length {}>>", img.data.len()));
        self.put_stream(&img.data);
        self.put("endobj");

        // Soft mask.
        if !img.smask.is_empty() {
            let mut mask = ImageInfo {
                w: img.w,
                h: img.h,
                cs: "DeviceGray".to_owned(),
                bpc: 8,
                f: img.f.clone(),
                dp: format!(
                    "/Predictor 15 /Colors 1 /BitsPerComponent 8 /Columns {}",
                    img.w
                ),
                data: std::mem::take(&mut img.smask),
                ..ImageInfo::default()
            };
            self.put_image(&mut mask);
        }
        // Palette.
        if img.cs == "Indexed" {
            let pal = img.pal.clone();
            self.put_stream_object(&pal);
        }
    }

    fn put_resource_dict(&mut self) {
        self.put("/ProcSet [/PDF /Text /ImageB /ImageC /ImageI]");
        self.put("/Font <<");
        for i in 0..self.used_fonts.len() {
            self.put(&format!("/F{} {} 0 R", i + 1, self.used_fonts[i].n));
        }
        self.put(">>");
        self.put("/XObject <<");
        for i in 0..self.images.len() {
            self.put(&format!("/I{} {} 0 R", self.images[i].i, self.images[i].n));
        }
        self.put(">>");
        if !self.layers.is_empty() {
            self.put("/Properties <<");
            for i in 0..self.layers.len() {
                self.put(&format!("/OC{i} {} 0 R", self.layers[i].n));
            }
            self.put(">>");
        }
    }

    fn put_resources(&mut self) {
        self.put_layers();
        self.put_fonts();
        self.put_images();
        // Resource dictionary.
        self.new_obj(Some(2));
        self.put("<<");
        self.put_resource_dict();
        self.put(">>");
        self.put("endobj");
        self.put_encryption_object();
    }

    /// Writes the encryption dictionary object when the document is protected.
    #[cfg(feature = "protection")]
    fn put_encryption_object(&mut self) {
        let Some(lines) = self.encryption.as_ref().map(|e| e.dictionary_lines()) else {
            return;
        };
        self.new_obj(None);
        let n = self.n;
        if let Some(enc) = self.encryption.as_mut() {
            enc.obj_id = n;
        }
        self.put("<<");
        for line in lines {
            self.put_bytes(&line);
        }
        self.put(">>");
        self.put("endobj");
    }

    #[cfg(not(feature = "protection"))]
    fn put_encryption_object(&mut self) {}

    fn put_info(&mut self) {
        let date = self.creation_date.unwrap_or_else(now_utc);
        let tz = if self.time_zone.len() == 6 {
            format!("{}'{}'", &self.time_zone[..3], &self.time_zone[4..6])
        } else {
            self.time_zone.clone()
        };
        let stamp = format_pdf_date(&date, &tz);
        self.set_metadata("CreationDate", &stamp);
        for i in 0..self.metadata.len() {
            let (key, value) = self.metadata[i].clone();
            let mut line = format!("/{key} ").into_bytes();
            line.extend(self.pdf_text_string(&value));
            self.put_bytes(&line);
        }
    }

    fn put_catalog(&mut self) {
        let first = self.page_info.first().map_or(0, |p| p.n);
        self.put("/Type /Catalog");
        self.put("/Pages 1 0 R");
        match self.zoom_mode {
            ZoomMode::FullPage => self.put(&format!("/OpenAction [{first} 0 R /Fit]")),
            ZoomMode::FullWidth => self.put(&format!("/OpenAction [{first} 0 R /FitH null]")),
            ZoomMode::Real => self.put(&format!("/OpenAction [{first} 0 R /XYZ null null 1]")),
            ZoomMode::Custom if self.zoom_factor > 0 => self.put(&format!(
                "/OpenAction [{first} 0 R /XYZ null null {}]",
                fixed(f64::from(self.zoom_factor) / 100.0, 2)
            )),
            _ => {}
        }
        match self.layout_mode {
            LayoutMode::Single => self.put("/PageLayout /SinglePage"),
            LayoutMode::Continuous => self.put("/PageLayout /OneColumn"),
            LayoutMode::Two => self.put("/PageLayout /TwoColumnLeft"),
            LayoutMode::Default => {}
        }
        if !self.layers.is_empty() {
            let all: String = self
                .layers
                .iter()
                .map(|l| format!("{} 0 R ", l.n))
                .collect();
            let off: String = self
                .layers
                .iter()
                .filter(|l| !l.visible)
                .map(|l| format!("{} 0 R ", l.n))
                .collect();
            self.put(&format!(
                "/OCProperties <</OCGs [{all}] /D <</OFF [{off}] /Order [{all}]>>>>"
            ));
            if self.open_layer_pane {
                self.put("/PageMode /UseOC");
            }
        }
    }

    fn put_trailer(&mut self) {
        self.put(&format!("/Size {}", self.n + 1));
        self.put(&format!("/Root {} 0 R", self.n));
        self.put(&format!("/Info {} 0 R", self.n - 1));
        #[cfg(feature = "protection")]
        if let Some(enc) = &self.encryption {
            let id = enc.obj_id;
            self.put(&format!("/Encrypt {id} 0 R"));
            self.put("/ID [()()]");
        }
    }

    /// Writes the whole document into `buffer`.
    pub(crate) fn end_doc(&mut self) {
        // Optional content needs PDF 1.5.
        if self.pdf_version < 1.5 && !self.layers.is_empty() {
            self.pdf_version = 1.5;
        }
        let header = format!("%PDF-{}", pdf_version_string(self.pdf_version));
        self.put(&header);
        self.put_pages();
        self.put_resources();
        // Info.
        self.new_obj(None);
        self.put("<<");
        self.put_info();
        self.put(">>");
        self.put("endobj");
        // Catalog.
        self.new_obj(None);
        self.put("<<");
        self.put_catalog();
        self.put(">>");
        self.put("endobj");
        // Cross-reference table.
        let offset = self.buffer.len();
        self.put("xref");
        self.put(&format!("0 {}", self.n + 1));
        self.put("0000000000 65535 f ");
        for i in 0..self.n {
            let line = format!("{:010} 00000 n ", self.offsets.get(i).copied().unwrap_or(0));
            self.put(&line);
        }
        // Trailer.
        self.put("trailer");
        self.put("<<");
        self.put_trailer();
        self.put(">>");
        self.put("startxref");
        self.put(&offset.to_string());
        self.put("%%EOF");
        self.state = 3;
        self.page_info.clear();
        self.pages.clear();
        self.page_links.clear();
        self.links.clear();
    }
}

fn format_pdf_date(d: &DateTime, tz: &str) -> String {
    format!(
        "D:{:04}{:02}{:02}{:02}{:02}{:02}{tz}",
        d.year, d.month, d.day, d.hour, d.minute, d.second
    )
}

/// Replaces every occurrence of `from` in `data` with `to`.
fn replace_bytes(data: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    if from.is_empty() {
        return data.to_vec();
    }
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        if data[i..].starts_with(from) {
            out.extend_from_slice(to);
            i += from.len();
        } else {
            out.push(data[i]);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_all_occurrences() {
        assert_eq!(
            replace_bytes(b"page {nb} of {nb}", b"{nb}", b"3"),
            b"page 3 of 3".to_vec()
        );
        assert_eq!(replace_bytes(b"abc", b"", b"x"), b"abc".to_vec());
    }

    #[test]
    fn formats_pdf_date() {
        let d = DateTime {
            year: 2026,
            month: 10,
            day: 21,
            hour: 8,
            minute: 5,
            second: 9,
        };
        assert_eq!(format_pdf_date(&d, "Z"), "D:20261021080509Z");
        assert_eq!(format_pdf_date(&d, "-03'00'"), "D:20261021080509-03'00'");
    }
}
