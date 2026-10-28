//! Acceptance test on the packs installed in `assets/packs` (not versioned).
//! Run with `cargo test -p dd-catalog-io --test real_packs -- --ignored`.

use std::path::Path;

use dd_catalog::{AssetKind, AssetQuery, AssetSource, Catalog};
use dd_catalog_io::PackIndexSource;

fn installed_packs() -> Vec<PackIndexSource> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let mut archives: Vec<_> = std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("{}: {error}", directory.display()))
        .map(|entry| entry.expect("readable directory").path())
        .collect();
    archives.sort();
    archives.into_iter().map(PackIndexSource::new).collect()
}

#[test]
#[ignore = "needs the licensed packs in assets/packs"]
fn indexes_every_installed_pack() {
    let packs = installed_packs();
    let sources: Vec<&dyn AssetSource> =
        packs.iter().map(|pack| pack as &dyn AssetSource).collect();
    let started = std::time::Instant::now();
    let catalog = Catalog::from_sources(&sources).expect("readable packs");
    println!("{} assets in {:?}", catalog.len(), started.elapsed());
    assert!(catalog.len() > 10_000);

    let crate_path = "res://packs/LOREGD01/textures/objects/[LORE] Containers/Container_Large_Metal_Crate_Green.webp";
    let crate_size = catalog
        .get(crate_path)
        .and_then(|asset| asset.metrics.size)
        .expect("sized crate");
    assert_eq!(
        (crate_size.width, crate_size.height),
        (452.0 / 256.0, 390.0 / 256.0)
    );
    let globe_path = "res://textures/objects/activities/administration/atlas_globe_01.png";
    let globe_size = catalog
        .get(globe_path)
        .and_then(|asset| asset.metrics.size)
        .expect("sized globe");
    assert_eq!(
        (globe_size.width, globe_size.height),
        (87.0 / 256.0, 88.0 / 256.0)
    );
    let unsized_objects = catalog
        .search(&AssetQuery {
            kind: Some(AssetKind::Object),
            ..AssetQuery::default()
        })
        .into_iter()
        .filter(|asset| asset.metrics.size.is_none())
        .count();
    assert_eq!(unsized_objects, 0);

    let doors = catalog.search(&AssetQuery {
        text: "door underhive industrial".to_owned(),
        kind: Some(AssetKind::Portal),
        ..AssetQuery::default()
    });
    let names: Vec<&str> = doors.iter().map(|door| door.name.as_str()).collect();
    assert!(names.contains(&"Door_Underhive_Industrial"), "{names:?}");

    let tavern = catalog.search(&AssetQuery {
        tags: vec!["Tavern".to_owned()],
        pack: Some("default".to_owned()),
        ..AssetQuery::default()
    });
    assert_ne!(tavern, Vec::<&dd_catalog::Asset>::new());

    let crates = catalog.search(&AssetQuery {
        text: "crate".to_owned(),
        pack: Some("LOREGD01".to_owned()),
        limit: 5,
        ..AssetQuery::default()
    });
    for asset in &crates {
        println!("{} {:?}", asset.path.as_str(), asset.tags);
    }
    assert_eq!(crates.len(), 5);
}
