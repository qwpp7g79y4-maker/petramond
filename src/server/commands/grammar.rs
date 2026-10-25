use crate::net::protocol::{CommandNode, CommandWord};

/// Every command a player may run, by operator status. Non-operators run none.
pub(crate) fn player_commands(operator: bool) -> Vec<CommandNode> {
    if !operator {
        return Vec::new();
    }
    let time_presets = ["day", "noon", "night", "midnight"]
        .iter()
        .map(|p| CommandNode::literal(p, Vec::new()))
        .collect();
    let mut mobs: Vec<String> = crate::mob::Mob::all()
        .iter()
        .map(|&m| crate::mob::def(m).key.to_owned())
        .collect();
    mobs.sort_unstable();
    vec![
        CommandNode::literal(
            "deop",
            vec![CommandNode::arg("player", Vec::new(), Vec::new())],
        ),
        CommandNode::literal(
            "op",
            vec![CommandNode::arg("player", Vec::new(), Vec::new())],
        ),
        CommandNode::literal(
            "say",
            vec![CommandNode::arg("message", Vec::new(), Vec::new())],
        ),
        CommandNode::literal("summon", vec![CommandNode::arg("mob", mobs, Vec::new())]),
        CommandNode::literal(
            "time",
            vec![
                CommandNode::literal("freeze", Vec::new()),
                CommandNode::literal("set", time_presets),
                CommandNode::literal("unfreeze", Vec::new()),
            ],
        ),
    ]
}

pub(super) fn command_names() -> Vec<String> {
    player_commands(true)
        .into_iter()
        .filter_map(|node| match node.word {
            CommandWord::Literal(name) => Some(name),
            CommandWord::Arg { .. } => None,
        })
        .collect()
}
