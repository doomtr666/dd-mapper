//! The MCP server: tools, resources and prompts over a [`Workspace`].

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use base64::Engine as _;
use dd_app::{
    AppError, LayoutReport, LayoutRequest, OpenReport, RoomReport, ScatterReport, Workspace,
};
use dd_catalog::AssetQuery;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ContentBlock, GetPromptRequestParams, GetPromptResponse, GetPromptResult,
    Implementation, ListPromptsResult, ListResourcesResult, PaginatedRequestParams, Prompt,
    PromptArgument, PromptMessage, ReadResourceRequestParams, ReadResourceResponse,
    ReadResourceResult, Resource, ResourceContents, Role, ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, tool, tool_handler, tool_router};

use crate::guidance::{
    GENERAL_PROMPT_DESCRIPTION, GENERAL_PROMPT_NAME, INSTRUCTIONS, WARHAMMER_PROMPT_DESCRIPTION,
    WARHAMMER_PROMPT_NAME, general_prompt, warhammer_prompt,
};
use crate::params::{
    CatalogSearchParams, CaveCarveParams, CaveEntranceParams, CaveGenerateParams,
    DEFAULT_PIXELS_PER_CELL, DoorAddParams, FloorPaintParams, LayoutGenerateParams, LightAddParams,
    MapAmbientParams, MapBackgroundParams, MapIdParams, MapNewParams, MapOpenParams,
    MapPreviewParams, MapSaveParams, PathAddParams, PatternAddParams, PropPlaceParams,
    PropRemoveParams, PropScatterParams, RoomAddParams, WallAddParams,
};

const SERVER_NAME: &str = "dd-mapper";
const CATALOG_SUMMARY_URI: &str = "catalog://summary";
const BRIEF_ARGUMENT: &str = "brief";
const DEFAULT_BRIEF: &str = "a location of your choice";
const SETTING_ARGUMENT: &str = "setting";
const DEFAULT_SETTING: &str = "fantasy";

/// Serves the design tools of one session; clones share the same maps.
#[derive(Clone)]
pub struct DdMapperServer {
    workspace: Arc<Mutex<Workspace>>,
    tool_router: ToolRouter<Self>,
}

impl DdMapperServer {
    #[must_use]
    pub fn new(workspace: Workspace) -> Self {
        Self {
            workspace: Arc::new(Mutex::new(workspace)),
            tool_router: Self::tool_router(),
        }
    }

    fn with_workspace<T>(&self, action: impl FnOnce(&mut Workspace) -> T) -> T {
        let mut workspace = self
            .workspace
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        action(&mut workspace)
    }
}

#[tool_router]
impl DdMapperServer {
    #[tool(
        description = "Create an empty map of width x height cells (1 cell = 5 ft). \
Returns its map_id."
    )]
    fn map_new(&self, Parameters(params): Parameters<MapNewParams>) -> CallToolResult {
        reply(self.with_workspace(|workspace| workspace.new_map(params.width, params.height)))
    }

    #[tool(
        description = "Open a .dungeondraft_map file saved by Dungeondraft as a new map: \
rooms, pillars, free walls, doors, floor tiles, props, lights and patterns. Saving it keeps \
everything else the file holds. Returns its map_id and what could not be imported."
    )]
    fn map_open(&self, Parameters(params): Parameters<MapOpenParams>) -> CallToolResult {
        let source = Path::new(&params.path);
        let opened = self.with_workspace(|workspace| workspace.open_map(source));
        reply(opened.map(|report| open_report(&report)))
    }

    #[tool(
        description = "Draw a whole building on a new map in one go: its outer walls, \
partitions splitting it into rooms, and doors so that every room can be reached, plus \
entrances. Generators: bsp (rooms split at random), hab_block (cells along a corridor), \
ship_deck (a hab block with a bow). Returns the map_id; then furnish it."
    )]
    fn layout_generate(
        &self,
        Parameters(params): Parameters<LayoutGenerateParams>,
    ) -> CallToolResult {
        let request = LayoutRequest::from(params);
        let report = self.with_workspace(|workspace| workspace.generate(&request));
        reply(report.map(|report| layout_report(&report)))
    }

    #[tool(
        description = "Grow a network of caverns over a rectangle of cells, with natural \
rock walls; every part can be reached. Combine it with cave_carve to add chambers and \
tunnels, and cave_entrance where paths come in."
    )]
    fn cave_generate(&self, Parameters(params): Parameters<CaveGenerateParams>) -> CallToolResult {
        let map = params.map_id.clone();
        let network = dd_app::CaveNetwork::from(params);
        let dug = self.with_workspace(|workspace| workspace.grow_caves(&map, &network));
        reply(dug.map(|samples| format!("Dug about {} cells of cave.", cells(samples))))
    }

    #[tool(
        description = "Dig a cave shape, or fill it back with rock: a chamber (center and \
radius), a tunnel (points and width), or an exact rectangle or polygon for mines. Chambers \
and tunnels get natural walls; roughness sets how jagged."
    )]
    fn cave_carve(&self, Parameters(params): Parameters<CaveCarveParams>) -> CallToolResult {
        let (map, request) = params.into_request();
        let verb = if request.fill { "Filled" } else { "Dug" };
        let changed = self.with_workspace(|workspace| workspace.carve_cave(&map, &request));
        reply(changed.map(|samples| format!("{verb} about {} cells.", cells(samples))))
    }

    #[tool(
        description = "Leave the cave walls open over a shape, where a path, a road or a \
door enters the cave; put it on the cave's edge."
    )]
    fn cave_entrance(&self, Parameters(params): Parameters<CaveEntranceParams>) -> CallToolResult {
        let shape = params.shape.into();
        let opened =
            self.with_workspace(|workspace| workspace.open_cave_entrance(&params.map_id, &shape));
        reply(
            opened.map(|samples| format!("Opened the walls over about {} cells.", cells(samples))),
        )
    }

    #[tool(
        description = "Add a walled room: a rectangle or polygon on grid vertices, with \
optional pillars, a floor tileset and a wall texture (asset paths from catalog_search). \
Rooms sharing area or a wall length merge, like with Dungeondraft's room tool. \
Returns the room id and what the merge absorbed."
    )]
    fn room_add(&self, Parameters(params): Parameters<RoomAddParams>) -> CallToolResult {
        let (map, request) = params.into_request();
        let report = self.with_workspace(|workspace| workspace.add_room(&map, &request));
        reply(report.map(|report| room_report(&report)))
    }

    #[tool(
        description = "Add a free-standing wall through points on grid vertices, such as a \
partition splitting a room, so that doors can join the parts. It does not close and never \
merges with rooms. Returns the wall id (W0...)."
    )]
    fn wall_add(&self, Parameters(params): Parameters<WallAddParams>) -> CallToolResult {
        let (map, request) = params.into_request();
        reply(self.with_workspace(|workspace| workspace.add_wall(&map, &request)))
    }

    #[tool(
        description = "Add a door or window (kind portal) on the walls of a room (R0) or of \
a free-standing wall (W0), at the wall point nearest to (x, y). Returns the door id."
    )]
    fn door_add(&self, Parameters(params): Parameters<DoorAddParams>) -> CallToolResult {
        let (map, request) = params.into_request();
        reply(self.with_workspace(|workspace| workspace.add_door(&map, &request)))
    }

    #[tool(
        description = "Place an object centered on (x, y), turned by 0, 90, 180 or 270 \
degrees. Its footprint is the asset's real size. Returns the prop id."
    )]
    fn prop_place(&self, Parameters(params): Parameters<PropPlaceParams>) -> CallToolResult {
        let (map, request) = params.into_request();
        reply(self.with_workspace(|workspace| workspace.place_prop(&map, &request)))
    }

    #[tool(
        description = "Draw a path (kind path) along a line of points in cells: conveyor \
belts, cables, pipes, rails, worn trails. Points may lie anywhere, even off the map's edges, \
so that a road or a belt can leave the map. Returns the path id (T0...)."
    )]
    fn path_add(&self, Parameters(params): Parameters<PathAddParams>) -> CallToolResult {
        let (map, request) = params.into_request();
        reply(self.with_workspace(|workspace| workspace.add_path(&map, &request)))
    }

    #[tool(
        description = "Repaint the floor of an area with another tileset (kind tileset), \
such as one room of a building: a grating for the engine room, plates for the bridge, worn \
decking for cabins. Walls stay as they are."
    )]
    fn floor_paint(&self, Parameters(params): Parameters<FloorPaintParams>) -> CallToolResult {
        let (map, request) = params.into_request();
        let painted = self.with_workspace(|workspace| workspace.paint_floor(&map, &request));
        reply(painted.map(|cells| format!("Painted {cells} cells.")))
    }

    #[tool(description = "Remove a prop, for instance to place it again turned another way.")]
    fn prop_remove(&self, Parameters(params): Parameters<PropRemoveParams>) -> CallToolResult {
        let removed =
            self.with_workspace(|workspace| workspace.remove_prop(&params.map_id, &params.prop_id));
        reply(removed.map(|()| format!("Removed {}.", params.prop_id)))
    }

    #[tool(
        description = "Strew small props at random over an area: debris, cans, papers, \
cables, bones... Each is picked among the given assets and turned at random, inside a room, \
clear of walls, doorways and other props. With along_walls, props line the walls instead, \
backs to the wall and facing the room: lockers, shelves, barrels, pipes. Pick props of at least \
half a cell: smaller ones barely show. Returns the prop ids."
    )]
    fn prop_scatter(&self, Parameters(params): Parameters<PropScatterParams>) -> CallToolResult {
        let (map, request) = params.into_request();
        let report = self.with_workspace(|workspace| workspace.scatter(&map, &request));
        reply(report.map(|report| scatter_report(&report)))
    }

    #[tool(
        description = "Set the ambient light of a map, the light everywhere before lights \
add theirs. It is full white by default, which flattens everything: dim it (404858 for an \
underhive, 202838 for a derelict) and place lights where the scene needs them."
    )]
    fn map_ambient(&self, Parameters(params): Parameters<MapAmbientParams>) -> CallToolResult {
        let set = self
            .with_workspace(|workspace| workspace.set_ambient_light(&params.map_id, &params.color));
        reply(set.map(|argb| format!("{} now has an ambient light of {argb:08x}.", params.map_id)))
    }

    #[tool(
        description = "Cover the whole ground of a map, around and under the rooms, with a \
terrain texture (kind terrain): the void of space, rock, sand..."
    )]
    fn map_background(
        &self,
        Parameters(params): Parameters<MapBackgroundParams>,
    ) -> CallToolResult {
        let set = self
            .with_workspace(|workspace| workspace.set_background(&params.map_id, &params.asset));
        let name = params.asset.rsplit('/').next().unwrap_or_default();
        let name = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        reply(set.map(|()| format!("{} now lies on {name}.", params.map_id)))
    }

    #[tool(
        description = "Add a light (kind light) centered on (x, y), with an optional range \
in cells, intensity and RRGGBB color. Returns the light id (L0...)."
    )]
    fn light_add(&self, Parameters(params): Parameters<LightAddParams>) -> CallToolResult {
        let (map, request) = params.into_request();
        reply(self.with_workspace(|workspace| workspace.add_light(&map, &request)))
    }

    #[tool(
        description = "Paint a pattern texture (kind pattern) over a rectangle or polygon on grid vertices: grates, fans, hazard stripes, markings. Returns the pattern id (S0...)."
    )]
    fn pattern_add(&self, Parameters(params): Parameters<PatternAddParams>) -> CallToolResult {
        let (map, request) = params.into_request();
        reply(self.with_workspace(|workspace| workspace.add_pattern(&map, &request)))
    }

    #[tool(
        description = "Describe a map in text: rooms, doors and props with their ids, \
positions and assets, as labelled on map_preview."
    )]
    fn map_describe(&self, Parameters(params): Parameters<MapIdParams>) -> CallToolResult {
        reply(self.with_workspace(|workspace| workspace.describe(&params.map_id)))
    }

    #[tool(
        description = "Check a map for likely design mistakes: rooms without doors, props \
outside rooms, across walls or overlapping."
    )]
    fn map_validate(&self, Parameters(params): Parameters<MapIdParams>) -> CallToolResult {
        let warnings = self.with_workspace(|workspace| workspace.validate(&params.map_id));
        reply(warnings.map(|warnings| warnings_text(&warnings)))
    }

    #[tool(
        description = "Draw a schematic PNG of a map: floors colored by tileset, walls in \
black, doors in orange, props as blue boxes, with R/D/P ids. Look at it after each step."
    )]
    fn map_preview(&self, Parameters(params): Parameters<MapPreviewParams>) -> CallToolResult {
        let pixels_per_cell = params.pixels_per_cell.unwrap_or(DEFAULT_PIXELS_PER_CELL);
        let png =
            self.with_workspace(|workspace| workspace.preview_png(&params.map_id, pixels_per_cell));
        match png {
            Ok(png) => {
                let data = base64::engine::general_purpose::STANDARD.encode(png);
                CallToolResult::success(vec![ContentBlock::image(data, "image/png")])
            }
            Err(error) => failure(&error),
        }
    }

    #[tool(description = "Write a map to a .dungeondraft_map file, to open in Dungeondraft.")]
    fn map_save(&self, Parameters(params): Parameters<MapSaveParams>) -> CallToolResult {
        let destination = Path::new(&params.path);
        let saved = self.with_workspace(|workspace| workspace.save(&params.map_id, destination));
        reply(saved.map(|()| format!("Saved {} to {}.", params.map_id, params.path)))
    }

    #[tool(
        description = "Search assets by words, kind (tileset, wall, portal, object, light, \
pattern, terrain, path), tags or pack. Returns asset paths with their size in cells."
    )]
    fn catalog_search(
        &self,
        Parameters(params): Parameters<CatalogSearchParams>,
    ) -> CallToolResult {
        let query = AssetQuery::from(params);
        let lines = self.with_workspace(|workspace| {
            workspace
                .search_catalog(&query)
                .into_iter()
                .map(dd_app::asset_line)
                .collect::<Vec<_>>()
        });
        if lines.is_empty() {
            return success("No asset matches; try fewer or other words, or another kind.");
        }
        success(&lines.join("\n"))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DdMapperServer {
    fn get_info(&self) -> ServerConfig {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_prompts()
            .build();
        let identity = Implementation::new(SERVER_NAME, env!("CARGO_PKG_VERSION"))
            .with_description("Designs Dungeondraft maps on a grid, with previews.");
        ServerConfig::new(capabilities)
            .with_server_info(identity)
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let summary = Resource::new(CATALOG_SUMMARY_URI, "catalog_summary")
            .with_description("How many assets each pack provides, by kind.")
            .with_mime_type("text/plain");
        Ok(ListResourcesResult {
            resources: vec![summary],
            ..ListResourcesResult::default()
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        if request.uri != CATALOG_SUMMARY_URI {
            let message = format!("no resource {}; try {CATALOG_SUMMARY_URI}", request.uri);
            return Err(ErrorData::resource_not_found(message, None));
        }
        let summary = self.with_workspace(|workspace| workspace.catalog_summary());
        let contents = ResourceContents::text(summary, CATALOG_SUMMARY_URI);
        Ok(ReadResourceResult::new(vec![contents]).into())
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        let brief = || {
            PromptArgument::new(BRIEF_ARGUMENT)
                .with_description("The location to design, in a sentence.")
                .with_required(false)
        };
        let setting = PromptArgument::new(SETTING_ARGUMENT)
            .with_description(
                "The world it belongs to: medieval fantasy, cyberpunk, gothic horror...",
            )
            .with_required(false);
        let general = Prompt::new(
            GENERAL_PROMPT_NAME,
            Some(GENERAL_PROMPT_DESCRIPTION),
            Some(vec![brief(), setting]),
        );
        let warhammer = Prompt::new(
            WARHAMMER_PROMPT_NAME,
            Some(WARHAMMER_PROMPT_DESCRIPTION),
            Some(vec![brief()]),
        );
        Ok(ListPromptsResult {
            prompts: vec![general, warhammer],
            ..ListPromptsResult::default()
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResponse, ErrorData> {
        let argument = |name: &str, default: &str| {
            request
                .arguments
                .as_ref()
                .and_then(|arguments| arguments.get(name))
                .and_then(|value| value.as_str())
                .unwrap_or(default)
                .to_owned()
        };
        let brief = argument(BRIEF_ARGUMENT, DEFAULT_BRIEF);
        let (text, description) = match request.name.as_str() {
            GENERAL_PROMPT_NAME => (
                general_prompt(&brief, &argument(SETTING_ARGUMENT, DEFAULT_SETTING)),
                GENERAL_PROMPT_DESCRIPTION,
            ),
            WARHAMMER_PROMPT_NAME => (warhammer_prompt(&brief), WARHAMMER_PROMPT_DESCRIPTION),
            other => {
                let message = format!(
                    "no prompt {other}; try {GENERAL_PROMPT_NAME} or {WARHAMMER_PROMPT_NAME}"
                );
                return Err(ErrorData::invalid_params(message, None));
            }
        };
        let message = PromptMessage::new_text(Role::User, text);
        Ok(GetPromptResult::new(vec![message])
            .with_description(description)
            .into())
    }
}

fn reply(result: Result<String, AppError>) -> CallToolResult {
    match result {
        Ok(text) => success(&text),
        Err(error) => failure(&error),
    }
}

fn success(text: &str) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(text)])
}

fn failure(error: &AppError) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(error.to_string())])
}

fn room_report(report: &RoomReport) -> String {
    let mut parts = vec![report.id.clone()];
    if !report.absorbed.is_empty() {
        parts.push(format!("merged rooms {}", report.absorbed.join(", ")));
    }
    if !report.removed_doors.is_empty() {
        parts.push(format!(
            "removed doors {} whose wall disappeared",
            report.removed_doors.join(", ")
        ));
    }
    parts.join("; ")
}

/// Quarter-cell samples as whole cells, sixteen to a cell.
fn cells(samples: usize) -> f64 {
    let samples = u32::try_from(samples).unwrap_or(u32::MAX);
    (f64::from(samples) / 16.0).round()
}

fn scatter_report(report: &ScatterReport) -> String {
    let placed = match report.props.as_slice() {
        [] => "Placed no prop".to_owned(),
        [only] => format!("Placed 1 prop: {only}"),
        [first, .., last] => format!("Placed {} props: {first} to {last}", report.props.len()),
    };
    let text = if report.missing == 0 {
        format!("{placed}.")
    } else {
        format!(
            "{placed}; {} found no free spot, so the area is full: use a larger area or fewer props.",
            report.missing
        )
    };
    if report.tiny.is_empty() {
        return text;
    }
    let verb = if report.tiny.len() == 1 { "is" } else { "are" };
    format!(
        "{text} {} {verb} smaller than a third of a cell and will barely show: prefer larger clutter.",
        report.tiny.join(", ")
    )
}

fn layout_report(report: &LayoutReport) -> String {
    format!(
        "{}: room {} split into {} areas by {} walls, with {} doors. \
Call map_describe and map_preview to see where they are.",
        report.map, report.room, report.areas, report.walls, report.doors
    )
}

fn open_report(report: &OpenReport) -> String {
    if report.warnings.is_empty() {
        return report.id.clone();
    }
    format!(
        "{}\nNot imported:\n- {}",
        report.id,
        report.warnings.join("\n- ")
    )
}

fn warnings_text(warnings: &[String]) -> String {
    if warnings.is_empty() {
        return "No warning.".to_owned();
    }
    warnings.join("\n")
}
