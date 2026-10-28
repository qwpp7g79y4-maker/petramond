use crate::data::PlayerAttribute;
use crate::ids::{ItemId, PlayerId};
use crate::legality::prelude::*;

host_domain! {
    PlayerCall {
        ChatSend {
            text: String,
            targets: Option<Vec<PlayerId>>,
        } => legal(SERVER, Sim, Write),
        Players => legal(SERVER, Sim, Read),
        UnlockRecipe {
            player: PlayerId,
            recipe: String,
        } => legal(SERVER, Sim, Write),
        RecipeUnlocked {
            player: PlayerId,
            recipe: String,
        } => legal(SERVER, Sim, Read),
        PlayerHeld {
            player: PlayerId,
        } => legal(SERVER, Sim, Read),
        GiveItem {
            player: PlayerId,
            item: String,
            count: u8,
            data: Vec<(String, Vec<u8>)>,
        } => legal(SERVER, Sim, Write),
        SetPlayerHeldData {
            player: PlayerId,
            expect_item: String,
            expect_data: Vec<(String, Vec<u8>)>,
            data: Vec<(String, Vec<u8>)>,
        } => legal(SERVER, Sim, Write),
        SetPlayerAttribute {
            player: PlayerId,
            attribute: PlayerAttribute,
            scale: f32,
        } => legal(SERVER, Sim, Write),
        /// Bar a set of [`BodyAction`](crate::BodyAction)s on one body — the claim
        /// for "these hands are busy". An empty list releases it.
        /// → [`HostRet::Bool`](crate::HostRet::Bool) (`false` = no such reachable session).
        ///
        /// The sibling of [`SetPlayerAttribute`](Self::SetPlayerAttribute) but
        /// resolved by UNION, not product: two claimants barring different things
        /// both mean it, and one able to un-bar another's would make "may this
        /// body mine" depend on claimant order. TRANSIENT, and MIRRORED to the
        /// barred player's client so their own prediction stops with it — a client
        /// still predicting a break the server will refuse shows a crack creeping
        /// up a block that never breaks.
        ///
        /// Cancelling at [`EventKind::BlockBreakPre`](crate::EventKind::BlockBreakPre) is not a substitute: it
        /// refuses the break only at the END of the timer, after a second of crack
        /// animation, and a swing at the air has no pre-event at all. Barring the
        /// ACTION is the honest shape — the button simply does nothing, on both
        /// sides, for as long as the claim stands.
        ///
        /// [`EventKind::BlockBreakPre`]: crate::EventKind::BlockBreakPre
        SetPlayerDeniedActions {
            player: PlayerId,
            actions: Vec<crate::BodyAction>,
        } => legal(SERVER, Sim, Write),
        TakeItem {
            player: PlayerId,
            item: String,
            count: u8,
            data: Option<Vec<(String, Vec<u8>)>>,
        } => legal(SERVER, Sim, Write),
        PlayerIdentity {
            player: PlayerId,
        } => legal(SERVER, Sim, Read),
        SetHealth {
            player: PlayerId,
            value: i32,
        } => legal(SERVER, Sim, Write),
        Teleport {
            player: PlayerId,
            pos: [f64; 3],
        } => legal(SERVER, Sim, Write),
        EffectApply {
            player: PlayerId,
            key: String,
            ticks: u32,
        } => legal(SERVER, Sim, Write),
        EffectsActive {
            player: PlayerId,
        } => legal(SERVER, Sim, Read),
        ConsumeHeld {
            player: PlayerId,
            item: ItemId,
            count: u32,
        } => legal(SERVER, Sim, Write),
        ReplaceHeldOne {
            player: PlayerId,
            item: ItemId,
            replacement: String,
        } => legal(SERVER, Sim, Write),
        PlayerInputs {
            player_ids: Vec<PlayerId>,
        } => legal(SERVER, Sim, Read),
    }
}
