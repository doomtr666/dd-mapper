use std::fmt;

/// What an asset is used for, from the folder Dungeondraft stores it in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AssetKind {
    Tileset,
    Wall,
    Portal,
    Object,
    Light,
    Pattern,
    Terrain,
    Path,
}

const ALL: [AssetKind; 8] = [
    AssetKind::Tileset,
    AssetKind::Wall,
    AssetKind::Portal,
    AssetKind::Object,
    AssetKind::Light,
    AssetKind::Pattern,
    AssetKind::Terrain,
    AssetKind::Path,
];

impl AssetKind {
    /// The `textures/<folder>/` Dungeondraft uses for this kind.
    #[must_use]
    pub const fn folder(self) -> &'static str {
        match self {
            Self::Tileset => "tilesets",
            Self::Wall => "walls",
            Self::Portal => "portals",
            Self::Object => "objects",
            Self::Light => "lights",
            Self::Pattern => "patterns",
            Self::Terrain => "terrain",
            Self::Path => "paths",
        }
    }

    #[must_use]
    pub fn from_folder(folder: &str) -> Option<Self> {
        ALL.into_iter().find(|kind| kind.folder() == folder)
    }
}

impl fmt::Display for AssetKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.folder())
    }
}

#[cfg(test)]
mod tests {
    use super::AssetKind;

    #[test]
    fn kinds_come_from_dungeondraft_texture_folders() {
        assert_eq!(AssetKind::from_folder("walls"), Some(AssetKind::Wall));
        assert_eq!(AssetKind::from_folder("terrain"), Some(AssetKind::Terrain));
        assert_eq!(AssetKind::from_folder("roofs"), None);
    }

    #[test]
    fn folder_and_kind_round_trip() {
        for kind in super::ALL {
            assert_eq!(AssetKind::from_folder(kind.folder()), Some(kind));
        }
    }
}
