#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BlockTag(u8);

const ENGINE_TAGS: &[&str] = &[
    "leaves",
    "log",
    "terrain",
    "no_grass_decay",
    "fragile",
    "replaceable",
    "soil",
    "sand",
    "no_pane_connect",
    "climbable",
    "snow_cover",
    "slippery",
    "sapling",
    "bed",
    "merges_with_self",
    "snow_bedded",
    "rock",
    "canopy",
    "nav_hazard",
    "axial",
    "stony",
    "spawn_proof",
];

static BLOCK_TAGS: crate::content::Slot<crate::registry::TagTable> =
    crate::content::Slot::new("block tags", &[], |_| {
        Ok(crate::registry::TagTable::new(ENGINE_TAGS))
    });

impl BlockTag {
    #[inline]
    pub(super) const fn id(self) -> u8 {
        self.0
    }

    pub const LEAVES: BlockTag = BlockTag(0);
    pub const LOG: BlockTag = BlockTag(1);
    pub const TERRAIN: BlockTag = BlockTag(2);
    pub const NO_GRASS_DECAY: BlockTag = BlockTag(3);
    pub const FRAGILE: BlockTag = BlockTag(4);
    pub const REPLACEABLE: BlockTag = BlockTag(5);
    pub const SOIL: BlockTag = BlockTag(6);
    pub const SAND: BlockTag = BlockTag(7);
    pub const NO_PANE_CONNECT: BlockTag = BlockTag(8);
    pub const CLIMBABLE: BlockTag = BlockTag(9);
    pub const SNOW_COVER: BlockTag = BlockTag(10);
    pub const SLIPPERY: BlockTag = BlockTag(11);
    pub const SAPLING: BlockTag = BlockTag(12);
    pub const BED: BlockTag = BlockTag(13);
    pub const MERGES_WITH_SELF: BlockTag = BlockTag(14);
    /// Ground decoration drawn standing in snow, the dual of [`SNOW_COVER`](Self::SNOW_COVER).
    ///
    /// A pebble and a snow layer both want the same cell. With this tag, when a
    /// horizontal neighbour is a `snow_cover` block, the mesher draws that
    /// block's boxes under the decoration. It's worked out from the neighbours at
    /// mesh time, so it heals the moment snow is placed or dug.
    pub const SNOW_BEDDED: BlockTag = BlockTag(15);
    pub const ROCK: BlockTag = BlockTag(16);
    pub const CANOPY: BlockTag = BlockTag(17);

    pub const NAV_HAZARD: BlockTag = BlockTag(18);
    pub const AXIAL: BlockTag = BlockTag(19);
    /// No spawner stands a body on this floor.
    pub const SPAWN_PROOF: BlockTag = BlockTag(21);

    pub fn resolve(name: &str) -> Result<BlockTag, String> {
        BLOCK_TAGS.current().resolve(name).map(BlockTag)
    }

    pub fn lookup(name: &str) -> Option<BlockTag> {
        BLOCK_TAGS.current().lookup(name).map(BlockTag)
    }

    /// The tag's key as rows and mods name it: `petramond:soil`, `farming:compostable`.
    pub fn key(self) -> String {
        let name = BLOCK_TAGS.current().name(self.0);
        if crate::registry::is_namespaced(name) {
            name.to_owned()
        } else {
            format!("petramond:{name}")
        }
    }
}
