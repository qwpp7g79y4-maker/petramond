use crate::world::{World, WorldSide};
use petramond_math::math::IVec3;
use petramond_world::block::Block;
use petramond_world::fluid::held;

impl<S: WorldSide> World<S> {
    pub fn cell_kv_set(&mut self, wx: i32, wy: i32, wz: i32, key: String, value: Vec<u8>) -> bool {
        if key == held::KV_KEY {
            let block = self.data.physics_block(wx, wy, wz);
            return held::decode(block, &value)
                .is_some_and(|fluid| self.set_held_fluid(IVec3::new(wx, wy, wz), Some(fluid)));
        }
        if !self.data.cell_kv_writable(wx, wy, wz) {
            return false;
        }
        let pos = petramond_math::math::IVec3::new(wx, wy, wz);
        let capture = self
            .side
            .server()
            .is_some_and(|s| s.replication.wants_cell_kv_delta(pos));
        let log_value = capture.then(|| value.clone());
        let Some((s, lx, ly, lz)) = self.data.chunk_at_world_mut(wx, wy, wz) else {
            return false;
        };
        s.cell_kv_set(lx, ly, lz, key.clone(), value);
        s.modified = true;
        if let (Some(v), Some(server)) = (log_value, self.side.server_mut()) {
            server
                .replication
                .cell_kv_delta_log
                .insert((pos, key.clone()), Some(v));
        }
        if petramond_world::block::kv_key_affects_mesh(&key) {
            self.queue_dirty_meshes_sampling_cell(wx, wy, wz);
        }
        self.data.remark_state_key_bakes(wx, wy, wz, &key);
        true
    }
    pub fn cell_kv_remove(&mut self, wx: i32, wy: i32, wz: i32, key: &str) -> bool {
        if key == held::KV_KEY {
            return self.set_held_fluid(IVec3::new(wx, wy, wz), None);
        }
        if !self.data.cell_kv_writable(wx, wy, wz) {
            return false;
        }
        let Some((s, lx, ly, lz)) = self.data.chunk_at_world_mut(wx, wy, wz) else {
            return false;
        };
        let removed = s.cell_kv_remove(lx, ly, lz, key);
        if removed {
            s.modified = true;
            let pos = petramond_math::math::IVec3::new(wx, wy, wz);
            if let Some(server) = self.side.server_mut() {
                if server.replication.wants_cell_kv_delta(pos) {
                    server
                        .replication
                        .cell_kv_delta_log
                        .insert((pos, key.to_string()), None);
                }
            }
            if petramond_world::block::kv_key_affects_mesh(key) {
                self.queue_dirty_meshes_sampling_cell(wx, wy, wz);
            }
            self.data.remark_state_key_bakes(wx, wy, wz, key);
        }
        removed
    }

    /// Fills the block at `pos` with `fluid`, or empties it. False when the block cannot hold
    /// that fluid or already does. The cell's light stays as it was; flow, nav and meshes see
    /// the change as they would a block edit.
    pub fn set_held_fluid(&mut self, pos: IVec3, fluid: Option<Block>) -> bool {
        let IVec3 { x, y, z } = pos;
        if !self.data.cell_kv_writable(x, y, z) {
            return false;
        }
        let Some((s, lx, ly, lz)) = self.data.chunk_at_world_mut(x, y, z) else {
            return false;
        };
        if !s.set_held_fluid(lx, ly, lz, fluid) {
            return false;
        }
        self.queue_dirty_meshes_sampling_cell(x, y, z);
        self.queue_dirty_meshes_walking_fluid(pos);
        self.notify_light_equivalent_change_nav(x, y, z, true);
        true
    }
}
