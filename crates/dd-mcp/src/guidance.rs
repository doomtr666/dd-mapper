//! Texts guiding the model: server instructions and the design prompt.

pub const INSTRUCTIONS: &str = "\
Designs Dungeondraft maps on a grid of cells (one cell = 5 feet). \
Find assets with catalog_search, create a map with map_new (or open one saved by \
Dungeondraft with map_open, or draw a whole building with layout_generate), then add rooms, \
partitions (wall_add), doors, props, lights (light_add), floor patterns (pattern_add), \
clutter (prop_scatter), a floor per room (floor_paint) and a background \
(map_background), and dim the ambient light (map_ambient). Check your work \
with map_preview (an image with R/W/D/P/L ids) and map_describe, fix it, run map_validate, and \
finish with map_save. Read the catalog://summary resource to know which packs \
are available.";

pub const DESIGN_PROMPT_NAME: &str = "design_40k_location";

pub const DESIGN_PROMPT_DESCRIPTION: &str =
    "Guides the design of a Warhammer 40k (Imperium Maledictum) location, step by step.";

/// The design workflow for `brief`.
pub fn design_prompt(brief: &str) -> String {
    format!(
        "Design a Dungeondraft map for this Warhammer 40,000 location, for an \
Imperium Maledictum game: {brief}.

Work in this order, and look at the map after each step:
1. Plan first: list the areas, their purpose, size in cells (1 cell = 5 ft) and \
how they connect. Keep corridors at least 2 cells wide.
2. Find assets with catalog_search: a floor tileset (kind tileset), a wall \
texture (kind wall) and doors (kind portal: door assets also exist as objects, \
which cannot go in walls). Prefer the gothic and industrial LORE packs for the \
Imperium, and keep one wall style per structure.
3. Create the map with map_new, a little larger than the plan. For a generic \
building (hab block, ship deck, bunker), layout_generate draws the walls, \
partitions and doors at once: hall=true keeps a large hall between two wings \
(a production floor, a nave, a mess), margin leaves room around it (engines \
behind a ship). Try other seeds until the layout fits, then go to step 6.
4. Caves: cave_generate grows a network of caverns over an area, cave_carve \
digs chambers and tunnels with natural walls (or fills them back), and \
cave_entrance opens the walls where a path enters. Props on cave floors need no \
room. Then add rooms with room_add. Overlapping or touching rooms merge into one, like \
in Dungeondraft: use that to shape a building, its corridors and wings. Use \
pillars for columns and solid blocks.
5. Split the building into rooms with wall_add: partitions are open walls \
that never merge. Then add doors with door_add, on a room's outer walls (R0) \
or on a partition (W0), at a point on that wall.
6. Call map_preview and map_describe. Give every room a purpose before \
furnishing it: an empty room reads as bare floor.
7. Floors: give rooms their own tileset with floor_paint, within one family of \
materials (worn, clean, grated variants of the same style), never a patchwork.
8. Furnish with prop_place: the piece that tells the room's purpose first \
(a machine, a cogitator bank, bunks, an altar), then line the walls with \
prop_scatter along_walls=true (lockers, shelves, barrels, pipes, crates). \
Footprints are real sizes in cells; keep doorways free. Seats and consoles of \
the LORE packs face down (south) at 0 degrees, west at 90, north at 180 and \
east at 270; to turn a prop, prop_remove it and place it again.
9. Light: dim the ambient light with map_ambient (404858 for an underhive, \
202838 for a derelict), then put a light in every room with light_add. Prefer \
plain glows such as res://textures/lights/soft.png: caged and window lights \
project their pattern on the floor. A cold blue or a sickly green sets the \
underhive mood; keep intensities around 0.6 to 1.2.
10. Details: conveyor belts, cables and pipes with path_add (search kind \
path); floor patterns with pattern_add for man-made shapes only (grates, \
hazard stripes): a pattern keeps the straight edges of its shape, so make spills \
and puddles with organic props instead: blood_water_stain props given a color \
(5aff3c for acid, 3a2a1a for oil), or swamp_puddle. Then \
clutter with prop_scatter (debris piles, papers, tools, bones of at least half \
a cell: smaller ones barely show), and a background with map_background \
(terrain_void for space, rock or ash outdoors). Preview again.
11. Run map_validate, address the warnings, then map_save to a \
.dungeondraft_map file and give its path."
    )
}
