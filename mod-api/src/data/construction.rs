use serde::{Deserialize, Serialize};

use super::*;

/// One cell, whole and by name: what [`BlockCall::GetCells`] reads and
/// [`BlockCall::SetCells`] writes, and what a blueprint or a carried block keeps.
///
/// `state` is the block's cell state with the bytes that name other blocks zeroed; `refs` names
/// those blocks by state offset, so a record survives a session (block ids do not). `fluid` is a
/// fluid block's level. `data` is the cell's data map (cell KV), including a held fluid.
///
/// [`BlockCall::GetCells`]: crate::BlockCall::GetCells
/// [`BlockCall::SetCells`]: crate::BlockCall::SetCells
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CellRecord {
    pub block: String,
    #[serde(with = "serde_bytes")]
    pub state: Vec<u8>,
    pub refs: Vec<(u8, String)>,
    pub fluid: u8,
    pub data: Vec<(String, Vec<u8>)>,
    pub container: Option<Vec<Option<ItemStackData>>>,
}

impl CellRecord {
    /// A block alone: default state, no data, no container.
    pub fn of(block: impl Into<String>) -> Self {
        Self {
            block: block.into(),
            state: Vec::new(),
            refs: Vec::new(),
            fluid: 0,
            data: Vec::new(),
            container: None,
        }
    }
}

/// The cells that belong together with a cell: a model block's footprint or a compound's members
/// (a door's two halves), the cell they are anchored at, and their facing when they turn.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CellGroupData {
    pub anchor: [i32; 3],
    pub facing: Option<crate::Facing>,
    pub cells: Vec<[i32; 3]>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RecordPlan {
    Air,
    Member {
        anchor: [i32; 3],
    },
    Unit {
        cost: Vec<ItemStackData>,
        footprint: Vec<[i32; 3]>,
    },
    Unsupported {
        reason: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RecordStatus {
    Unloaded,
    Satisfied,
    Place {
        missing: Vec<ItemStackData>,
    },
    Clear {
        at: [i32; 3],
        block: BlockId,
        footprint: Vec<[i32; 3]>,
        holds_items: bool,
    },
    Pending {
        anchor: [i32; 3],
    },
    Unsupported {
        reason: String,
    },
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ActionRefusal {
    NoActor,
    Unloaded,
    OutOfReach,
    NoLineOfSight,
    NothingToDo,
    Unbreakable,
    Obstructed,
    NoFace,
    NoSupport,
    BodyInTheWay,
    MissingItems,
    NoTool,
    Unsupported,
    NotOwned,
    Vetoed,
    Changed,
    NotAimed,
    Misaligned,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum DigProgress {
    Digging { progress: f32 },
    Breaking,
    Refused(ActionRefusal),
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum PlaceRequest {
    Queued,
    Satisfied,
    Refused(ActionRefusal),
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Route {
    Open,
    Closed,
    Undecided,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Flood {
    Reached(Vec<([i32; 3], u32)>),
    Exceeded,
    Deferred,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockChanges {
    pub next: u64,
    pub lost: bool,
    pub cells: Vec<[i32; 3]>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ActorAction {
    Dig,
    Place,
    Use,
}

pub type SchematicId = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SchematicInfoData {
    pub title: String,
    pub size: [i32; 3],
    pub cells: u64,
    pub sections: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SchematicLookup {
    Missing,
    Loading,
    Ready(SchematicInfoData),
    Failed { reason: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SchematicCellsData {
    pub cells: Vec<([i32; 3], u16)>,
    pub palette: Vec<CellRecord>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SchematicGhostData {
    pub asset: SchematicId,
    pub origin: [i32; 3],
    pub turns: u8,
    pub viewers: Vec<PlayerId>,
    pub yields_to_positioning: bool,
}
