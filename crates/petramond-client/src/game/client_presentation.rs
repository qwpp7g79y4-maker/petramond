use super::Game;

impl Game {
    pub(super) fn sync_sleep_camera_on_open(
        &mut self,
        self_events: &petramond::net::protocol::SelfEvents,
    ) {
        if self_events.open_screen != Some(petramond::net::protocol::OpenScreen::Sleep) {
            return;
        }
        self.local.cam.yaw = self.local.player.yaw;
        self.local.cam.pitch = self.local.player.pitch;
        self.sync_camera_to_player_eye(0.0);
    }

    #[inline]
    pub(super) fn tick_alpha(&self) -> f32 {
        self.replica.entities.alpha()
    }

    /// What the local player wears, by slot.
    pub fn worn_items(
        &self,
    ) -> [Option<petramond_world::item::ItemType>; petramond_world::inventory::WEAR_SLOTS] {
        petramond_world::inventory::WearSlot::ALL.map(|slot| {
            self.replica
                .self_view
                .inventory
                .worn(slot)
                .map(|stack| stack.item)
        })
    }

    pub(super) fn held_item_light(&self) -> (u8, petramond_world::light::BlockLight6) {
        let c = self.local.cam.pos.block();
        self.replica
            .world
            .data()
            .dynamic_light_at_world(c.x, c.y, c.z)
    }

    pub(super) fn tick_mesh_budget(&mut self) {
        const MESH_BUDGET: usize = 256;
        self.replica.world.tick_mesh_budget(MESH_BUDGET);
    }
}
