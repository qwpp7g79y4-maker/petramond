/// Row data `petramond:pours`: used on the world, the item puts down `fluid` and becomes
/// `becomes`. Its sibling `petramond:scoops` lists the fluids it takes up, each with the item it
/// turns into.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Pour {
    pub fluid: crate::block::Block,
    pub becomes: super::ItemType,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum UseRay {
    #[default]
    Solid,
    Fluids(&'static [crate::block::Block]),
}

impl UseRay {
    pub fn stops_at(self, fluid: crate::block::Block) -> bool {
        match self {
            UseRay::Solid => false,
            UseRay::Fluids(fluids) => fluids.contains(&fluid),
        }
    }

    pub fn sees_fluid(self) -> bool {
        matches!(self, UseRay::Fluids(fluids) if !fluids.is_empty())
    }
}
