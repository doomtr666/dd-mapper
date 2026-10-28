# dd-mapper

Designs [Dungeondraft](https://dungeondraft.net/) 1.2 maps on a grid of cells and writes
`.dungeondraft_map` files that Dungeondraft opens as if they were its own. It works with
Dungeondraft's own assets and with any asset pack whose author allows third-party tools to
read it, in any setting. It can be driven in two ways:

- **`dd-mcp`**, an [MCP](https://modelcontextprotocol.io/) server: an LLM such as Claude
  plans the map, adds rooms, partitions, doors, props, lights and patterns, and looks at
  schematic previews as it works.
- **`dd-mapper`**, a command line: builds maps from TOML plans and checks, previews or
  searches assets, without an LLM.

One cell is 5 feet (256 pixels in Dungeondraft). Positions are in cells, and corners sit
on grid vertices: `(0, 0)` is the top-left corner of the map.

## Examples

The plans in [`examples/`](examples) build complete maps with Dungeondraft's own assets
only. Each one is a TOML file of a few hundred lines: rooms, walls, doors, painted ground,
props, scattered clutter and lights, all positioned in cells. Each map below was built by
one command, then opened in Dungeondraft and exported.

```sh
dd-mapper generate examples/fantasy_inn.toml --out fantasy_inn.dungeondraft_map \
  --preview fantasy_inn.png --packs assets/packs
```

`--packs` points at the folder holding `Dungeondraft.pck` (see
[Asset sources](#asset-sources)); `--preview` also writes the schematic view.

### The Crooked Lantern: a village inn at night

![The Crooked Lantern, rendered by Dungeondraft](images/fantasy_inn.jpg)

[`examples/fantasy_inn.toml`](examples/fantasy_inn.toml), 36 × 26 cells. A timber inn
split by partitions into five rooms, each with its own floor and purpose:

- **The common room**: the hearth with a bear-skin rug, a bar with stools and kegs
  behind it, four round tables with candles, cards, plates and a joint of meat.
- **The snug**: a quiet table with a chess board between two armchairs.
- **The stair landing**, **the kitchen** (flagstones, oven, stove, a pig on the spit, crates
  of produce along the walls) and **the storeroom** (worn boards, great casks, the cellar
  trapdoor, cobwebs in the corners).
- **Outside**: the road worn to dirt with cart ruts, the path to the door, flowers and
  bushes along the walls, the fenced stable yard with its cart, hay and well.

It shows how light sets a mood: a dark night ambient, and one small warm light for each
fire: the hearth, a candle on every table, the lamps on the bar, the oven, the lantern
over the door, firelight spilling out of the windows, and pale moonlight on the yard.
Colorable assets (armchairs, tables, carpet, cart, flowers) are each given a color.

### Ambush on the old toll road: a forest encounter

![Ambush on the old toll road, rendered by Dungeondraft](images/forest_ambush.jpg)

[`examples/forest_ambush.toml`](examples/forest_ambush.toml), 36 × 26 cells, all outdoors:
a background terrain and painted ground, no rooms.

- **On the road**: a felled pine blocks the way; the merchant's wagon lies smashed just
  before it, the ox dead in the traces, the cargo strewn and arrows everywhere.
- **The bandits' cover**: boulders, thorns and roots along the ridge north of the road.
- **Their camp** (north-west), in a trampled clearing: two tents, the fire with a spit,
  bedrolls, the loot and a prisoner in a cage.
- **The standing stones** (south-east), reached by a footpath: a ring of stones on bare
  rock around glowing mushrooms.
- **The wood**: pines and an autumn tree placed by hand, ferns, leaves and grass scattered,
  moss painted under the trees, fading into the grass.

### The schematic views

| The inn | The ambush |
|---|---|
| ![Schematic view of the inn](images/fantasy_inn_preview.png) | ![Schematic view of the ambush](images/forest_ambush_preview.png) |

`dd-mapper` and the MCP server draw these themselves, without the asset images: they are
how an LLM sees the map while it builds it, and how you check a plan before opening
Dungeondraft. Each element carries the id that the tools and plans use:

- **R** rooms, their floor tinted by tileset, and **W** free-standing walls, drawn in black;
- **D** doors and windows, in orange on the walls;
- **P** props, as blue boxes at their real footprint;
- **L** lights, a yellow dot with a circle for their range;
- **T** paths (roads, ruts, fences), as bands.

Floor patterns show as tints without a label; painted ground is not drawn. Open the map
in Dungeondraft to see them.

## Build

Rust 1.88 or later:

```sh
cargo build --release
```

This builds `target/release/dd-mcp` and `target/release/dd-mapper` (with `.exe` on Windows).

## Asset sources

Both binaries take the same options to know which assets exist:

| Option | Repeatable | What it reads |
| --- | --- | --- |
| `--packs <dir\|file>` | yes | `.dungeondraft_pack` files and `Dungeondraft.pck`: asset names, tags and image sizes |
| `--maps <dir\|file>` | yes | saved `.dungeondraft_map` files, to learn door widths and prop sizes |
| `--catalog <file>` | no | hand-written notes in `catalog.toml` (see below) |
| `--template <file>` | no | a map saved by Dungeondraft, used as the skeleton of new maps |

Packs are read only for their file index, `pack.json`, tags and image headers (which give
asset sizes). Image content is never extracted or copied. If a pack's `pack.json` does not
allow third-party mapping software to read it, loading stops with an error that names the
pack. Packs are licensed: keep them
out of the repository. `.gitignore` already excludes them.

Later sources refine earlier ones: packs, then maps, then `catalog.toml`.

### `catalog.toml`

```toml
[[asset]]
path = "res://packs/LOREGD01/textures/portals/Door_Underhive_Industrial.webp"
tags = ["door", "industrial"]
size = [1.2, 0.3]       # width and height, in cells
anchor = "wall"         # or "center"
portal_radius = 154     # half the door width, in Dungeondraft pixels
```

## Use with Claude (MCP)

The server speaks MCP over stdio.

### Claude Code

The repository ships a `.mcp.json` that starts the server from this checkout: build
it with `cargo build --release`, put your packs in `assets/packs`, and open Claude Code
at the root of the repository. Paths are relative to that root:

```json
{
  "mcpServers": {
    "dd-mapper": {
      "command": "target/release/dd-mcp",
      "args": ["--packs", "assets/packs", "--maps", "assets/maps"]
    }
  }
}
```

To use it from another project, add it there with absolute paths:

```sh
claude mcp add dd-mapper --scope project -- /path/to/dd-mcp --packs /path/to/packs --maps /path/to/maps
```

### Claude Desktop

Add the same `mcpServers` entry to `claude_desktop_config.json`, with absolute paths
(Claude Desktop does not start servers from a project directory):

- Windows: `%APPDATA%\Claude\claude_desktop_config.json`
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`

Then restart Claude Desktop.

> On Windows, a running server locks `dd-mcp.exe`. Disable the server in your client
> (or close the client) before rebuilding, then reconnect so that the client sees the new
> tool schemas.

### Tools, resource and prompt

| Tool | Does |
| --- | --- |
| `catalog_search` | finds assets by words, kind, tags or pack; gives paths and sizes in cells |
| `map_new` | creates an empty map |
| `map_open` | opens a map saved by Dungeondraft; saving keeps what the server does not model |
| `layout_generate` | draws a whole building on a new map: `bsp`, `hab_block` or `ship_deck` |
| `terrain_paint` | paints a terrain texture over the ground, fading into what lies around |
| `cave_generate` | grows a network of caverns over an area; every part can be reached |
| `cave_carve` | digs (or fills) a chamber, a tunnel or an exact area, with natural walls |
| `cave_entrance` | leaves the cave walls open where a path enters |
| `room_add` | adds a walled room (rectangle or polygon, with pillars); rooms merge like in Dungeondraft |
| `wall_add` | adds a free-standing wall, such as a partition |
| `door_add` | puts a door on a room (`R0`) or a wall (`W0`), at the wall point nearest to a position |
| `prop_place` | places an object, turned by any angle |
| `prop_remove` | removes a prop, for instance to place it again turned another way |
| `prop_scatter` | strews small props at random over an area, inside rooms, clear of walls, doorways and other props |
| `light_add` | adds a light, with range, intensity and color |
| `floor_paint` | repaints the floor of an area, such as one room, with another tileset |
| `path_add` | draws a path along points: conveyor belts, cables, pipes, trails |
| `pattern_add` | paints a floor pattern over grid shapes (grates, hazard stripes...) or a true circle, optionally outlined |
| `map_ambient` | sets the ambient light; dim it so that lights stand out |
| `map_background` | covers the whole ground with a terrain texture, such as `terrain_void` for space |
| `map_describe` | describes the map in text, with the ids shown on previews |
| `map_preview` | draws a schematic PNG of the map |
| `map_validate` | lists likely mistakes: rooms without doors, props across walls... |
| `map_save` | writes the `.dungeondraft_map` file |

The `catalog://summary` resource counts assets by pack and kind. Two prompts walk the model
through designing a location, from the plan to the saved file:

- `design_location` works in any setting, given as an argument (medieval fantasy,
  cyberpunk...);
- `design_40k_location` adds the rules of the Warhammer 40,000 universe, for Imperium
  Maledictum: Grimdorx props first, nothing modern or post-apocalyptic.

## Command line

```text
dd-mapper generate <plan.toml> --out <map.dungeondraft_map> [--preview <map.png>]
dd-mapper validate <map.dungeondraft_map>
dd-mapper preview <map.dungeondraft_map> --out <map.png> [--scale <pixels per cell>]
dd-mapper catalog [words...] [--kind <kind>] [--pack <id>] [--tag <tag>]... [--limit <n>]
dd-mapper help
```

Each command also takes the asset source options above. `catalog` without criteria counts
assets by pack and kind. `validate` exits with status 1 when it finds likely mistakes.
Errors exit with status 2.

For example, to search the packs for asset paths and then build the example plan:

```sh
dd-mapper catalog crate metal --kind object --packs assets/packs
dd-mapper generate crates/dd-cli/tests/data/hab_block.toml --out hab.dungeondraft_map --preview hab.png --packs assets/packs
```

### Plans

A plan is a TOML file. It starts from either a blank `[map]` or a `[generate]`d building,
then adds elements in this order: caves (networks, shapes, entrances), rooms, walls, doors, repainted floors, patterns, paths, props, scattered clutter
and lights. Ids
follow that order (`R0`, `W0`, `D0`...). A generated building already holds `R0` and
walls `W0`, `W1`... An element that cannot be added stops the build with its table name
and position, such as `[[door]] 2: ...`.

```toml
[style]                      # assets used where an element names none
floor = "res://packs/LOREGD01/textures/tilesets/simple/Tileset_RT_Punched_Floor.webp"
wall = "res://packs/LOREGD01/textures/walls/Wall_Underhive_Industrial.webp"
door = "res://packs/LOREGD01/textures/portals/Door_Underhive_Industrial.webp"
background = "res://textures/terrain/terrain_void.png"   # the ground around rooms
ambient = "404858"           # light everywhere; dim it so that lights stand out

[map]                        # a blank map...
width = 30
height = 20

# [generate]                 # ...or a generated building
# generator = "hab_block"    # bsp, hab_block or ship_deck
# width = 30
# height = 14
# seed = 7                   # the same seed draws the same building
# margin = 1                 # free cells around the building, such as room for engines
# hall = false               # bsp: a large hall, half the building, between two wings
# min_room = 3               # narrowest room, in cells

[[cave_network]]             # caverns grown over a rectangle of cells
x = 1
y = 1
width = 20
height = 18
density = 0.47               # share of starting rock: lower is more open
seed = 3

[[cave]]                     # dig (or fill = true) a chamber, tunnel, rectangle or polygon
shape = { type = "chamber", x = 24, y = 10, radius = 3 }
# shape = { type = "tunnel", points = [[17, 10], [24, 10]], width = 2 }
roughness = 0.4              # 0 smooth to 1 jagged walls
seed = 1

[[cave_entrance]]            # walls left open where a path comes in
shape = { type = "chamber", x = 27, y = 10, radius = 1 }

[[terrain]]                  # ground painted over the background, fading out
asset = "res://packs/LORENT01/textures/terrain/TIleset_Rock_Bed.webp"
shape = { type = "chamber", x = 20, y = 14, radius = 8 }
falloff = 2                  # cells of fade at the edge
roughness = 0.4

[[room]]
shape = { type = "rectangle", x = 1, y = 1, width = 12, height = 8 }
pillars = [{ type = "rectangle", x = 6, y = 4, width = 1, height = 1 }]
# floor = "..."              # overrides the style

[[wall]]
points = [[6, 1], [6, 9]]

[[door]]
on = "W0"                    # a room (R0) or a wall (W0)
at = [6, 2.5]                # snapped to the nearest point of its walls

[[floor]]                    # another tileset for part of the floor
shape = { type = "rectangle", x = 7, y = 1, width = 6, height = 8 }
tileset = "res://packs/LORESP01/textures/tilesets/simple/Floor_SpaceShip_Grate_Worn.webp"

[[pattern]]
asset = "res://packs/LOREGD01/textures/patterns/normal/Artifcial/Tileset_AncientGothic_Quarters_Sewer_Open.webp"
shape = { type = "polygon", points = [[8, 2], [10, 2], [10, 4]] }
# or shape = { type = "circle", x = 20, y = 14, radius = 6 }
# outline = true             # a dark edge, like a cut through the ground
# color = "7f80beff"

[[prop]]
asset = "res://packs/LOREGD01/textures/objects/[LORE] Containers/Container_Large_Metal_Crate_Green.webp"
at = [3, 3]                  # center, in cells
rotation = 90                # clockwise degrees, any angle

[[path]]                     # conveyor belts, cables, pipes, trails
asset = "res://packs/LOREGD01/textures/paths/Artificial/Path_Conveyor_Belt_Large.webp"
points = [[2.5, 7], [12, 7], [12, 3.5]]   # cells, anywhere, even off the map's edges
width = 1.5                  # cells, 1 by default
closed = false
sharp = false                # true: straight segments, for stripes, fences, pipes

[[scatter]]                  # clutter strewn at random inside rooms
assets = ["res://packs/LOREAP01/textures/objects/[LORE] Clutter/Debris_Can_Empty.webp"]
shape = { type = "rectangle", x = 1, y = 1, width = 12, height = 8 }
count = 15
seed = 3

[[light]]
asset = "res://packs/LOREGD01/textures/lights/Effect_Caged_Light_Cast.webp"
at = [4, 4]
range = 5                    # cells
intensity = 1.0
color = "80beff"             # RRGGBB or AARRGGBB
```

### Generators

All three leave a one-cell margin around the building and give every room a door, so that
every room can be reached:

- `bsp` splits the building at random into rooms at least `min_room` cells wide (a
  manufactorum, a bunker, administratum offices) and adds one entrance on the south side.
  With `hall`, half the building stays one large hall between two wings of rooms, and the
  entrance opens into the hall.
- `hab_block` lines both sides of a two-cell corridor with cells. Each cell opens on the
  corridor, and the corridor has an entrance at each end.
- `ship_deck` is a hab block ending in a pointed bow, entered from the stern.

## Architecture

A Cargo workspace. Each crate depends only on the crates listed after it, so there are no
cycles:

| Crate | Role | Depends on |
| --- | --- | --- |
| `dd-domain` | grid, polygons, rooms and their merges, partitions, doors, props, lights, patterns | — |
| `dd-format` | Godot 3 variant and JSON codecs; `.dungeondraft_map` model, export and import | `dd-domain` |
| `dd-catalog` | assets, kinds, metrics and search; the `AssetSource` port | `dd-domain` |
| `dd-catalog-io` | asset sources: pack index, saved maps, `catalog.toml` | `dd-catalog`, `dd-format` |
| `dd-gen` | seeded layout generators | `dd-domain` |
| `dd-render` | schematic PNG previews | `dd-domain` |
| `dd-app` | use cases over a workspace of maps, behind the `MapFiles` and `PreviewRenderer` ports | `dd-catalog`, `dd-gen` |
| `dd-setup` | composition: source options, file and preview adapters, default template | `dd-app`, `dd-catalog-io`, `dd-format`, `dd-render` |
| `dd-mcp` | MCP server (`dd-mcp` binary) | `dd-app`, `dd-setup` |
| `dd-cli` | command line (`dd-mapper` binary) | `dd-app`, `dd-setup` |

How a map is written:

- New maps are written over a template saved by Dungeondraft.
- Opened maps are written over their own file, so that everything the domain does not
  model survives a round trip.
- Unknown JSON fields keep their place.
- Numbers are formatted as Godot formats them, so that unchanged files are rewritten byte
  for byte.

## Development

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

`fixtures/` holds maps saved by Dungeondraft that the format tests read and rewrite. Two
tests are ignored by default:

- `real_packs` needs the packs in `assets/packs`. Run it with
  `cargo test -p dd-catalog-io -- --ignored`.
- `print_warnings` prints what importing the fixtures leaves out.

## License

dd-mapper is released under the [MIT license](LICENSE). It covers the tool only: asset
packs keep their own terms, and the maps you draw with them follow those terms. Dungeondraft
is a product of its own authors; dd-mapper is not affiliated with it.
