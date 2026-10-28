use crate::data::{ContainerAddress, GuiAudience, GuiValue};
use crate::ids::PlayerId;
use crate::legality::prelude::*;

host_domain! {
    GuiCall {
        /// Write one key of the GUI state map of each session `to` reaches. `false` = it reached
        /// none (no such connected session, or nobody viewing there).
        GuiStateSet {
            to: GuiAudience,
            key: String,
            value: GuiValue,
        } => legal(SERVER, Sim, Write),
        GuiStateGet {
            player: PlayerId,
            key: String,
        } => legal(SERVER, Sim, Read),
        GuiOpen {
            player: PlayerId,
            kind_key: String,
            at: Option<ContainerAddress>,
        } => legal(SERVER, Sim, Write),
        GuiClose {
            player: PlayerId,
        } => legal(SERVER, Sim, Write),
        GuiViewers => legal(SERVER, Sim, Read),
    }
}
