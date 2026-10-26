//! Integration tests: a real MCP client drives the server in process.

use base64::Engine as _;
use dd_app::Workspace;
use dd_catalog::{Asset, AssetMetrics, Catalog, CellSize};
use dd_domain::AssetPath;
use dd_format::model::MapFile;
use dd_mcp::server::DdMapperServer;
use dd_setup::adapters::{DungeondraftFiles, SchematicRenderer};
use rmcp::model::{CallToolRequestParams, ClientConfig};
use rmcp::service::RunningService;
use rmcp::{ClientHandler, RoleClient, ServiceExt};
use serde_json::{Value, json};

const FLOOR: &str = "res://packs/LOREGD01/textures/tilesets/simple/Tileset_RT_Punched_Floor.webp";
const WALL: &str = "res://packs/LOREGD01/textures/walls/Wall_Underhive_Industrial.webp";
const DOOR: &str = "res://packs/LOREGD01/textures/portals/Door_Underhive_Industrial.webp";
const LIGHT: &str = "res://packs/LOREGD01/textures/lights/Effect_Caged_Light_Cast.webp";
const GRATE: &str = "res://packs/LOREGD01/textures/patterns/normal/Artifcial/Tileset_AncientGothic_Quarters_Sewer_Open.webp";
const VOID: &str = "res://textures/terrain/terrain_void.png";
const BELT: &str = "res://packs/LOREGD01/textures/paths/Artificial/Path_Conveyor_Belt_Large.webp";
const CAN: &str = "res://packs/LOREGD01/textures/objects/[LORE] Clutter/Debris_Can_Empty.webp";
const CRATE: &str = "res://packs/LOREGD01/textures/objects/[LORE] Containers/Container_Large_Metal_Crate_Green.webp";

#[derive(Debug, Clone, Default)]
struct TestClient;

impl ClientHandler for TestClient {
    fn get_info(&self) -> ClientConfig {
        ClientConfig::default()
    }
}

type Client = RunningService<RoleClient, TestClient>;

fn catalog() -> Catalog {
    let mut catalog = Catalog::default();
    for (path, metrics) in [
        (FLOOR, AssetMetrics::default()),
        (WALL, AssetMetrics::default()),
        (LIGHT, AssetMetrics::default()),
        (GRATE, AssetMetrics::default()),
        (VOID, AssetMetrics::default()),
        (BELT, AssetMetrics::default()),
        (
            CAN,
            AssetMetrics {
                size: Some(CellSize {
                    width: 0.2,
                    height: 0.3,
                }),
                ..AssetMetrics::default()
            },
        ),
        (
            DOOR,
            AssetMetrics {
                portal_radius: Some(154.0),
                ..AssetMetrics::default()
            },
        ),
        (
            CRATE,
            AssetMetrics {
                size: Some(CellSize {
                    width: 1.8,
                    height: 1.5,
                }),
                ..AssetMetrics::default()
            },
        ),
    ] {
        let mut asset = Asset::from_path(AssetPath::new(path).unwrap()).unwrap();
        asset.metrics = metrics;
        catalog.add(asset);
    }
    catalog
}

async fn connect() -> Client {
    let workspace = Workspace::new(
        catalog(),
        Box::new(DungeondraftFiles::with_default_template().unwrap()),
        Box::new(SchematicRenderer),
    );
    let (server_io, client_io) = tokio::io::duplex(1 << 20);
    tokio::spawn(async move {
        let server = DdMapperServer::new(workspace).serve(server_io).await?;
        server.waiting().await?;
        anyhow::Ok(())
    });
    TestClient.serve(client_io).await.unwrap()
}

/// Calls a tool; returns its JSON result.
async fn call(client: &Client, tool: &'static str, arguments: Value) -> Value {
    let Value::Object(arguments) = arguments else {
        panic!("tool arguments must be an object");
    };
    let result = client
        .call_tool(CallToolRequestParams::new(tool).with_arguments(arguments))
        .await
        .unwrap_or_else(|error| panic!("{tool}: {error}"));
    serde_json::to_value(result).unwrap()
}

fn text(result: &Value) -> String {
    result["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|block| block["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_error(result: &Value) -> bool {
    result["isError"].as_bool().unwrap_or(false)
}

fn ok_text(result: &Value) -> String {
    assert!(!is_error(result), "unexpected error: {}", text(result));
    text(result)
}

#[tokio::test]
async fn introduces_itself_as_dd_mapper() {
    let client = connect().await;
    let info = serde_json::to_value(client.peer_info().unwrap()).unwrap();
    assert_eq!(info["serverInfo"]["name"], "dd-mapper");
    assert_eq!(info["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
    assert!(
        info["instructions"]
            .as_str()
            .unwrap()
            .contains("map_preview")
    );
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn lists_the_design_tools() {
    let client = connect().await;
    let tools = client.list_all_tools().await.unwrap();
    let mut names: Vec<String> = tools.iter().map(|tool| tool.name.to_string()).collect();
    names.sort();
    assert_eq!(
        names,
        [
            "catalog_search",
            "cave_carve",
            "cave_entrance",
            "cave_generate",
            "door_add",
            "floor_paint",
            "layout_generate",
            "light_add",
            "map_ambient",
            "map_background",
            "map_describe",
            "map_new",
            "map_open",
            "map_preview",
            "map_save",
            "map_validate",
            "path_add",
            "pattern_add",
            "prop_place",
            "prop_remove",
            "prop_scatter",
            "room_add",
            "wall_add",
        ]
    );
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn designs_previews_and_saves_a_map() {
    let client = connect().await;
    let map = ok_text(&call(&client, "map_new", json!({"width": 20, "height": 12})).await);
    assert_eq!(map, "map-1");

    let room = json!({
        "map_id": map,
        "shape": {"type": "rectangle", "x": 1, "y": 1, "width": 8, "height": 6},
        "pillars": [{"type": "rectangle", "x": 4, "y": 3, "width": 1, "height": 1}],
        "floor": FLOOR,
        "wall": WALL,
    });
    assert!(ok_text(&call(&client, "room_add", room).await).contains("R0"));
    let door = json!({"map_id": map, "on": "R0", "x": 5, "y": 7.2, "asset": DOOR});
    assert_eq!(ok_text(&call(&client, "door_add", door).await), "D0");
    let partition = json!({"map_id": map, "points": [[7, 1], [7, 7]], "wall": WALL});
    assert_eq!(ok_text(&call(&client, "wall_add", partition).await), "W0");
    let inner_door = json!({"map_id": map, "on": "W0", "x": 7, "y": 5.5, "asset": DOOR});
    assert_eq!(ok_text(&call(&client, "door_add", inner_door).await), "D1");
    let prop = json!({"map_id": map, "asset": CRATE, "x": 2.5, "y": 2.5, "rotation": 90});
    assert_eq!(ok_text(&call(&client, "prop_place", prop).await), "P0");
    let light =
        json!({"map_id": map, "asset": LIGHT, "x": 3, "y": 4, "range": 3.5, "color": "ffad58"});
    assert_eq!(ok_text(&call(&client, "light_add", light).await), "L0");
    let grate = json!({
        "map_id": map,
        "shape": {"type": "rectangle", "x": 6, "y": 5, "width": 1, "height": 1},
        "asset": GRATE,
    });
    assert_eq!(ok_text(&call(&client, "pattern_add", grate).await), "S0");

    let description = ok_text(&call(&client, "map_describe", json!({"map_id": map})).await);
    for id in ["R0:", "D0:", "P0:", "W0:", "D1: on W0", "L0:", "S0:"] {
        assert!(description.contains(id), "{description}");
    }
    let warnings = ok_text(&call(&client, "map_validate", json!({"map_id": map})).await);
    assert_eq!(warnings, "No warning.");

    let preview = call(
        &client,
        "map_preview",
        json!({"map_id": map, "pixels_per_cell": 16}),
    )
    .await;
    let image = &preview["content"][0];
    assert_eq!(image["type"], "image");
    assert_eq!(image["mimeType"], "image/png");
    let png = base64::engine::general_purpose::STANDARD
        .decode(image["data"].as_str().unwrap())
        .unwrap();
    assert!(png.starts_with(b"\x89PNG"));

    let destination = std::env::temp_dir().join(format!(
        "dd-mapper-session-{}.dungeondraft_map",
        std::process::id()
    ));
    let save = json!({"map_id": map, "path": destination.display().to_string()});
    ok_text(&call(&client, "map_save", save).await);
    let saved = MapFile::parse(&std::fs::read_to_string(&destination).unwrap()).unwrap();
    std::fs::remove_file(&destination).unwrap();
    let level = &saved.world().levels["0"];
    assert_eq!((saved.world().width, saved.world().height), (20, 12));
    assert_eq!(level.walls.len(), 3);
    assert_eq!(level.walls[0].portals.len(), 1);
    let partition = &level.walls[2];
    assert_eq!((partition.wall_type, partition.closed_loop), (1, false));
    assert_eq!(partition.portals.len(), 1);
    assert!((level.walls[0].portals[0].radius - 154.0).abs() < f64::EPSILON);
    assert_eq!(level.objects.len(), 1);
    assert_eq!(level.lights.len(), 1);
    assert_eq!(level.patterns.len(), 1);
    assert!((level.lights[0].range - 3.5).abs() < f64::EPSILON);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn generates_a_building_ready_to_furnish() {
    let client = connect().await;
    let generate = |generator: &str, height: u32| {
        json!({
            "generator": generator,
            "width": 30,
            "height": height,
            "seed": 7,
            "floor": FLOOR,
            "wall": WALL,
            "door": DOOR,
        })
    };
    let report = ok_text(&call(&client, "layout_generate", generate("hab_block", 14)).await);
    assert!(report.starts_with("map-1: room R0 split into "), "{report}");
    let warnings = ok_text(&call(&client, "map_validate", json!({"map_id": "map-1"})).await);
    assert_eq!(warnings, "No warning.");
    let description = ok_text(&call(&client, "map_describe", json!({"map_id": "map-1"})).await);
    assert!(description.contains("W0:"), "{description}");

    let too_small = call(&client, "layout_generate", generate("ship_deck", 4)).await;
    assert!(is_error(&too_small));
    assert!(
        text(&too_small).contains("at least 8 x 6 cells"),
        "{}",
        text(&too_small)
    );
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn dresses_a_building_with_clutter_and_a_background() {
    let client = connect().await;
    let building = json!({
        "generator": "hab_block", "width": 30, "height": 14, "seed": 7,
        "floor": FLOOR, "wall": WALL, "door": DOOR,
    });
    ok_text(&call(&client, "layout_generate", building).await);
    let background = json!({"map_id": "map-1", "asset": VOID});
    assert_eq!(
        ok_text(&call(&client, "map_background", background).await),
        "map-1 now lies on terrain_void."
    );
    let floor = json!({
        "map_id": "map-1",
        "shape": {"type": "rectangle", "x": 1, "y": 6, "width": 28, "height": 2},
        "tileset": FLOOR,
    });
    assert_eq!(
        ok_text(&call(&client, "floor_paint", floor).await),
        "Painted 56 cells."
    );
    let ambient = json!({"map_id": "map-1", "color": "404858"});
    assert_eq!(
        ok_text(&call(&client, "map_ambient", ambient).await),
        "map-1 now has an ambient light of ff404858."
    );
    let scatter = json!({
        "map_id": "map-1",
        "assets": [CAN],
        "shape": {"type": "rectangle", "x": 0, "y": 0, "width": 30, "height": 14},
        "count": 12,
        "seed": 3,
    });
    let strewn = ok_text(&call(&client, "prop_scatter", scatter).await);
    assert_eq!(
        strewn,
        "Placed 12 props: P0 to P11. Debris_Can_Empty is smaller than a third of a cell and will barely show: prefer larger clutter."
    );
    let lined = json!({
        "map_id": "map-1",
        "assets": [CRATE],
        "shape": {"type": "rectangle", "x": 0, "y": 0, "width": 30, "height": 14},
        "count": 4,
        "seed": 5,
        "along_walls": true,
    });
    assert_eq!(
        ok_text(&call(&client, "prop_scatter", lined).await),
        "Placed 4 props: P12 to P15."
    );
    let belt = json!({
        "map_id": "map-1",
        "asset": BELT,
        "points": [[2.5, 7], [27.5, 7]],
        "width": 1.5,
    });
    assert_eq!(ok_text(&call(&client, "path_add", belt).await), "T0");
    let removed = json!({"map_id": "map-1", "prop_id": "P3"});
    assert_eq!(
        ok_text(&call(&client, "prop_remove", removed).await),
        "Removed P3."
    );
    let warnings = ok_text(&call(&client, "map_validate", json!({"map_id": "map-1"})).await);
    assert_eq!(warnings, "No warning.");

    let destination = std::env::temp_dir().join(format!(
        "dd-mapper-dressed-{}.dungeondraft_map",
        std::process::id()
    ));
    let save = json!({"map_id": "map-1", "path": destination.display().to_string()});
    ok_text(&call(&client, "map_save", save).await);
    let saved = MapFile::parse(&std::fs::read_to_string(&destination).unwrap()).unwrap();
    std::fs::remove_file(&destination).unwrap();
    let level = &saved.world().levels["0"];
    assert_eq!(level.terrain.texture_1, VOID);
    let environment = level.environment.as_ref().unwrap();
    assert_eq!(environment.ambient_light.0, 0xff40_4858);
    assert_eq!(level.objects.len(), 15);
    assert_eq!(level.paths.len(), 1);
    assert!((level.paths[0].width - 384.0).abs() < f64::EPSILON);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn digs_caves_and_saves_them() {
    let client = connect().await;
    ok_text(&call(&client, "map_new", json!({"width": 30, "height": 20})).await);
    let grown = json!({"map_id": "map-1", "x": 1, "y": 1, "width": 20, "height": 18, "seed": 5});
    assert!(ok_text(&call(&client, "cave_generate", grown).await).starts_with("Dug about "));
    let chamber = json!({
        "map_id": "map-1",
        "shape": {"type": "chamber", "x": 25, "y": 10, "radius": 3},
        "roughness": 0.5,
        "seed": 2,
    });
    ok_text(&call(&client, "cave_carve", chamber).await);
    let tunnel = json!({
        "map_id": "map-1",
        "shape": {"type": "tunnel", "points": [[18, 10], [25, 10]], "width": 2},
    });
    ok_text(&call(&client, "cave_carve", tunnel).await);
    let opening = json!({
        "map_id": "map-1",
        "shape": {"type": "chamber", "x": 28, "y": 10, "radius": 1},
    });
    ok_text(&call(&client, "cave_entrance", opening).await);
    let description = ok_text(&call(&client, "map_describe", json!({"map_id": "map-1"})).await);
    assert!(description.contains("with entrances"), "{description}");
    let warnings = ok_text(&call(&client, "map_validate", json!({"map_id": "map-1"})).await);
    assert_eq!(warnings, "No warning.");

    let destination = std::env::temp_dir().join(format!(
        "dd-mapper-caves-{}.dungeondraft_map",
        std::process::id()
    ));
    let save = json!({"map_id": "map-1", "path": destination.display().to_string()});
    ok_text(&call(&client, "map_save", save).await);
    let saved = MapFile::parse(&std::fs::read_to_string(&destination).unwrap()).unwrap();
    std::fs::remove_file(&destination).unwrap();
    let cave = &saved.world().levels["0"].cave;
    assert!(cave.bitmap.0.iter().any(|byte| *byte != 0));
    assert!(cave.entrance_bitmap.0.iter().any(|byte| *byte != 0));
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn searches_the_catalog() {
    let client = connect().await;
    let found = ok_text(
        &call(
            &client,
            "catalog_search",
            json!({"query": "crate", "kind": "object"}),
        )
        .await,
    );
    assert!(found.contains(CRATE), "{found}");
    assert!(found.contains("1.8 x 1.5 cells"), "{found}");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn errors_tell_the_model_what_to_do() {
    let client = connect().await;
    let map = ok_text(&call(&client, "map_new", json!({"width": 20, "height": 12})).await);
    let room = json!({
        "map_id": map,
        "shape": {"type": "rectangle", "x": 1, "y": 1, "width": 8, "height": 6},
        "floor": FLOOR,
        "wall": "res://packs/LOREGD01/textures/walls/Wall_Underhive_Industrail.webp",
    });
    let result = call(&client, "room_add", room).await;
    assert!(is_error(&result));
    let message = text(&result);
    assert!(
        message.contains(WALL) && message.contains("catalog_search"),
        "{message}"
    );
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn exposes_the_catalog_summary_as_a_resource() {
    let client = connect().await;
    let resources = client.list_all_resources().await.unwrap();
    let uris: Vec<String> = resources
        .iter()
        .map(|resource| resource.uri.clone())
        .collect();
    assert_eq!(uris, ["catalog://summary"]);
    let params = serde_json::from_value(json!({"uri": "catalog://summary"})).unwrap();
    let read = serde_json::to_value(client.read_resource(params).await.unwrap()).unwrap();
    assert!(
        read["contents"][0]["text"]
            .as_str()
            .unwrap()
            .starts_with("9 assets.")
    );
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn offers_a_design_prompt() {
    let client = connect().await;
    let prompts = client.list_all_prompts().await.unwrap();
    assert_eq!(prompts[0].name, "design_40k_location");
    let params = serde_json::from_value(json!({
        "name": "design_40k_location",
        "arguments": {"brief": "an underhive smuggler den"}
    }))
    .unwrap();
    let prompt = serde_json::to_value(client.get_prompt(params).await.unwrap()).unwrap();
    let guidance = prompt["messages"][0]["content"]["text"].as_str().unwrap();
    for step in [
        "an underhive smuggler den",
        "catalog_search",
        "kind portal",
        "wall_add",
        "light_add",
        "pattern_add",
        "map_preview",
        "map_save",
    ] {
        assert!(guidance.contains(step), "{step} missing from {guidance}");
    }
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn opens_a_map_and_saves_it_over_its_own_file() {
    let client = connect().await;
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/test2.dungeondraft_map");
    let opened = ok_text(
        &call(
            &client,
            "map_open",
            json!({"path": fixture.display().to_string()}),
        )
        .await,
    );
    assert_eq!(opened, "map-1");
    let description = ok_text(&call(&client, "map_describe", json!({"map_id": "map-1"})).await);
    assert!(description.contains("15 pillars"), "{description}");

    let destination = std::env::temp_dir().join(format!(
        "dd-mapper-reopened-{}.dungeondraft_map",
        std::process::id()
    ));
    let save = json!({"map_id": "map-1", "path": destination.display().to_string()});
    ok_text(&call(&client, "map_save", save).await);
    let saved = MapFile::parse(&std::fs::read_to_string(&destination).unwrap()).unwrap();
    std::fs::remove_file(&destination).unwrap();
    let original = MapFile::parse(&std::fs::read_to_string(&fixture).unwrap()).unwrap();
    let (level, before) = (&saved.world().levels["0"], &original.world().levels["0"]);
    assert_eq!(
        (
            level.objects.len(),
            level.lights.len(),
            level.patterns.len()
        ),
        (21, 6, 4)
    );
    assert_eq!(level.terrain.splat, before.terrain.splat);
    client.cancel().await.unwrap();
}
