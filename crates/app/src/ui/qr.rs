//! A QR code drawn on the screen: the pairing link of "Share this machine",
//! for a phone's camera.
//!
//! The modules are painted as quads, not scaled from an image, so they stay
//! sharp at any zoom and pixel density. The tile is always dark on light,
//! whatever the theme: scanners do not read an inverted code reliably.

use crate::theme::px;
use gpui_kit::prelude::*;
use gpui_kit::{div, fill, point, rgb, size, Bounds, Div, Pixels};
use qrcode::{Color, EcLevel, QrCode};
use std::rc::Rc;

/// Light modules around the code, on each side: what the standard asks for.
const QUIET: usize = 4;

/// The modules of a code and the text they say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Qr {
    /// What the code says.
    pub text: String,
    /// Modules per side, without the quiet zone.
    pub side: usize,
    /// Whether each module is dark, row by row.
    pub dark: Vec<bool>,
}

impl Qr {
    /// The code of `text`; `None` when it is too long for one.
    pub fn of(text: &str) -> Option<Self> {
        let code = QrCode::with_error_correction_level(text.as_bytes(), EcLevel::M).ok()?;
        Some(Self {
            text: text.to_owned(),
            side: code.width(),
            dark: code
                .to_colors()
                .into_iter()
                .map(|c| c == Color::Dark)
                .collect(),
        })
    }

    /// The dark runs of a row: where each starts and how many modules it has.
    fn runs(&self, row: usize) -> Vec<(usize, usize)> {
        let cells = &self.dark[row * self.side..(row + 1) * self.side];
        let mut runs = Vec::new();
        let mut start = None;
        for (at, dark) in cells.iter().copied().chain([false]).enumerate() {
            match (dark, start) {
                (true, None) => start = Some(at),
                (false, Some(from)) => {
                    runs.push((from, at - from));
                    start = None;
                }
                _ => {}
            }
        }
        runs
    }
}

/// The code on its light tile, `side` wide and high.
pub fn tile(qr: &Rc<Qr>, side: Pixels) -> Div {
    let paper = rgb(0xF5F5F5);
    let ink = rgb(0x0A0A0A);
    let qr = qr.clone();
    div().flex_none().size(side).bg(paper).child(
        gpui_kit::canvas(
            |_, _, _| (),
            move |bounds: Bounds<Pixels>, _, window, _| {
                let module = bounds.size.width / (qr.side + 2 * QUIET) as f32;
                let origin = bounds.origin + point(module * QUIET as f32, module * QUIET as f32);
                for row in 0..qr.side {
                    for (from, len) in qr.runs(row) {
                        // A hair wider and taller than its cell, so neighbours
                        // leave no seam at a fractional scale.
                        let at = origin + point(module * from as f32, module * row as f32);
                        let extent = size(module * len as f32 + px(0.5), module + px(0.5));
                        window.paint_quad(fill(Bounds::new(at, extent), ink));
                    }
                }
            },
        )
        .size_full(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pairing_link_fits_a_small_code_with_its_three_finders() {
        let qr =
            Qr::of("leon://pair?code=ABCD-EFG-HJK&relay=wss%3A%2F%2Frelay.getleon.dev").unwrap();
        assert_eq!(qr.dark.len(), qr.side * qr.side);
        assert!(qr.side <= 41, "{}", qr.side);
        // A finder pattern starts with seven dark modules, in three corners.
        assert_eq!(qr.runs(0).first(), Some(&(0, 7)));
        assert_eq!(qr.runs(0).last(), Some(&(qr.side - 7, 7)));
        assert_eq!(qr.runs(qr.side - 1).first(), Some(&(0, 7)));
    }

    #[test]
    fn the_runs_of_a_row_cover_exactly_its_dark_modules() {
        let qr = Qr {
            text: String::new(),
            side: 6,
            dark: [true, true, false, true, false, false].repeat(6),
        };
        assert_eq!(qr.runs(0), vec![(0, 2), (3, 1)]);
        let all = Qr {
            text: String::new(),
            side: 2,
            dark: vec![true; 4],
        };
        assert_eq!(all.runs(1), vec![(0, 2)]);
    }
}
