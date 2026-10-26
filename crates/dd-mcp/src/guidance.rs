//! Texts guiding the model: server instructions and the design prompt.

pub const INSTRUCTIONS: &str = "\
Designs Dungeondraft maps on a grid of cells (one cell = 5 feet). \
Find assets with catalog_search, create a map with map_new (or open one saved by \
Dungeondraft with map_open, or draw a whole building with layout_generate), then add rooms, \
partitions (wall_add), doors, props, lights (light_add) and floor patterns (pattern_add). Check your work \
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
partitions and doors at once; try other seeds until the layout fits, then go \
to step 6.
4. Add rooms with room_add. Overlapping or touching rooms merge into one, like \
in Dungeondraft: use that to shape a building, its corridors and wings. Use \
pillars for columns and solid blocks.
5. Split the building into rooms with wall_add: partitions are open walls \
that never merge. Then add doors with door_add, on a room's outer walls (R0) \
or on a partition (W0), at a point on that wall.
6. Call map_preview and map_describe. Fix what looks wrong before going on.
7. Furnish with prop_place: consoles, crates, aquilas, cogitators... Search \
objects with catalog_search; footprints are real sizes in cells, so keep props \
inside rooms and off doorways. Light the scene with light_add: caged lamps, \
glowing tanks, window shafts (search kind light); a cold blue or a sickly green \
sets the underhive mood. Add floor details with pattern_add: sewer grates, vent fans, hazard markings (search kind pattern). Preview again.
8. Run map_validate, address the warnings, then map_save to a \
.dungeondraft_map file and give its path."
    )
}
