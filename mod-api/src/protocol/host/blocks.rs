use crate::data::{CellRecord, RayFilter};
use crate::ids::BlockId;
use crate::legality::prelude::*;

host_domain! {
    /// Cells of the live world. Reads are stream-final gated: an unloaded or not yet final cell
    /// answers `None`/`false`, so retry rather than act on it. Reads legal on a client instance
    /// answer from its replica.
    BlockCall {
        /// Each cell's block. → [`HostRet::Blocks`](crate::HostRet::Blocks).
        GetBlocks {
            positions: Vec<[i32; 3]>,
        } => legal(SERVER_CLIENT, Sim, Read),
        /// Each cell whole: block, state, fluid level, data and container.
        /// → [`HostRet::Cells`](crate::HostRet::Cells).
        GetCells {
            positions: Vec<[i32; 3]>,
        } => legal(SERVER, Sim, Read),
        /// Writes each block in its default state; the cell's data and container go with the old
        /// block. Per entry: `false` = unloaded or not final. → [`HostRet::Bools`](crate::HostRet::Bools).
        SetBlocks {
            blocks: Vec<([i32; 3], BlockId)>,
        } => legal(SERVER, Sim, Write),
        /// Writes whole cells as one edit. A compound (a door's halves, a model block's footprint)
        /// is listed whole, and the cells a replaced compound leaves behind are cleared. All or
        /// nothing: `false` = an unloaded cell, an incomplete compound, a duplicate cell or an
        /// unknown block name. → [`HostRet::Bool`](crate::HostRet::Bool).
        SetCells {
            cells: Vec<([i32; 3], CellRecord)>,
        } => legal(SERVER, Sim, Write),
        /// Changes each cell's block in place, keeping its state, data, container and draw set,
        /// with the same footprint and no placement events (a furnace lighting). Both blocks are
        /// the caller's own. → [`HostRet::Bools`](crate::HostRet::Bools).
        SwapBlocks {
            swaps: Vec<([i32; 3], BlockId)>,
        } => legal(SERVER, Sim, Write),
        /// Breaks each cell the way the world does when a block loses its support: bare-hand
        /// drops, the container spilled, and the block's residue (the fluid it held or melts to)
        /// left in its place. `false` = unreadable or empty. → [`HostRet::Bools`](crate::HostRet::Bools).
        BreakBlocks {
            positions: Vec<[i32; 3]>,
        } => legal(SERVER, Sim, Write),
        /// Runs each cell's block behaviors' scheduled tick `delay` ticks from now. Pending ticks
        /// are saved with their section.
        ScheduleTicks {
            ticks: Vec<([i32; 3], u64)>,
        } => legal(SERVER, Sim, Write),
        /// Whether each cell is loaded and final. → [`HostRet::Bools`](crate::HostRet::Bools).
        IsLoaded {
            positions: Vec<[i32; 3]>,
        } => legal(SERVER_CLIENT, Sim, Read),
        /// → [`HostRet::Lights`](crate::HostRet::Lights).
        LightAt {
            positions: Vec<[i32; 3]>,
        } => legal(SERVER_CLIENT, Sim, Read),
        /// Each column's surface biome id. → [`HostRet::Biomes`](crate::HostRet::Biomes).
        BiomeAt {
            columns: Vec<[i32; 2]>,
        } => legal(SERVER_CLIENT, Sim, Read),
        /// Each column's top collidable y. → [`HostRet::MaybeI32s`](crate::HostRet::MaybeI32s).
        SurfaceYAt {
            columns: Vec<[i32; 2]>,
        } => legal(SERVER_CLIENT, Sim, Read),
        /// → [`HostRet::CollisionShapes`](crate::HostRet::CollisionShapes).
        CollisionShapeAt {
            positions: Vec<[i32; 3]>,
        } => legal(SERVER_CLIENT, Sim, Read),
        /// Finds every cell in the inclusive box `min..=max` matching one of `blocks`, scanned
        /// host-side in one pass (don't page through [`BlockCall::GetBlocks`] to search).
        /// Order is ascending y, then z, then x, so "nearest match" is just the caller folding over
        /// that list.
        /// Box capped at 32768 cells (32³), `blocks` at the sim batch cap. Inverted box is an
        /// error.
        /// Any unreadable cell makes the whole reply `None`, so a partial search can't race a
        /// saved overlay. Cells outside the world's vertical range count as empty and get
        /// clamped, not gated, or a box poking past the top would stall forever.
        /// → [`HostRet::FoundBlocks`](crate::HostRet::FoundBlocks).
        FindBlocks {
            min: [i32; 3],
            max: [i32; 3],
            blocks: Vec<BlockId>,
        } => legal(SERVER, Sim, Read),
        SetModelParts {
            sets: Vec<([i32; 3], u32, Option<[u8; 3]>)>,
        } => legal(SERVER, Sim, Write),
        SetBlockDraws {
            sets: Vec<([i32; 3], Vec<crate::DrawPrim>)>,
        } => legal(SERVER, Sim, Write),
        /// Points in a block's own (footprint) space, in world space.
        BlockLocalToWorld {
            pos: [i32; 3],
            points: Vec<[f32; 3]>,
        } => legal(SERVER_CLIENT, Sim, Read),
        Raycast {
            from: [f64; 3],
            dir: [f32; 3],
            max: f32,
            filter: RayFilter,
        } => legal(SERVER_CLIENT, Sim, Read),
        BlockChangesSince {
            since: Option<u64>,
        } => legal(SERVER, Sim, Read),
        /// Whether a block would be held at each cell by the world's support rule: a fragile row
        /// by its declared support, every other block always. `None` asks about the block already
        /// there, `Some` about that block standing there instead. Unreadable cells answer `true`
        /// (nothing to act on). → [`HostRet::Bools`](crate::HostRet::Bools).
        SupportedAt {
            cells: Vec<([i32; 3], Option<BlockId>)>,
        } => legal(SERVER_CLIENT, Sim, Read),
        /// For each cell holding fluid, that fluid and the world height of its open surface (the
        /// top of the column the cell is in, as the world renders and floats bodies on it).
        /// `None` = no fluid or unreadable. → [`HostRet::FluidSurfaces`](crate::HostRet::FluidSurfaces).
        FluidSurfaceAt {
            positions: Vec<[i32; 3]>,
        } => legal(SERVER_CLIENT, Sim, Read),
        /// The group each cell belongs to; `None` = a lone cell or unreadable.
        /// → [`HostRet::CellGroups`](crate::HostRet::CellGroups).
        CellGroup {
            positions: Vec<[i32; 3]>,
        } => legal(SERVER_CLIENT, Sim, Read),
        /// How each cell compares to the record wanted there: satisfied, what placing it would
        /// cost, what stands in the way, or the anchor it waits on.
        /// → [`HostRet::RecordStatuses`](crate::HostRet::RecordStatuses).
        RecordStatuses {
            cells: Vec<([i32; 3], CellRecord)>,
        } => legal(SERVER, Sim, Read),
        /// Grows a `features.json` feature at `pos`, seeded by `seed`, the way a sapling grows its
        /// tree: its anchor check runs against the live world, and it commits only if every cell
        /// it writes is air, a replaceable block, one of `over` (replaced) or one of `keep` (grown
        /// through and left standing). All or nothing. → [`HostRet::Bool`](crate::HostRet::Bool).
        PlaceFeature {
            feature: String,
            pos: [i32; 3],
            seed: u64,
            over: Vec<BlockId>,
            keep: Vec<BlockId>,
        } => legal(SERVER, Sim, Write),
        /// Whether each world-space box (`min`, `max`) is clear of collidable terrain. Unreadable
        /// cells count as blocked. → [`HostRet::Bools`](crate::HostRet::Bools).
        BoxesClear {
            boxes: Vec<([f64; 3], [f64; 3])>,
        } => legal(SERVER_CLIENT, Sim, Read),
    }
}
