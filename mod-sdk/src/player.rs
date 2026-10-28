use mod_api::{
    BodyAction, BonePoseData, EffectStateData, EntityRef, HeldPose, PlayerAttribute, PlayerId,
    PlayerInputData, PlayerSnapshot,
};

use crate::__rt::host_fn;
use crate::__rt::try_host_fn;

try_host_fn! {
    pub fn try_player_inputs(player_ids: Vec<PlayerId>) -> Vec<Option<PlayerInputData>>
        => PlayerInputs { player_ids } => PlayerInputs
}

pub fn player_facing_xz(yaw: f32) -> [f32; 2] {
    let (s, c) = yaw.sin_cos();
    [s, c]
}

host_fn! {
    pub fn acting_player() -> Option<PlayerId> => ActingPlayer => ActingPlayer
}

host_fn! {
    /// `player`'s snapshot; `None` = no such reachable session (on a client instance: anyone but
    /// the local player, or outside a prediction dispatch).
    pub fn player_state(player: PlayerId) -> Option<Box<PlayerSnapshot>>
        => PlayerState { player } => Player
}

pub fn player_input(player: PlayerId) -> Option<PlayerInputData> {
    player_inputs(vec![player]).pop().flatten()
}

host_fn! {
    pub fn player_inputs(player_ids: Vec<PlayerId>) -> Vec<Option<PlayerInputData>>
        => PlayerInputs { player_ids } => PlayerInputs
}

host_fn! {
    pub fn player_held(player: PlayerId) -> Option<mod_api::ItemStackData>
        => PlayerHeld { player } => HeldStack
}

host_fn! {
    pub fn player_inventory(player: PlayerId) -> Option<Vec<Option<mod_api::ItemStackData>>>
        => PlayerInventory { player } => ContainerSlots
}

pub fn player_item_count(player: PlayerId, item: &str) -> u32 {
    player_inventory(player)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|stack| stack.item == item)
        .map(|stack| u32::from(stack.count))
        .sum()
}

host_fn! {
    pub fn take_item(
        player: PlayerId,
        item: &str,
        count: u8,
        data: Option<&[(&str, &[u8])]>,
    ) -> Option<mod_api::ItemStackData>
        => TakeItem {
            player,
            item: item.into(),
            count,
            data: data.map(|d| d.iter().map(|(k, v)| (k.to_string(), v.to_vec())).collect()),
        } => ItemStack
}

host_fn! {
    pub fn consume_held(player: PlayerId, item: mod_api::ItemId, count: u32) -> bool
        => ConsumeHeld { player, item, count } => Bool
}

host_fn! {
    pub fn replace_held_one(player: PlayerId, item: mod_api::ItemId, replacement: &str) -> bool
        => ReplaceHeldOne { player, item, replacement: replacement.into() } => Bool
}

host_fn! {
    pub fn give_item(player: PlayerId, item: &str, count: u8, data: &[(&str, &[u8])]) -> bool
        => GiveItem {
            player,
            item: item.into(),
            count,
            data: data.iter().map(|(k, v)| (k.to_string(), v.to_vec())).collect(),
        } => Bool
}

host_fn! {
    /// Rewrite the instance data on the stack `player` is HOLDING, iff it is
    /// still an `expect_item` carrying exactly `expect_data` — the
    /// compare-and-set a wear system needs to restamp a tool between the event
    /// it observed and this write. The compare covers the VALUE being
    /// replaced, not just the item: a hand swapped OR the same stack
    /// re-stamped by another handler or another mod in between refuses rather
    /// than clobbers, so two writers in one tick cannot silently drop one of
    /// the two updates. Pass the map you READ off the stack as `expect_data`
    /// (`&[]` expects a plain stack) and the FULL replacement as `data` (≤4
    /// namespaced keys; `&[]` clears it); the item and count stay. `false` =
    /// empty/other hand, data that no longer matches `expect_data`, unknown
    /// item name, or no such connected session — re-read the stack and
    /// recompute rather than retrying the same write.
    pub fn set_player_held_data(
        player: PlayerId,
        expect_item: &str,
        expect_data: &[(&str, &[u8])],
        data: &[(&str, &[u8])],
    ) -> bool
        => SetPlayerHeldData {
            player,
            expect_item: expect_item.into(),
            expect_data: expect_data.iter().map(|(k, v)| (k.to_string(), v.to_vec())).collect(),
            data: data.iter().map(|(k, v)| (k.to_string(), v.to_vec())).collect(),
        } => Bool
}

host_fn! {
    pub fn set_health(player: PlayerId, value: i32) -> bool
        => SetHealth { player, value } => Bool
}

host_fn! {
    pub fn teleport(player: PlayerId, pos: [f64; 3]) -> bool
        => Teleport { player, pos } => Bool
}

host_fn! {
    pub fn effect_apply(player: PlayerId, key: &str, ticks: u32) -> bool
        => EffectApply { player, key: key.into(), ticks } => Bool
}

pub fn effect_remove(player: PlayerId, key: &str) -> bool {
    effect_apply(player, key, 0)
}

host_fn! {
    pub fn effects_active(player: PlayerId) -> Option<Vec<EffectStateData>>
        => EffectsActive { player } => Effects
}

host_fn! {
    pub fn chat_send(text: &str, targets: Option<&[PlayerId]>) -> bool
        => ChatSend {
            text: text.into(),
            targets: targets.map(|ids| ids.to_vec()),
        } => Bool
}

host_fn! {
    pub fn players() -> Vec<mod_api::PlayerListEntry> => Players => Players
}

host_fn! {
    pub fn unlock_recipe(player: PlayerId, recipe: &str) -> bool
        => UnlockRecipe { player, recipe: recipe.into() } => Bool
}

host_fn! {
    pub fn recipe_unlocked(player: PlayerId, recipe: &str) -> bool
        => RecipeUnlocked { player, recipe: recipe.into() } => Bool
}

host_fn! {
    pub fn set_player_attribute(player: PlayerId, attribute: PlayerAttribute, scale: f32) -> bool
        => SetPlayerAttribute { player, attribute, scale } => Bool
}

host_fn! {
    pub fn set_player_held_pose(
        player: PlayerId,
        main: Option<HeldPose>,
        off: Option<HeldPose>,
    ) -> bool => SetPlayerHeldPose { player, main, off } => Bool
}

host_fn! {
    pub fn set_held_display(body: EntityRef, main: Option<&str>, off: Option<&str>) -> bool
        => SetHeldDisplay {
            body,
            main: main.map(str::to_owned),
            off: off.map(str::to_owned),
        } => Bool
}

host_fn! {
    pub fn set_player_bone_pose(player: PlayerId, bones: Vec<BonePoseData>) -> bool
        => SetPlayerBonePose { player, bones } => Bool
}

host_fn! {
    /// Grabs player's use gesture (one interact press), holds it till they let go. `false` if no
    /// reachable session.
    ///
    /// Gesture has one owner. Most interactions leave it free, which is what lets a held button
    /// keep placing blocks. Continuous uses hold it, and nothing else gets the button meanwhile.
    /// [`PlayerSnapshot::holds_use`] is true only for you, check against that, not the raw button.
    ///
    /// Call from [`EventKind::UseUnclaimed`], the fallback after the interact chain runs. Taking
    /// the press doesn't touch the world, so pose the body yourself.
    ///
    /// [`EventKind::UseUnclaimed`]: mod_api::EventKind::UseUnclaimed
    pub fn hold_use(player: PlayerId) -> bool => HoldUse { player } => Bool
}

host_fn! {
    pub fn set_player_denied_actions(player: PlayerId, actions: Vec<BodyAction>) -> bool
        => SetPlayerDeniedActions { player, actions } => Bool
}

host_fn! {
    pub fn set_player_animator_params(player: PlayerId, params: Vec<mod_api::AnimatorParam>) -> bool
        => SetPlayerAnimatorParams { player, params } => Bool
}

host_fn! {
    pub fn set_player_animator_plays(player: PlayerId, plays: Vec<mod_api::AnimatorPlay>) -> bool
        => SetPlayerAnimatorPlays { player, plays } => Bool
}

host_fn! {
    pub fn fire_player_animator_event(player: PlayerId, rig: &str, event: &str) -> bool
        => FirePlayerAnimatorEvent { player, rig: rig.into(), event: event.into() } => Bool
}

host_fn! {
    pub fn animation_clip(rig: &str, clip: &str) -> Option<mod_api::AnimationClipInfo>
        => AnimationClip { rig: rig.into(), clip: clip.into() } => AnimationClip
}

host_fn! {
    pub fn player_identity(player: PlayerId) -> Option<mod_api::PlayerIdentityData>
        => PlayerIdentity { player } => Identity
}

/// A player's view frame: the eye and the unit axes its look runs along.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Aim {
    pub eye: [f64; 3],
    pub forward: [f32; 3],
    /// Level, toward the player's left.
    pub across: [f32; 3],
    pub up: [f32; 3],
}

impl Aim {
    pub fn of(state: &mod_api::PlayerSnapshot) -> Aim {
        let (cp, sp) = (state.pitch.cos(), state.pitch.sin());
        let (sy, cy) = (state.yaw.sin(), state.yaw.cos());
        let forward = [sy * cp, sp, cy * cp];
        let across = [cy, 0.0, -sy];
        let up = [
            across[1] * forward[2] - across[2] * forward[1],
            across[2] * forward[0] - across[0] * forward[2],
            across[0] * forward[1] - across[1] * forward[0],
        ];
        Aim {
            eye: [
                state.pos[0],
                state.pos[1] + f64::from(state.eye_height),
                state.pos[2],
            ],
            forward,
            across,
            up,
        }
    }

    /// The point `right`, `up` and `ahead` blocks off the eye along this frame.
    pub fn at(&self, right: f32, up: f32, ahead: f32) -> [f64; 3] {
        std::array::from_fn(|i| {
            self.eye[i]
                + f64::from(-self.across[i] * right + self.up[i] * up + self.forward[i] * ahead)
        })
    }
}
