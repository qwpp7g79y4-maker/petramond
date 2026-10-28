use serde::Deserialize;

use crate::sound_registry::Sound;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlockSoundAction {
    Dig,
    Break,
    Place,
    Step,
    Open,
    Close,
}

/// A block's sound per action: its material's set, with the row's own entries laid over it.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockSoundSet {
    pub dig: Option<Sound>,
    pub break_: Option<Sound>,
    pub place: Option<Sound>,
    pub step: Option<Sound>,
    pub open: Option<Sound>,
    pub close: Option<Sound>,
}

impl BlockSoundSet {
    #[inline]
    pub fn get(&self, action: BlockSoundAction) -> Option<Sound> {
        match action {
            BlockSoundAction::Dig => self.dig,
            BlockSoundAction::Break => self.break_,
            BlockSoundAction::Place => self.place,
            BlockSoundAction::Step => self.step,
            BlockSoundAction::Open => self.open,
            BlockSoundAction::Close => self.close,
        }
    }

    /// `self` with every action `over` names replaced.
    pub fn overlaid(self, over: BlockSoundSet) -> BlockSoundSet {
        BlockSoundSet {
            dig: over.dig.or(self.dig),
            break_: over.break_.or(self.break_),
            place: over.place.or(self.place),
            step: over.step.or(self.step),
            open: over.open.or(self.open),
            close: over.close.or(self.close),
        }
    }
}

/// `sounds` on a material or block row: sound names by action.
#[derive(Clone, Debug, Default, Deserialize, serde::Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawSoundSet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dig: Option<String>,
    #[serde(default, rename = "break", skip_serializing_if = "Option::is_none")]
    break_: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    place: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    step: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    open: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    close: Option<String>,
}

impl RawSoundSet {
    pub fn is_empty(&self) -> bool {
        *self == RawSoundSet::default()
    }

    pub fn resolve(&self) -> Result<BlockSoundSet, String> {
        let sound = |name: &Option<String>| -> Result<Option<Sound>, String> {
            name.as_deref()
                .map(|n| {
                    crate::sound_registry::by_name(n).ok_or_else(|| format!("unknown sound '{n}'"))
                })
                .transpose()
        };
        Ok(BlockSoundSet {
            dig: sound(&self.dig)?,
            break_: sound(&self.break_)?,
            place: sound(&self.place)?,
            step: sound(&self.step)?,
            open: sound(&self.open)?,
            close: sound(&self.close)?,
        })
    }
}
