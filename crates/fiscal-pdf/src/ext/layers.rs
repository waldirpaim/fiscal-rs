//! Optional content groups ("layers") that a viewer can show or hide.

use crate::document::Fpdf;

/// A layer (optional content group).
#[derive(Debug, Clone)]
pub(crate) struct Layer {
    pub name: String,
    pub visible: bool,
    /// PDF object number, assigned when the document is written.
    pub n: usize,
}

impl Fpdf {
    /// Defines a layer and returns its identifier.
    pub fn add_layer(&mut self, name: &str, visible: bool) -> usize {
        self.layers.push(Layer {
            name: name.to_owned(),
            visible,
            n: 0,
        });
        self.layers.len() - 1
    }

    /// Starts drawing on a layer (closing the previous one).
    pub fn begin_layer(&mut self, id: usize) {
        if id >= self.layers.len() {
            self.fail(format!("Invalid Layer Id: {id}"));
            return;
        }
        self.end_layer();
        self.out(&format!("/OC /OC{id} BDC"));
        self.current_layer = Some(id);
    }

    /// Starts drawing on the layer called `name`.
    pub fn begin_layer_named(&mut self, name: &str) {
        match self.layers.iter().position(|l| l.name == name) {
            Some(id) => self.begin_layer(id),
            None => self.fail(format!("Layer Name not found: {name}")),
        }
    }

    /// Ends the current layer, if any.
    pub fn end_layer(&mut self) {
        if self.current_layer.take().is_some() {
            self.out("EMC");
        }
    }

    /// Makes the viewer open the layers pane.
    pub fn open_layer_pane(&mut self) {
        self.open_layer_pane = true;
    }

    /// Writes one `OCG` object per layer.
    pub(crate) fn put_layers(&mut self) {
        for i in 0..self.layers.len() {
            self.new_obj(None);
            self.layers[i].n = self.n;
            let mut line = b"<</Type /OCG /Name ".to_vec();
            line.extend(self.pdf_text_string(&self.layers[i].name.clone()));
            line.extend_from_slice(b">>");
            self.put_bytes(&line);
            self.put("endobj");
        }
    }
}
