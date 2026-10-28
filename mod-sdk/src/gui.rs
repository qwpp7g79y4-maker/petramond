use mod_api::{ContainerAddress, GuiAudience, GuiValue, GuiViewerData, PlayerId};

use crate::__rt::host_fn;

host_fn! {
    /// Writes a key in the GUI state map of a player (a [`PlayerId`]), or of every player with a
    /// GUI open anchored at a [`ContainerAddress`]. Labels bound to the key redraw. `rotimage`
    /// reads angle in radians from an `F32`. A framed image/button reads its frame from an `I32`
    /// at `bind.frame`. Keys are mod-local to one session, cleared on open/close. `false` = it
    /// reached nobody.
    ///
    /// A machine publishes its gauges to its anchor's viewers when it steps and when its block
    /// hears [`BlockHookKind::ViewersChanged`](mod_api::BlockHookKind::ViewersChanged).
    pub fn gui_state_set(to: impl Into<GuiAudience>, key: &str, value: GuiValue) -> bool
        => GuiStateSet { to: to.into(), key: key.into(), value } => Bool
}

host_fn! {
    pub fn gui_viewers() -> Vec<GuiViewerData> => GuiViewers => GuiViewers
}

host_fn! {
    pub fn gui_state_get(player: PlayerId, key: &str) -> Option<GuiValue>
        => GuiStateGet { player, key: key.into() } => GuiValue
}

host_fn! {
    pub fn gui_open(
        player: PlayerId,
        kind_key: &str,
        at: Option<impl Into<ContainerAddress>>,
    ) -> bool
        => GuiOpen { player, kind_key: kind_key.into(), at: at.map(Into::into) } => Bool
}

host_fn! {
    pub fn gui_close(player: PlayerId) -> bool => GuiClose { player } => Bool
}
