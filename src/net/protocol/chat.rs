use serde::{Deserialize, Serialize};

pub const MAX_CHAT_CHARS: usize = 256;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatColor {
    White,
    Red,
    Yellow,
    Blue,
    Cyan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatSpan {
    pub fg: ChatColor,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatLine {
    pub seq: u64,
    pub spans: Vec<ChatSpan>,
}

/// One word of a command's grammar and the words that may follow it. The server sends each
/// player the commands they may run; the client completes chat input against it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandNode {
    pub word: CommandWord,
    pub next: Vec<CommandNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandWord {
    Literal(String),
    /// An argument; `choices` are the values worth suggesting (empty for free text).
    Arg {
        name: String,
        choices: Vec<String>,
    },
}

impl CommandNode {
    pub fn literal(word: &str, next: Vec<CommandNode>) -> Self {
        Self {
            word: CommandWord::Literal(word.to_owned()),
            next,
        }
    }

    pub fn arg(name: &str, choices: Vec<String>, next: Vec<CommandNode>) -> Self {
        Self {
            word: CommandWord::Arg {
                name: name.to_owned(),
                choices,
            },
            next,
        }
    }
}
