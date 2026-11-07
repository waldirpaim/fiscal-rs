//! Rotation, rounded rectangles, dashes and arcs.

use crate::document::Fpdf;
use crate::fmt::fixed;

impl Fpdf {
    /// Rotates the following drawing counter-clockwise by `angle` degrees around `(x, y)`
    /// (`None` = the current position). `rotate(0.0, None, None)` restores the normal
    /// orientation; an open rotation is closed automatically at the end of the page.
    pub fn rotate(&mut self, angle: f64, x: Option<f64>, y: Option<f64>) {
        let x = x.unwrap_or(self.x);
        let y = y.unwrap_or(self.y);
        if self.angle != 0.0 {
            self.out("Q");
        }
        self.angle = angle;
        if angle != 0.0 {
            let rad = angle.to_radians();
            let (s, c) = rad.sin_cos();
            let cx = x * self.k;
            let cy = (self.h - y) * self.k;
            let op = format!(
                "q {} {} {} {} {} {} cm 1 0 0 1 {} {} cm",
                fixed(c, 5),
                fixed(s, 5),
                fixed(-s, 5),
                fixed(c, 5),
                fixed(cx, 2),
                fixed(cy, 2),
                fixed(-cx, 2),
                fixed(-cy, 2)
            );
            self.out(&op);
        }
    }

    /// Draws a rectangle with rounded corners. `corners` lists the rounded corners as digits
    /// (`1` top-left, `2` top-right, `3` bottom-right, `4` bottom-left; `""` = none);
    /// `style` is `"F"` (fill), `"FD"`/`"DF"` (both) or anything else for the outline.
    #[allow(clippy::too_many_arguments)]
    pub fn rounded_rect(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        radius: f64,
        corners: &str,
        style: &str,
    ) {
        let k = self.k;
        let hp = self.h;
        let op = match style {
            "F" => "f",
            "FD" | "DF" => "B",
            _ => "S",
        };
        let arc = 4.0 / 3.0 * (2.0_f64.sqrt() - 1.0);
        let pt = |a: f64, b: f64| format!("{} {}", fixed(a * k, 2), fixed((hp - b) * k, 2));

        self.out(&format!("{} m", pt(x + radius, y)));
        let mut xc = x + width - radius;
        let mut yc = y + radius;
        self.out(&format!("{} l", pt(xc, y)));
        if !corners.contains('2') {
            self.out(&format!("{} l", pt(x + width, y)));
        } else {
            self.arc(
                xc + radius * arc,
                yc - radius,
                xc + radius,
                yc - radius * arc,
                xc + radius,
                yc,
            );
        }

        xc = x + width - radius;
        yc = y + height - radius;
        self.out(&format!("{} l", pt(x + width, yc)));
        if !corners.contains('3') {
            self.out(&format!("{} l", pt(x + width, y + height)));
        } else {
            self.arc(
                xc + radius,
                yc + radius * arc,
                xc + radius * arc,
                yc + radius,
                xc,
                yc + radius,
            );
        }

        xc = x + radius;
        yc = y + height - radius;
        self.out(&format!("{} l", pt(xc, y + height)));
        if !corners.contains('4') {
            self.out(&format!("{} l", pt(x, y + height)));
        } else {
            self.arc(
                xc - radius * arc,
                yc + radius,
                xc - radius,
                yc + radius * arc,
                xc - radius,
                yc,
            );
        }

        xc = x + radius;
        yc = y + radius;
        self.out(&format!("{} l", pt(x, yc)));
        if !corners.contains('1') {
            self.out(&format!("{} l", pt(x, y)));
            self.out(&format!("{} l", pt(x + radius, y)));
        } else {
            self.arc(
                xc - radius,
                yc - radius * arc,
                xc - radius * arc,
                yc - radius,
                xc,
                yc - radius,
            );
        }
        self.out(op);
    }

    /// Appends a cubic Bézier curve to the current path (coordinates in user units).
    fn arc(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) {
        let (k, h) = (self.k, self.h);
        let op = format!(
            "{} {} {} {} {} {} c ",
            fixed(x1 * k, 2),
            fixed((h - y1) * k, 2),
            fixed(x2 * k, 2),
            fixed((h - y2) * k, 2),
            fixed(x3 * k, 2),
            fixed((h - y3) * k, 2)
        );
        self.out(&op);
    }

    /// Sets the dash pattern: `black` units drawn, `white` units skipped. Non-positive
    /// values restore solid lines.
    pub fn set_dash(&mut self, black: f64, white: f64) {
        if black > 0.0 && white > 0.0 {
            let op = format!(
                "[{} {}] 0 d",
                fixed(black * self.k, 3),
                fixed(white * self.k, 3)
            );
            self.out(&op);
        } else {
            self.out("[] 0 d");
        }
    }

    /// Sets a dash pattern with equal dash and gap length.
    pub fn set_dash_width(&mut self, width: f64) {
        self.set_dash(width, width);
    }

    /// Draws a dashed line.
    pub fn dashed_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, dash_width: f64) {
        self.set_dash_width(dash_width);
        self.line(x1, y1, x2, y2);
        self.set_dash_width(0.0);
    }

    /// Draws a rectangle with a dashed outline.
    pub fn dashed_rect(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        style: &str,
        dash_width: f64,
    ) {
        self.set_dash_width(dash_width);
        self.rect(x, y, width, height, style);
        self.set_dash_width(0.0);
    }

    /// Saves the graphics state (`q`).
    pub fn start_transform(&mut self) {
        self.out("q");
    }

    /// Restores the graphics state (`Q`).
    pub fn stop_transform(&mut self) {
        self.out("Q");
    }

    /// Applies the transformation matrix `[a b c d e f]` to the following drawing.
    pub fn transform(&mut self, m: [f64; 6]) {
        let op = format!(
            "{} {} {} {} {} {} cm",
            fixed(m[0], 3),
            fixed(m[1], 3),
            fixed(m[2], 3),
            fixed(m[3], 3),
            fixed(m[4], 3),
            fixed(m[5], 3)
        );
        self.out(&op);
    }

    /// Translates the coordinate system by `(tx, ty)` user units.
    pub fn translate(&mut self, tx: f64, ty: f64) {
        self.transform([1.0, 0.0, 0.0, 1.0, tx * self.k, -ty * self.k]);
    }
}
