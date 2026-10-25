use petramond_math::math::Vec3;
use petramond_math::world_pos::WorldPos;

use super::ServerGame;

impl ServerGame {
    /// Pins the local player at `pos`, whole and breathing, so a tool can stand it anywhere
    /// (under water, in lava) while the world runs around it.
    pub(crate) fn hold_local_player(&mut self, pos: WorldPos) {
        if self.sessions.local_id().is_none() {
            return;
        }
        let sess = &mut self.sessions[0];
        sess.player.pos = pos;
        sess.player.vel = Vec3::ZERO;
        sess.input.claim_pos = pos;
        sess.player.clear_exposure();
        sess.player.set_health(crate::player::MAX_HEALTH);
    }
}
