use crate::data::EntityRef;
use crate::ids::PlayerId;
use crate::legality::prelude::*;

host_domain! {
    BodyCall {
        /// One player's snapshot; `None` = no such reachable session. A client instance
        /// answers only for the local player, during prediction dispatches.
        PlayerState {
            player: PlayerId,
        } => legal(SERVER_CLIENT, Sim, Read),
        SetPlayerHeldPose {
            player: PlayerId,
            main: Option<crate::HeldPose>,
            off: Option<crate::HeldPose>,
        } => legal(SERVER_CLIENT, Sim, Write),
        SetPlayerBonePose {
            player: PlayerId,
            bones: Vec<crate::BonePoseData>,
        } => legal(SERVER_CLIENT, Sim, Write),
        HoldUse {
            player: PlayerId,
        } => legal(SERVER_CLIENT, Sim, Write),
        /// Draws `main`/`off` in a body's hands in place of what it holds (`None` = the real
        /// item). A player's display is per mod and predicted by its own client; a client
        /// instance may set only its local player's.
        SetHeldDisplay {
            body: EntityRef,
            main: Option<String>,
            off: Option<String>,
        } => legal(SERVER_CLIENT, Sim, Write),
        PlayerInventory {
            player: PlayerId,
        } => legal(SERVER_CLIENT, Sim, Read),
        SetPlayerAnimatorParams {
            player: PlayerId,
            params: Vec<crate::AnimatorParam>,
        } => legal(SERVER_CLIENT, Sim, Write),
        SetPlayerAnimatorPlays {
            player: PlayerId,
            plays: Vec<crate::AnimatorPlay>,
        } => legal(SERVER_CLIENT, Sim, Write),
        FirePlayerAnimatorEvent {
            player: PlayerId,
            rig: String,
            event: String,
        } => legal(SERVER_CLIENT, Sim, Write),
        AnimationClip {
            rig: String,
            clip: String,
        } => legal(Sides::SERVER_CLIENT.union(Sides::SHELL), Any, Read),
        ActingPlayer => legal(SERVER_CLIENT, Sim, Read),
    }
}
