use std::sync::Arc;

use petramond_world::gui_state::GuiStateMap;

use crate::player::{Player, PlayerId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OpenGui {
    pub kind: petramond_world::gui_state::GuiKind,
    pub anchor: Option<crate::menu::MenuAnchor>,
}

pub trait PlayerRoster {
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn id_at(&self, index: usize) -> PlayerId;

    fn player_at(&mut self, index: usize) -> &mut Player;

    fn gui_state_at(&mut self, index: usize) -> &mut Arc<GuiStateMap>;

    fn open_gui_at(&self, index: usize) -> Option<OpenGui>;

    fn index_of(&self, id: PlayerId) -> Option<usize> {
        (0..self.len()).find(|&i| self.id_at(i) == id)
    }
}

pub struct SessionPlayerRef<'a> {
    pub id: PlayerId,
    pub player: &'a mut Player,
    pub gui_state: &'a mut Arc<GuiStateMap>,
    pub gui: Option<OpenGui>,
}

#[derive(Default)]
pub struct RosterRefs<'a> {
    sessions: Vec<SessionPlayerRef<'a>>,
}

impl<'a> RosterRefs<'a> {
    pub fn new(sessions: Vec<SessionPlayerRef<'a>>) -> Self {
        Self { sessions }
    }

    pub fn empty() -> Self {
        Self::default()
    }
}

impl PlayerRoster for RosterRefs<'_> {
    fn len(&self) -> usize {
        self.sessions.len()
    }

    fn id_at(&self, index: usize) -> PlayerId {
        self.sessions[index].id
    }

    fn player_at(&mut self, index: usize) -> &mut Player {
        &mut *self.sessions[index].player
    }

    fn gui_state_at(&mut self, index: usize) -> &mut Arc<GuiStateMap> {
        &mut *self.sessions[index].gui_state
    }

    fn open_gui_at(&self, index: usize) -> Option<OpenGui> {
        self.sessions[index].gui
    }
}
