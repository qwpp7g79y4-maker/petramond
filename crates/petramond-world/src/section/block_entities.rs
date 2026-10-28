use crate::chunk::section_idx;
use crate::container::Container;

use super::{BlockEntities, CellMap, Section};

impl Section {
    #[inline]
    fn block_entity_key(x: usize, y: usize, z: usize) -> u16 {
        section_idx(x, y, z) as u16
    }

    #[inline]
    fn entities_mut(&mut self) -> &mut BlockEntities {
        self.entities.get_or_insert_default()
    }

    #[inline]
    pub fn container_at(&self, x: usize, y: usize, z: usize) -> Option<&Container> {
        self.entities
            .as_deref()
            .and_then(|e| e.containers.get(&Self::block_entity_key(x, y, z)))
    }

    #[inline]
    pub fn container_at_mut(&mut self, x: usize, y: usize, z: usize) -> Option<&mut Container> {
        self.entities
            .as_deref_mut()
            .and_then(|e| e.containers.get_mut(&Self::block_entity_key(x, y, z)))
    }

    pub fn insert_container(&mut self, x: usize, y: usize, z: usize, c: Container) {
        self.entities_mut()
            .containers
            .insert(Self::block_entity_key(x, y, z), c);
        self.modified = true;
    }

    pub fn take_container(&mut self, x: usize, y: usize, z: usize) -> Option<Container> {
        let removed = self
            .entities
            .as_deref_mut()
            .and_then(|e| e.containers.remove(&Self::block_entity_key(x, y, z)));
        if removed.is_some() {
            self.modified = true;
        }
        removed
    }

    #[inline]
    pub fn containers(&self) -> &CellMap<Container> {
        match &self.entities {
            Some(e) => &e.containers,
            None => crate::block_state::empty_map!(Container),
        }
    }
}
