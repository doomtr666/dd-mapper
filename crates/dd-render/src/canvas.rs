//! Drawing in cell coordinates over a `tiny-skia` pixmap.

use dd_domain::{Cell, CellPoint, CellPosition, GridSize};
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Mask, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform,
};

use crate::palette::Rgb;
use crate::{MAX_SIDE_PX, Preview, RenderError};

const OPAQUE: u8 = 255;

pub struct Canvas {
    pixmap: Pixmap,
    pixels_per_cell: f32,
}

impl Canvas {
    pub fn new(grid: GridSize, pixels_per_cell: u32) -> Result<Self, RenderError> {
        let width = u64::from(grid.width()) * u64::from(pixels_per_cell);
        let height = u64::from(grid.height()) * u64::from(pixels_per_cell);
        let too_large = || RenderError::TooLarge { width, height };
        let side = |pixels: u64| {
            u32::try_from(pixels)
                .ok()
                .filter(|pixels| (1..=MAX_SIDE_PX).contains(pixels))
        };
        let (Some(width_px), Some(height_px)) = (side(width), side(height)) else {
            return Err(too_large());
        };
        let pixmap = Pixmap::new(width_px, height_px).ok_or_else(too_large)?;
        #[expect(
            clippy::cast_precision_loss,
            reason = "pixels per cell stay far below 2^24"
        )]
        let pixels_per_cell = pixels_per_cell as f32;
        Ok(Self {
            pixmap,
            pixels_per_cell,
        })
    }

    pub fn into_preview(self) -> Preview {
        Preview {
            pixmap: self.pixmap,
        }
    }

    pub fn pixels_per_cell(&self) -> f32 {
        self.pixels_per_cell
    }

    pub fn fill_background(&mut self, rgb: Rgb) {
        let [red, green, blue] = rgb;
        self.pixmap
            .fill(Color::from_rgba8(red, green, blue, OPAQUE));
    }

    /// A mask letting paint through inside `rings` only.
    pub fn mask_inside<'a>(
        &self,
        rings: impl IntoIterator<Item = &'a [CellPoint]>,
    ) -> Option<Mask> {
        let mut mask = Mask::new(self.pixmap.width(), self.pixmap.height())?;
        for ring in rings {
            let positions: Vec<CellPosition> =
                ring.iter().map(|point| position_of(*point)).collect();
            if let Some(path) = self.closed_path(&positions) {
                mask.fill_path(&path, FillRule::Winding, true, Transform::identity());
            }
        }
        Some(mask)
    }

    /// Fills `cell`, only where `mask` lets paint through.
    pub fn fill_cell_within(&mut self, cell: Cell, rgb: Rgb, mask: &Mask) {
        let (left, top) = self.to_pixels(CellPosition::new(
            f64::from(cell.column),
            f64::from(cell.row),
        ));
        let side = self.pixels_per_cell;
        if let Some(rect) = Rect::from_xywh(left, top, side, side) {
            self.pixmap
                .fill_rect(rect, &paint(rgb, OPAQUE), Transform::identity(), Some(mask));
        }
    }

    pub fn fill_box(&mut self, top_left: CellPosition, bottom_right: CellPosition, rgb: Rgb) {
        let (left, top) = self.to_pixels(top_left);
        let (right, bottom) = self.to_pixels(bottom_right);
        if let Some(rect) = Rect::from_ltrb(left, top, right, bottom) {
            self.pixmap
                .fill_rect(rect, &paint(rgb, OPAQUE), Transform::identity(), None);
        }
    }

    pub fn stroke_box(
        &mut self,
        top_left: CellPosition,
        bottom_right: CellPosition,
        rgb: Rgb,
        width_px: f32,
    ) {
        let corners = [
            top_left,
            CellPosition::new(bottom_right.x, top_left.y),
            bottom_right,
            CellPosition::new(top_left.x, bottom_right.y),
        ];
        self.stroke_closed(&corners, rgb, width_px);
    }

    /// Fills a pixel-aligned rectangle, for bitmap text.
    pub fn fill_pixels(&mut self, left: f32, top: f32, size: (f32, f32), rgb: Rgb) {
        if let Some(rect) = Rect::from_xywh(left, top, size.0, size.1) {
            self.pixmap
                .fill_rect(rect, &paint(rgb, OPAQUE), Transform::identity(), None);
        }
    }

    pub fn fill_ring(&mut self, ring: &[CellPoint], rgb: Rgb) {
        self.fill_ring_with_alpha(ring, rgb, OPAQUE);
    }

    pub fn fill_ring_with_alpha(&mut self, ring: &[CellPoint], rgb: Rgb, alpha: u8) {
        let positions: Vec<CellPosition> = ring.iter().map(|point| position_of(*point)).collect();
        if let Some(path) = self.closed_path(&positions) {
            self.pixmap.fill_path(
                &path,
                &paint(rgb, alpha),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    /// Strokes an open polyline through `points`.
    pub fn stroke_polyline(&mut self, points: &[CellPoint], rgb: Rgb, width_px: f32) {
        let mut builder = PathBuilder::new();
        for (index, point) in points.iter().enumerate() {
            let (x, y) = self.to_pixels(position_of(*point));
            if index == 0 {
                builder.move_to(x, y);
            } else {
                builder.line_to(x, y);
            }
        }
        let Some(path) = builder.finish() else {
            return;
        };
        let stroke = Stroke {
            width: width_px,
            ..Stroke::default()
        };
        self.pixmap.stroke_path(
            &path,
            &paint(rgb, OPAQUE),
            &stroke,
            Transform::identity(),
            None,
        );
    }

    /// A filled disc; `radius` in cells.
    pub fn fill_circle(&mut self, center: CellPosition, radius: f64, rgb: Rgb) {
        if let Some(path) = self.circle(center, radius) {
            self.pixmap.fill_path(
                &path,
                &paint(rgb, OPAQUE),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    /// A circle outline; `radius` in cells.
    pub fn stroke_circle(&mut self, center: CellPosition, radius: f64, rgb: Rgb, style: LineStyle) {
        if let Some(path) = self.circle(center, radius) {
            let stroke = Stroke {
                width: style.width_px,
                ..Stroke::default()
            };
            self.pixmap.stroke_path(
                &path,
                &paint(rgb, style.alpha),
                &stroke,
                Transform::identity(),
                None,
            );
        }
    }

    fn circle(&self, center: CellPosition, radius: f64) -> Option<tiny_skia::Path> {
        let (x, y) = self.to_pixels(center);
        let (edge, _) = self.to_pixels(CellPosition::new(center.x + radius, center.y));
        PathBuilder::from_circle(x, y, edge - x)
    }

    pub fn stroke_ring(&mut self, ring: &[CellPoint], rgb: Rgb, width_px: f32) {
        let positions: Vec<CellPosition> = ring.iter().map(|point| position_of(*point)).collect();
        self.stroke_closed(&positions, rgb, width_px);
    }

    /// An open line through `points`, with round joins and ends.
    pub fn stroke_through(&mut self, points: &[CellPosition], rgb: Rgb, style: LineStyle) {
        let Some((first, others)) = points.split_first() else {
            return;
        };
        let mut builder = PathBuilder::new();
        let (x, y) = self.to_pixels(*first);
        builder.move_to(x, y);
        for point in others {
            let (x, y) = self.to_pixels(*point);
            builder.line_to(x, y);
        }
        let Some(path) = builder.finish() else {
            return;
        };
        let stroke = Stroke {
            width: style.width_px,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };
        self.pixmap.stroke_path(
            &path,
            &paint(rgb, style.alpha),
            &stroke,
            Transform::identity(),
            None,
        );
    }

    pub fn line(&mut self, from: CellPosition, to: CellPosition, rgb: Rgb, style: LineStyle) {
        let mut builder = PathBuilder::new();
        let (from_x, from_y) = self.to_pixels(from);
        let (to_x, to_y) = self.to_pixels(to);
        builder.move_to(from_x, from_y);
        builder.line_to(to_x, to_y);
        let Some(path) = builder.finish() else {
            return;
        };
        let stroke = Stroke {
            width: style.width_px,
            line_cap: LineCap::Butt,
            ..Stroke::default()
        };
        self.pixmap.stroke_path(
            &path,
            &paint(rgb, style.alpha),
            &stroke,
            Transform::identity(),
            None,
        );
    }

    pub fn to_pixels(&self, position: CellPosition) -> (f32, f32) {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "map coordinates fit in f32"
        )]
        let scale = |coordinate: f64| coordinate as f32 * self.pixels_per_cell;
        (scale(position.x), scale(position.y))
    }

    fn stroke_closed(&mut self, corners: &[CellPosition], rgb: Rgb, width_px: f32) {
        if let Some(path) = self.closed_path(corners) {
            let stroke = Stroke {
                width: width_px,
                ..Stroke::default()
            };
            self.pixmap.stroke_path(
                &path,
                &paint(rgb, OPAQUE),
                &stroke,
                Transform::identity(),
                None,
            );
        }
    }

    fn closed_path(&self, corners: &[CellPosition]) -> Option<tiny_skia::Path> {
        let (first, others) = corners.split_first()?;
        let mut builder = PathBuilder::new();
        let (x, y) = self.to_pixels(*first);
        builder.move_to(x, y);
        for corner in others {
            let (x, y) = self.to_pixels(*corner);
            builder.line_to(x, y);
        }
        builder.close();
        builder.finish()
    }
}

/// Width and opacity of a line.
#[derive(Debug, Clone, Copy)]
pub struct LineStyle {
    pub width_px: f32,
    pub alpha: u8,
}

impl LineStyle {
    pub fn opaque(width_px: f32) -> Self {
        Self {
            width_px,
            alpha: OPAQUE,
        }
    }
}

fn position_of(point: CellPoint) -> CellPosition {
    CellPosition::new(f64::from(point.x), f64::from(point.y))
}

fn paint(rgb: Rgb, alpha: u8) -> Paint<'static> {
    let [red, green, blue] = rgb;
    let mut paint = Paint::default();
    paint.set_color_rgba8(red, green, blue, alpha);
    paint.anti_alias = true;
    paint
}
