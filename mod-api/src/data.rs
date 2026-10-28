use serde::{Deserialize, Serialize};

use crate::ids::{BlockId, ItemId, MobId, PlayerId};

mod ai;
mod batch;
mod construction;

pub use ai::*;
pub use batch::*;
pub use construction::*;

pub const MAX_MOB_ANIM_NAME_BYTES: usize = 64;

pub const MAX_MOB_ANIM_PHASE_MAGNITUDE: f32 = 1_000_000.0;

pub const MAX_MOB_ANIM_RATE_MAGNITUDE: f32 = 1_000.0;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum GuiValue {
    F32(f32),
    I32(i32),
    Str(String),
    List(Vec<std::collections::BTreeMap<String, Self>>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum MobTagValue {
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum MobTagLookup {
    MissingMob,
    Absent,
    Value(MobTagValue),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MobSnapshot {
    pub index: u32,
    pub kind: MobId,
    pub pos: [f64; 3],
    pub health: f32,
    pub id: u64,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub vel: [f32; 3],
    pub on_ground: bool,
    pub moving: bool,
    pub half_width: f32,
    pub height: f32,
    pub half_length: f32,
    pub entombed: bool,
    pub conditions: Vec<ConditionData>,
    /// The brain's current combat lock; absent while inspecting or escaping.
    pub target: Option<EntityRef>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ItemEntityData {
    pub id: u64,
    pub stack: ItemStackData,
    pub owner: Option<EntityRef>,
    pub pos: [f64; 3],
    pub vel: [f32; 3],
    pub motion: ItemMotion,
}

/// One entity, whichever body it is.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum EntityData {
    Mob(MobSnapshot),
    Item(ItemEntityData),
}

impl EntityData {
    pub fn id(&self) -> u64 {
        match self {
            EntityData::Mob(m) => m.id,
            EntityData::Item(i) => i.id,
        }
    }

    pub fn pos(&self) -> [f64; 3] {
        match self {
            EntityData::Mob(m) => m.pos,
            EntityData::Item(i) => i.pos,
        }
    }

    pub fn mob(self) -> Option<MobSnapshot> {
        match self {
            EntityData::Mob(m) => Some(m),
            EntityData::Item(_) => None,
        }
    }

    pub fn item(self) -> Option<ItemEntityData> {
        match self {
            EntityData::Item(i) => Some(i),
            EntityData::Mob(_) => None,
        }
    }
}

/// Which entities a query takes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum EntityFilter {
    All,
    Mobs,
    /// Mobs of these species only.
    MobsOf(Vec<MobId>),
    Items,
}

/// A despawned entity's remains: the stack an item entity held (a mob leaves none).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Despawned {
    pub stack: Option<ItemStackData>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub enum ItemMotion {
    Loose,
    Flight,
    Stuck { cell: [i32; 3] },
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EntityRef {
    Player(PlayerId),
    Entity(u64),
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RayFilter {
    Selectable,
    Collidable,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub struct RaycastHitData {
    pub block: [i32; 3],
    pub face: [i32; 3],
    pub distance: f32,
}

/// Item row data key of [`ItemFlight`] (with the engine's own presentation fields beside it).
pub const PROJECTILE_KEY: &str = "petramond:projectile";

/// The gravity an item flies under when its row names none (blocks/s²).
pub const DEFAULT_ITEM_GRAVITY: f32 = 20.0;

/// How an item entity flies: each tick gravity, then drag, then the move. The engine flies items
/// by exactly this, so a mod aiming or throwing one can fly it ahead of time.
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct ItemFlight {
    pub gravity: f32,
    /// The share of its speed it loses each second (0..=1).
    pub drag: f32,
}

impl Default for ItemFlight {
    fn default() -> Self {
        Self {
            gravity: DEFAULT_ITEM_GRAVITY,
            drag: 0.0,
        }
    }
}

impl ItemFlight {
    /// The share of its speed a flight keeps over `dt` seconds.
    pub fn drag_factor(self, dt: f32) -> f32 {
        (1.0 - self.drag).max(0.0).powf(dt)
    }

    /// The velocity after one step of `dt` seconds; the move is that velocity times `dt`.
    pub fn step(self, vel: [f32; 3], dt: f32) -> [f32; 3] {
        let k = self.drag_factor(dt);
        [vel[0] * k, (vel[1] - self.gravity * dt) * k, vel[2] * k]
    }
}

/// Stack data key of [`ItemEntityRules`].
pub const ITEM_ENTITY_KEY: &str = "petramond:item_entity";

/// How a stack behaves while it is an entity, carried as its `petramond:item_entity` data
/// through flight, lodging, dropping and saves. Give it at launch with [`ItemEntityRules::data`].
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ItemEntityRules {
    /// Whether a player walking into it takes it.
    pub pickup: bool,
    /// Ticks until it goes; `None` = a dropped item's usual life.
    pub lifetime_ticks: Option<u32>,
}

impl Default for ItemEntityRules {
    fn default() -> Self {
        Self {
            pickup: true,
            lifetime_ticks: None,
        }
    }
}

impl ItemEntityRules {
    /// The stack data entry that carries these rules.
    pub fn data(&self) -> (&'static str, Vec<u8>) {
        let mut json = format!("{{\"pickup\":{}", self.pickup);
        if let Some(ticks) = self.lifetime_ticks {
            json.push_str(&format!(",\"lifetime_ticks\":{ticks}"));
        }
        json.push('}');
        (ITEM_ENTITY_KEY, json.into_bytes())
    }
}

/// Where a steerable sound plays.
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum SoundAt {
    Point([f64; 3]),
    /// A live mob or item entity, followed as it moves.
    Entity(u64),
}

/// Where an attached player's body is pinned.
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum AttachTo {
    /// One of an entity's row `seats`, carried with its body.
    Seat { entity: u64, seat: u8 },
    /// A fixed world point, the body facing `yaw`. One body per point.
    At { pos: [f64; 3], yaw: f32 },
}

/// The shape an attached body takes. `Seated`: feet at the point, legs bent. `Lying`: flat on its
/// back, centred on the point, head toward the facing.
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Posture {
    Seated,
    Lying,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub struct Attachment {
    pub to: AttachTo,
    pub posture: Posture,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub struct MobRiderData {
    pub seat: u8,
    pub player_id: PlayerId,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MobRidersData {
    pub capacity: u8,
    pub riders: Vec<MobRiderData>,
}

impl MobRidersData {
    pub fn first_free_seat(&self) -> Option<u8> {
        (0..self.capacity).find(|s| !self.riders.iter().any(|r| r.seat == *s))
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub struct MobAnimStateData {
    pub phase: f32,
    pub rate: f32,
    pub seek: Option<f32>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub struct PlayerInputData {
    pub forward: f32,
    pub strafe: f32,
    pub jump: bool,
    pub sneak: bool,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Default)]
pub struct HeldPoseData {
    pub rotation: [f32; 3],
    pub translation: [f32; 3],
}

impl HeldPoseData {
    pub const IDENTITY: Self = Self {
        rotation: [0.0; 3],
        translation: [0.0; 3],
    };

    pub fn is_identity(&self) -> bool {
        *self == Self::IDENTITY
    }

    pub fn is_finite(&self) -> bool {
        self.rotation
            .iter()
            .chain(&self.translation)
            .all(|c| c.is_finite())
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Default)]
pub struct HeldPose {
    pub first_person: HeldPoseData,
    pub third_person: HeldPoseData,
}

impl HeldPose {
    pub const IDENTITY: Self = Self {
        first_person: HeldPoseData::IDENTITY,
        third_person: HeldPoseData::IDENTITY,
    };

    pub fn is_identity(&self) -> bool {
        self.first_person.is_identity() && self.third_person.is_identity()
    }

    pub fn is_finite(&self) -> bool {
        self.first_person.is_finite() && self.third_person.is_finite()
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum BonePoseMode {
    #[default]
    Compose,
    Replace,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct BonePoseData {
    pub bone: String,
    pub rotation: [f32; 3],
    pub translation: [f32; 3],
    pub mode: BonePoseMode,
}

impl BonePoseData {
    pub fn is_finite(&self) -> bool {
        self.rotation
            .iter()
            .chain(&self.translation)
            .all(|c| c.is_finite())
    }
}

pub mod bone {
    pub const HEAD: &str = "head";
    pub const BODY: &str = "body";
    pub const WAIST: &str = "waist";

    pub const MAIN_SHOULDER: &str = "left_shoulder";
    pub const MAIN_ELBOW: &str = "left_elbow";
    pub const OFF_SHOULDER: &str = "right_shoulder";
    pub const OFF_ELBOW: &str = "right_elbow";
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BodyAction {
    Attack,
    Mine,
    Use,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PlayerAttribute {
    MoveSpeed,
    AttackCooldown,
    FlySpeed,
    /// Breath spent per tick away from air (0 = never runs out).
    BreathDrain,
}

impl PlayerAttribute {
    pub const ALL: [PlayerAttribute; 4] = [
        Self::MoveSpeed,
        Self::AttackCooldown,
        Self::FlySpeed,
        Self::BreathDrain,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }

    /// The attribute's name in content rows.
    pub const fn key(self) -> &'static str {
        match self {
            Self::MoveSpeed => "move_speed",
            Self::AttackCooldown => "attack_cooldown",
            Self::FlySpeed => "fly_speed",
            Self::BreathDrain => "breath_drain",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.key() == key)
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SwingKind {
    Attack,
    Break,
    Place,
    Throw,
    Interact,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct HandSwing {
    pub mining: bool,
    pub main: Option<SwingKind>,
    pub off: Option<SwingKind>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerSnapshot {
    pub id: Option<PlayerId>,
    pub pos: [f64; 3],
    pub vel: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub health: i32,
    pub on_ground: bool,
    pub spectator: bool,
    pub sneak: bool,
    pub held: Option<ItemId>,
    pub off_held: Option<ItemId>,
    pub use_held: bool,
    pub holds_use: bool,
    pub held_count: u8,
    pub attached: Option<Attachment>,
    pub swing: HandSwing,
    pub half_width: f32,
    pub height: f32,
    pub eye_height: f32,
    pub entombed: bool,
    pub conditions: Vec<ConditionData>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub struct ConditionData {
    pub condition: crate::ConditionId,
    pub stage: u8,
    pub remaining: u32,
    pub elapsed: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ConditionInfoData {
    pub id: crate::ConditionId,
    pub key: String,
    pub stages: Vec<String>,
}

impl ConditionInfoData {
    pub fn stage(&self, name: &str) -> Option<u8> {
        self.stages.iter().position(|s| s == name).map(|i| i as u8)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerListEntry {
    pub id: PlayerId,
    pub state: PlayerSnapshot,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GuiViewerData {
    pub player_id: PlayerId,
    pub kind: String,
    pub anchor: Option<ContainerAddress>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub enum RuntimeSide {
    Server,
    Worldgen,
    Client,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DrawPrim {
    /// An axis-aligned box wearing an atlas TILE (the same names a block row's
    /// `tiles` use). `tint` multiplies it; `emissive` lifts it out of the
    /// cell's light so molten metal glows in a dark forge.
    ///
    /// The tile maps over the box's faces in CELL units, so a face longer than
    /// one block along either of its axes gets the tile STRETCHED rather than
    /// repeated — a prim is a machine's moving part, not a wall. Split a long
    /// run into per-cell boxes if you want the texture to tile.
    Cuboid {
        min: [f32; 3],
        max: [f32; 3],
        tile: String,
        tint: [u8; 3],
        emissive: bool,
    },
    /// An ITEM, drawn the way the game draws that item everywhere else — its
    /// sprite extruded, or its bbmodel — so a mould in a basin and the mould
    /// in your hand cannot drift apart when the art changes. `scale` is in
    /// block units (1.0 = one block wide).
    ///
    /// `pitch` (about +X) is applied BEFORE `yaw` (about +Y), both in radians.
    /// Pitch is not decoration: a sprite item is a VERTICAL slab, so laying
    /// one flat in a basin is `pitch = FRAC_PI_2` and nothing else will do it.
    Item {
        at: [f32; 3],
        scale: f32,
        yaw: f32,
        pitch: f32,
        item: String,
        tint: [u8; 3],
    },
    Sprite {
        at: [f32; 3],
        scale: f32,
        yaw: f32,
        pitch: f32,
        spin: f32,
        bob: [f32; 2],
        faces_viewer: bool,
        tile: String,
        tint: [u8; 3],
        emissive: bool,
    },
    /// A rope hanging between `from` and `to`, wearing atlas `tile` (once a block along it)
    /// tinted `tint`, `width` texels thick (at most [`LINE_WIDTH_MAX`]). It is drawn beside what
    /// the set shows: an item entity whose set holds only lines keeps its art.
    ///
    /// [`LINE_WIDTH_MAX`]: crate::LINE_WIDTH_MAX
    Line {
        from: LineEnd,
        to: LineEnd,
        tile: String,
        tint: [u8; 3],
        width: f32,
    },
}

/// Where a [`DrawPrim::Line`] ends: a point in the set's own frame, on a body, or at a world
/// point. A player end ties on at their main hand, an item entity's where it is drawn (its top,
/// a model's trailing end), a mob's above its middle.
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum LineEnd {
    Local([f32; 3]),
    Entity(EntityRef),
    At([f64; 3]),
}

impl LineEnd {
    pub fn is_finite(&self) -> bool {
        match self {
            LineEnd::Local(p) => p.iter().all(|v| v.is_finite()),
            LineEnd::At(p) => p.iter().all(|v| v.is_finite()),
            LineEnd::Entity(_) => true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ParticleTexture {
    Tile {
        tile: String,
        slice: [f32; 4],
        tint: [u8; 3],
    },
    Block {
        block: BlockId,
        tint: Option<[u8; 3]>,
    },
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ContainerAddress {
    Block([i32; 3]),
    Entity(u64),
}

/// Whose GUI state a write reaches: one player's, or every player with a GUI open anchored there.
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GuiAudience {
    Player(PlayerId),
    Viewers(ContainerAddress),
}

impl From<PlayerId> for GuiAudience {
    fn from(player: PlayerId) -> Self {
        GuiAudience::Player(player)
    }
}

impl From<ContainerAddress> for GuiAudience {
    fn from(at: ContainerAddress) -> Self {
        GuiAudience::Viewers(at)
    }
}

impl From<[i32; 3]> for ContainerAddress {
    fn from(pos: [i32; 3]) -> Self {
        Self::Block(pos)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PlayerIdentityData {
    pub name: String,
    pub operator: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemStackData {
    pub item: String,
    pub count: u8,
    pub data: Vec<(String, Vec<u8>)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ItemInfoData {
    pub max_stack: u8,
    pub tags: Vec<String>,
    pub display_name: String,
    pub block: Option<BlockId>,
    pub tool: Option<ToolInfoData>,
    pub food: Option<FoodInfoData>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ToolInfoData {
    pub kind: String,
    pub tier: u8,
    pub speed: f32,
    pub damage: [f32; 2],
    pub knockback: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlockInfoData {
    pub material: String,
    pub hardness: f32,
    pub harvest_tier: u8,
    pub preferred_tool: Option<String>,
    pub item: Option<ItemId>,
    pub collision: Vec<([f32; 3], [f32; 3])>,
    pub fluid: Option<FluidInfoData>,
    /// The fluid a body in this block is immersed in: the block itself for a fluid, the fluid it
    /// holds for a block that holds one (seagrass in water).
    pub medium: Option<BlockId>,
    pub replaceable: bool,
    /// The GUI kind a use-click opens here.
    pub open_gui: Option<String>,
    /// The row's behavior keys, in order (`door`, `fragile`, `farming:crop`).
    pub behaviors: Vec<String>,
    /// A use-click here is the block's own to answer: it opens `open_gui`, or a behavior answers
    /// it (a door swings, a bed sleeps).
    pub answers_use: bool,
    /// Tag keys of the ground this block can stand on (`petramond:soil`); empty = any ground.
    pub roots_on: Vec<String>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DrawFrame {
    Body,
    World,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FluidInfoData {
    pub delay: u64,
    pub drop_off: u8,
    pub renewable: bool,
    pub quench: Option<QuenchData>,
    pub contact_damage: Option<PulseData>,
    pub applies: Option<ConditionGrantData>,
    pub clears: Vec<crate::ConditionId>,
    pub destroys_items: bool,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub struct QuenchData {
    pub by: BlockId,
    pub result: BlockId,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub struct PulseData {
    pub amount: i32,
    pub interval: u32,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub struct ConditionGrantData {
    pub condition: crate::ConditionId,
    pub stage: u8,
    pub ticks: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FoodInfoData {
    pub eat_ticks: u32,
    pub effects: Vec<FoodEffectData>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FoodEffectData {
    pub effect: String,
    pub ticks: u32,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlockHookKind {
    RandomTick,
    ScheduledTick,
    NeighborUpdate,
    /// The block's container changed (any slot, from any source) since the last tick.
    ContainerChanged,
    /// A player opened or closed a GUI anchored at this block since the last tick.
    ViewersChanged,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct EffectStateData {
    pub key: String,
    pub remaining: u32,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub struct LightData {
    pub combined: u8,
    pub sky: u8,
    pub block: u8,
    pub block_rgb: [u8; 3],
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub enum CollisionShape {
    Empty,
    Partial,
    Full,
}

pub mod rig {
    pub const PLAYER_BODY: &str = "player_body";
    pub const PLAYER_FIRST_PERSON: &str = "player_first_person";
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AnimatorValue {
    Number(f32),
    Name(String),
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum AnimatorClock {
    Scrub(f32),
    Run { rate: f32, looping: bool },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AnimatorParam {
    pub rig: String,
    pub param: String,
    pub value: AnimatorValue,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AnimatorPlay {
    pub rig: String,
    pub slot: String,
    pub clip: String,
    pub clock: AnimatorClock,
    pub mirror: bool,
    pub priority: i32,
}

impl AnimatorPlay {
    fn new(rig: &str, slot: &str, clip: &str, clock: AnimatorClock) -> Self {
        AnimatorPlay {
            rig: rig.to_string(),
            slot: slot.to_string(),
            clip: clip.to_string(),
            clock,
            mirror: false,
            priority: 0,
        }
    }

    pub fn scrubbed(rig: &str, slot: &str, clip: &str, progress: f32) -> Self {
        Self::new(rig, slot, clip, AnimatorClock::Scrub(progress))
    }

    pub fn running(rig: &str, slot: &str, clip: &str, rate: f32, looping: bool) -> Self {
        Self::new(rig, slot, clip, AnimatorClock::Run { rate, looping })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AnimationClipInfo {
    pub length: f32,
    pub looping: bool,
    pub markers: Vec<(String, f32)>,
}
