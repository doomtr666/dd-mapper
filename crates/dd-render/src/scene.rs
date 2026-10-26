//! The layers of the preview, drawn bottom to top.

use dd_domain::{CellPosition, Door, GridSize, Map, Partition};

use crate::canvas::{Canvas, LineStyle};
use crate::font;
use crate::palette::{
    DOOR, GRID_ALPHA, LABEL_BACKGROUND, LABEL_TEXT, LIGHT, LIGHT_RANGE_ALPHA, PATTERN_ALPHA,
    PILLAR, PROP_FILL, PROP_OUTLINE, Rgb, VOID, WALL, floor_color, pattern_color,
};
use crate::{Preview, PreviewOptions, RenderError};

/// Wall thickness, as a share of a cell.
const WALL_THICKNESS: f32 = 0.12;
const MIN_WALL_PX: f32 = 2.0;
/// Doors are drawn this much thicker than walls, to stand out.
const DOOR_THICKNESS_FACTOR: f32 = 2.5;
/// Schematic door length, in cells.
const DOOR_LENGTH: f64 = 1.0;
const PROP_OUTLINE_PX: f32 = 1.5;
const GRID_LINE_PX: f32 = 1.0;
const BLACK: Rgb = [0, 0, 0];
/// Door labels sit this far inside the room, in cells.
const DOOR_LABEL_OFFSET: f64 = 0.6;
/// Radius of the dot marking a light, in cells.
const LIGHT_DOT_RADIUS: f64 = 0.2;
/// Light labels sit this far below the light, in cells.
const LIGHT_LABEL_OFFSET: f64 = 0.5;
/// Partition labels sit this far along the first segment.
const PARTITION_LABEL_ALONG: f64 = 0.2;
/// Label dots per cell, so labels scale with the preview.
const LABEL_DOTS_PER_CELL: f32 = 16.0;

/// Draws `map` as a schematic preview.
///
/// # Errors
/// Fails when the image would exceed [`crate::MAX_SIDE_PX`] per side.
pub fn render(map: &Map, options: &PreviewOptions) -> Result<Preview, RenderError> {
    let mut canvas = Canvas::new(map.grid(), options.pixels_per_cell)?;
    canvas.fill_background(VOID);
    draw_floor(&mut canvas, map);
    draw_patterns(&mut canvas, map);
    draw_pillars(&mut canvas, map);
    if options.grid {
        draw_grid(&mut canvas, map.grid());
    }
    draw_walls(&mut canvas, map);
    draw_partitions(&mut canvas, map);
    draw_doors(&mut canvas, map);
    draw_props(&mut canvas, map);
    draw_lights(&mut canvas, map);
    if options.labels {
        draw_labels(&mut canvas, map);
    }
    Ok(canvas.into_preview())
}

fn wall_width(canvas: &Canvas) -> f32 {
    (canvas.pixels_per_cell() * WALL_THICKNESS).max(MIN_WALL_PX)
}

/// Floor tiles, cut along the room outlines like Dungeondraft does.
fn draw_floor(canvas: &mut Canvas, map: &Map) {
    let outlines = map.rooms().map(|(_, room)| room.outline().vertices());
    let Some(inside_rooms) = canvas.mask_inside(outlines) else {
        return;
    };
    for (cell, tileset) in map.floor() {
        canvas.fill_cell_within(cell, floor_color(tileset), &inside_rooms);
    }
}

fn draw_patterns(canvas: &mut Canvas, map: &Map) {
    for (_, pattern) in map.patterns() {
        canvas.fill_ring_with_alpha(
            pattern.area.vertices(),
            pattern_color(&pattern.asset),
            PATTERN_ALPHA,
        );
    }
}

fn draw_pillars(canvas: &mut Canvas, map: &Map) {
    for (_, room) in map.rooms() {
        for pillar in room.pillars() {
            canvas.fill_ring(pillar.vertices(), PILLAR);
        }
    }
}

fn draw_grid(canvas: &mut Canvas, grid: GridSize) {
    let style = LineStyle {
        width_px: GRID_LINE_PX,
        alpha: GRID_ALPHA,
    };
    let (width, height) = (f64::from(grid.width()), f64::from(grid.height()));
    for column in 0..=grid.width() {
        let x = f64::from(column);
        let (top, bottom) = (CellPosition::new(x, 0.0), CellPosition::new(x, height));
        canvas.line(top, bottom, BLACK, style);
    }
    for row in 0..=grid.height() {
        let y = f64::from(row);
        let (left, right) = (CellPosition::new(0.0, y), CellPosition::new(width, y));
        canvas.line(left, right, BLACK, style);
    }
}

fn draw_walls(canvas: &mut Canvas, map: &Map) {
    let width = wall_width(canvas);
    for (_, room) in map.rooms() {
        for (_, ring) in room.wall_loops() {
            canvas.stroke_ring(ring.vertices(), WALL, width);
        }
    }
}

fn draw_partitions(canvas: &mut Canvas, map: &Map) {
    let width = wall_width(canvas);
    for (_, partition) in map.partitions() {
        canvas.stroke_polyline(partition.points(), WALL, width);
    }
}

fn draw_doors(canvas: &mut Canvas, map: &Map) {
    let style = LineStyle::opaque(wall_width(canvas) * DOOR_THICKNESS_FACTOR);
    for (_, door) in map.doors() {
        let Some((center, (dx, dy))) = door_frame(map, door) else {
            continue;
        };
        let half = DOOR_LENGTH / 2.0;
        let from = CellPosition::new(center.x - dx * half, center.y - dy * half);
        let to = CellPosition::new(center.x + dx * half, center.y + dy * half);
        canvas.line(from, to, DOOR, style);
    }
}

fn draw_props(canvas: &mut Canvas, map: &Map) {
    for (_, prop) in map.props() {
        let (top_left, bottom_right) = prop.bounds();
        canvas.fill_box(top_left, bottom_right, PROP_FILL);
        canvas.stroke_box(top_left, bottom_right, PROP_OUTLINE, PROP_OUTLINE_PX);
    }
}

/// A bright dot, and the reach of the light as a translucent circle.
fn draw_lights(canvas: &mut Canvas, map: &Map) {
    let style = LineStyle {
        width_px: wall_width(canvas),
        alpha: LIGHT_RANGE_ALPHA,
    };
    for (_, light) in map.lights() {
        canvas.stroke_circle(light.center, light.glow.range(), LIGHT, style);
        canvas.fill_circle(light.center, LIGHT_DOT_RADIUS, LIGHT);
    }
}

fn draw_labels(canvas: &mut Canvas, map: &Map) {
    for (id, room) in map.rooms() {
        if let Some(cell) = room.floor_cells().next() {
            let center = CellPosition::new(f64::from(cell.column) + 0.5, f64::from(cell.row) + 0.5);
            draw_label(canvas, &format!("R{}", id.0), center);
        }
    }
    for (id, door) in map.doors() {
        if let Some((center, (dx, dy))) = door_frame(map, door) {
            let inside = CellPosition::new(
                center.x - dy * DOOR_LABEL_OFFSET,
                center.y + dx * DOOR_LABEL_OFFSET,
            );
            draw_label(canvas, &format!("D{}", id.0), inside);
        }
    }
    for (id, prop) in map.props() {
        draw_label(canvas, &format!("P{}", id.0), prop.center);
    }
    for (id, light) in map.lights() {
        let below = CellPosition::new(light.center.x, light.center.y + LIGHT_LABEL_OFFSET);
        draw_label(canvas, &format!("L{}", id.0), below);
    }
    for (id, partition) in map.partitions() {
        if let Some(position) = partition_label_position(partition) {
            draw_label(canvas, &format!("W{}", id.0), position);
        }
    }
}

/// Where a partition's id goes: along its first segment, away from the
/// middle where doors usually stand.
fn partition_label_position(partition: &Partition) -> Option<CellPosition> {
    partition
        .segment(0)
        .map(|first| first.point_at(PARTITION_LABEL_ALONG))
}

/// Bitmap text on a dark plate, centered on `center`.
fn draw_label(canvas: &mut Canvas, text: &str, center: CellPosition) {
    let dot = (canvas.pixels_per_cell() / LABEL_DOTS_PER_CELL)
        .floor()
        .max(1.0);
    #[expect(clippy::cast_precision_loss, reason = "label sizes are tiny")]
    let (width, height) = (
        font::text_width(text) as f32 * dot,
        font::HEIGHT as f32 * dot,
    );
    let (center_x, center_y) = canvas.to_pixels(center);
    let (left, top) = (
        (center_x - width / 2.0).round(),
        (center_y - height / 2.0).round(),
    );
    canvas.fill_pixels(
        left - dot,
        top - dot,
        (width + 2.0 * dot, height + 2.0 * dot),
        LABEL_BACKGROUND,
    );
    for (column, row) in font::lit_dots(text) {
        #[expect(clippy::cast_precision_loss, reason = "label sizes are tiny")]
        let (x, y) = (left + column as f32 * dot, top + row as f32 * dot);
        canvas.fill_pixels(x, y, (dot, dot), LABEL_TEXT);
    }
}

/// Where the door stands, and the unit direction of its wall.
fn door_frame(map: &Map, door: &Door) -> Option<(CellPosition, (f64, f64))> {
    let segment = map.wall_segment(&door.position)?;
    let center = segment.point_at(door.position.t);
    let dx = f64::from(segment.end.x - segment.start.x);
    let dy = f64::from(segment.end.y - segment.start.y);
    let length = dx.hypot(dy);
    Some((center, (dx / length, dy / length)))
}

#[cfg(test)]
mod tests {
    use dd_domain::{
        AssetPath, CellPoint, CellPosition, Door, Footprint, Glow, GridSize, Light, Map, Partition,
        Pattern, Polygon, Prop, QuarterTurn, Room, WallLoop, WallPosition, WallRef,
    };

    use super::render;
    use crate::palette::{
        DOOR, LABEL_BACKGROUND, LABEL_TEXT, LIGHT, PILLAR, PROP_FILL, VOID, WALL, floor_color,
    };
    use crate::{Preview, PreviewOptions, RenderError};

    const PIXELS_PER_CELL: u32 = 20;

    fn asset(path: &str) -> AssetPath {
        AssetPath::new(path).unwrap()
    }

    /// A 6 x 5 room at (1, 1) with a pillar at (3, 3), a door in the middle
    /// of its bottom wall and a 1 x 1 crate centered at (5.5, 2.5).
    pub fn reference_map() -> Map {
        let mut map = Map::new(GridSize::new(10, 8).unwrap());
        let mut room = Room::new(
            Polygon::rectangle(CellPoint::new(1, 1), 6, 5).unwrap(),
            asset("wall.webp"),
        );
        room.add_pillar(Polygon::rectangle(CellPoint::new(3, 3), 1, 1).unwrap())
            .unwrap();
        let room = map.add_room(room, &asset("floor.webp")).unwrap().id;
        let position = WallPosition {
            wall: WallRef::Room {
                room,
                wall: WallLoop::Outline,
            },
            segment: 0,
            t: 0.5,
        };
        map.add_door(Door {
            position,
            asset: asset("door.webp"),
        })
        .unwrap();
        map.add_prop(Prop {
            asset: asset("crate.webp"),
            center: CellPosition::new(5.5, 2.5),
            rotation: QuarterTurn::Deg0,
            footprint: Footprint::new(1.0, 1.0).unwrap(),
        })
        .unwrap();
        map
    }

    fn plain_options() -> PreviewOptions {
        PreviewOptions {
            pixels_per_cell: PIXELS_PER_CELL,
            grid: false,
            labels: false,
        }
    }

    fn plain_preview() -> Preview {
        render(&reference_map(), &plain_options()).unwrap()
    }

    /// The pixel at a position given in cells.
    fn at(preview: &Preview, x: f64, y: f64) -> [u8; 3] {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let pixel = |cells: f64| (cells * f64::from(PIXELS_PER_CELL)) as u32;
        preview.rgb(pixel(x), pixel(y)).unwrap()
    }

    #[test]
    fn covers_the_whole_map() {
        let preview = plain_preview();
        assert_eq!((preview.width(), preview.height()), (200, 160));
        assert_eq!(at(&preview, 0.5, 0.5), VOID);
        assert_eq!(at(&preview, 9.5, 7.5), VOID);
    }

    #[test]
    fn paints_floors_pillars_and_walls() {
        let preview = plain_preview();
        assert_eq!(at(&preview, 2.5, 4.5), floor_color(&asset("floor.webp")));
        assert_eq!(at(&preview, 3.5, 3.5), PILLAR);
        assert_eq!(at(&preview, 1.0, 3.5), WALL);
        assert_eq!(at(&preview, 4.5, 1.0), WALL);
    }

    #[test]
    fn floors_stop_at_oblique_walls() {
        let mut map = Map::new(GridSize::new(6, 6).unwrap());
        let corners = [(0, 0), (4, 0), (4, 2), (2, 4), (0, 4)]
            .into_iter()
            .map(|(x, y)| CellPoint::new(x, y))
            .collect();
        let room = Room::new(Polygon::new(corners).unwrap(), asset("wall.webp"));
        map.add_room(room, &asset("floor.webp")).unwrap();
        let preview = render(&map, &plain_options()).unwrap();
        assert_eq!(at(&preview, 3.2, 2.2), floor_color(&asset("floor.webp")));
        assert_eq!(at(&preview, 3.8, 2.8), VOID);
    }

    #[test]
    fn draws_partitions_and_their_doors() {
        let mut map = reference_map();
        let points = vec![CellPoint::new(5, 1), CellPoint::new(5, 6)];
        let partition = map
            .add_partition(Partition::new(points, asset("wall.webp")).unwrap())
            .unwrap();
        let position = WallPosition {
            wall: WallRef::Partition(partition),
            segment: 0,
            t: 0.7,
        };
        map.add_door(Door {
            position,
            asset: asset("door.webp"),
        })
        .unwrap();
        let preview = render(&map, &plain_options()).unwrap();
        assert_eq!(at(&preview, 5.0, 1.5), WALL);
        assert_eq!(at(&preview, 5.0, 4.5), DOOR);
    }

    #[test]
    fn partition_labels_leave_the_middle_to_doors() {
        let points = vec![CellPoint::new(14, 4), CellPoint::new(14, 10)];
        let partition = Partition::new(points, asset("wall.webp")).unwrap();
        let position = super::partition_label_position(&partition).unwrap();
        assert!((position.x - 14.0).abs() < 1e-9);
        assert!((position.y - 5.2).abs() < 1e-9, "{position:?}");
    }

    #[test]
    fn marks_lights_and_their_range() {
        let mut map = reference_map();
        map.add_light(Light {
            asset: asset("light.webp"),
            center: CellPosition::new(2.5, 4.5),
            glow: Glow::new(1.5, 1.0, 0xffec_cd8b).unwrap(),
        })
        .unwrap();
        let preview = render(&map, &plain_options()).unwrap();
        assert_eq!(at(&preview, 2.5, 4.5), LIGHT);
        let floor = floor_color(&asset("floor.webp"));
        let on_range = at(&preview, 4.0, 4.5);
        assert_ne!(on_range, floor);
        assert_eq!(at(&preview, 4.5, 4.5), floor);
    }

    #[test]
    fn tints_patterned_floor() {
        let mut map = reference_map();
        map.add_pattern(Pattern {
            asset: asset("grate.webp"),
            area: Polygon::rectangle(CellPoint::new(2, 4), 1, 1).unwrap(),
            tint: 0xffff_ffff,
        })
        .unwrap();
        let preview = render(&map, &plain_options()).unwrap();
        let floor = floor_color(&asset("floor.webp"));
        assert_ne!(at(&preview, 2.5, 4.5), floor);
        assert_eq!(at(&preview, 1.5, 4.5), floor);
    }

    #[test]
    fn marks_doors_across_their_wall() {
        let preview = plain_preview();
        assert_eq!(at(&preview, 4.0, 6.0), DOOR);
        assert_eq!(at(&preview, 4.3, 6.0), DOOR);
        assert_eq!(at(&preview, 5.5, 6.0), WALL);
    }

    #[test]
    fn draws_props_as_boxes() {
        let preview = plain_preview();
        assert_eq!(at(&preview, 5.5, 2.5), PROP_FILL);
        assert_ne!(at(&preview, 5.5, 3.5), PROP_FILL);
    }

    #[test]
    fn grid_lines_darken_the_floor() {
        let options = PreviewOptions {
            grid: true,
            ..plain_options()
        };
        let preview = render(&reference_map(), &options).unwrap();
        let floor = floor_color(&asset("floor.webp"));
        let on_line = at(&preview, 3.0, 4.5);
        assert!(
            on_line.iter().zip(floor).all(|(line, floor)| *line < floor),
            "{on_line:?}"
        );
        assert_eq!(at(&preview, 2.5, 4.5), floor);
    }

    #[test]
    fn labels_rooms_doors_and_props() {
        let options = PreviewOptions {
            labels: true,
            ..plain_options()
        };
        let labelled = render(&reference_map(), &options).unwrap();
        let plain = plain_preview();
        let changed = labelled
            .rgba()
            .chunks(4)
            .zip(plain.rgba().chunks(4))
            .filter(|(labelled, plain)| labelled != plain)
            .count();
        assert!(changed > 100, "{changed} pixels changed");
        let colors: std::collections::HashSet<[u8; 3]> = (0..labelled.width())
            .flat_map(|x| (0..labelled.height()).map(move |y| (x, y)))
            .filter_map(|(x, y)| labelled.rgb(x, y))
            .collect();
        assert!(colors.contains(&LABEL_TEXT) && colors.contains(&LABEL_BACKGROUND));
    }

    #[test]
    fn encodes_a_png_holding_the_same_pixels() {
        let preview = plain_preview();
        let png = preview.to_png().unwrap();
        let decoded = tiny_skia::Pixmap::decode_png(&png).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (200, 160));
        assert_eq!(decoded.data(), preview.rgba());
    }

    #[test]
    fn reference_scene_snapshot() {
        let preview = render(&reference_map(), &PreviewOptions::default()).unwrap();
        let hash = crate::palette::fnv1a(preview.rgba());
        insta::assert_snapshot!(format!(
            "{} x {} px, rgba fnv1a {hash:016x}",
            preview.width(),
            preview.height()
        ));
    }

    #[test]
    fn small_props_stay_visible_under_their_label() {
        let mut map = Map::new(GridSize::new(4, 4).unwrap());
        map.add_prop(Prop {
            asset: asset("crate.webp"),
            center: CellPosition::new(2.0, 2.0),
            rotation: QuarterTurn::Deg0,
            footprint: Footprint::new(1.01, 0.63).unwrap(),
        })
        .unwrap();
        let preview = render(&map, &PreviewOptions::default()).unwrap();
        let visible_fill = (48..80)
            .flat_map(|x| (54..74).map(move |y| (x, y)))
            .filter(|&(x, y)| preview.rgb(x, y) == Some(PROP_FILL))
            .count();
        assert!(visible_fill > 50, "only {visible_fill} prop pixels visible");
    }

    #[test]
    fn refuses_previews_too_large_to_send() {
        let map = Map::new(GridSize::new(400, 10).unwrap());
        let options = PreviewOptions {
            pixels_per_cell: 32,
            ..PreviewOptions::default()
        };
        assert_eq!(
            render(&map, &options).unwrap_err(),
            RenderError::TooLarge {
                width: 12_800,
                height: 320
            }
        );
    }
}
