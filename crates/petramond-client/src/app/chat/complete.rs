use petramond::net::protocol::{CommandNode, CommandWord};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Completion {
    /// The whole input line once this completion is taken.
    pub line: String,
    /// The word offered, as listed.
    pub word: String,
    /// Characters of the line before the completed word.
    pub word_start: usize,
}

/// Completions for the last word of a `/command` line. A word typed in full that leads on
/// offers what may follow it instead (`/time set` lists the presets).
pub(super) fn complete(commands: &[CommandNode], line: &str) -> Vec<Completion> {
    let Some(body) = line.strip_prefix('/') else {
        return Vec::new();
    };
    let words: Vec<&str> = body.split(' ').collect();
    let (partial, typed) = words.split_last().expect("split yields at least one word");
    let mut level = commands;
    for word in typed {
        let Some(node) = level.iter().find(|node| accepts(node, word)) else {
            return Vec::new();
        };
        level = &node.next;
    }
    let found = offered(level, partial);
    if let [only] = found.as_slice() {
        if only == partial && !partial.is_empty() {
            let Some(node) = level.iter().find(|node| accepts(node, partial)) else {
                return Vec::new();
            };
            return completions(&format!("{line} "), offered(&node.next, ""));
        }
    }
    completions(&line[..line.len() - partial.len()], found)
}

fn completions(prefix: &str, words: Vec<String>) -> Vec<Completion> {
    let word_start = prefix.chars().count();
    words
        .into_iter()
        .map(|word| Completion {
            line: format!("{prefix}{word}"),
            word,
            word_start,
        })
        .collect()
}

fn accepts(node: &CommandNode, word: &str) -> bool {
    match &node.word {
        CommandWord::Literal(literal) => literal == word,
        CommandWord::Arg { .. } => !word.is_empty(),
    }
}

/// Words at this level that start with `partial`; namespaced choices also match on the name
/// after the colon, so `sha` finds `exploration:shark`.
fn offered(level: &[CommandNode], partial: &str) -> Vec<String> {
    let matches = |word: &str| {
        word.starts_with(partial)
            || word
                .split_once(':')
                .is_some_and(|(_, name)| name.starts_with(partial))
    };
    let mut out = Vec::new();
    for node in level {
        match &node.word {
            CommandWord::Literal(literal) if matches(literal) => out.push(literal.clone()),
            CommandWord::Literal(_) => {}
            CommandWord::Arg { choices, .. } => {
                out.extend(choices.iter().filter(|c| matches(c)).cloned());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> Vec<CommandNode> {
        let leaf = |w: &str| CommandNode::literal(w, Vec::new());
        vec![
            CommandNode::literal(
                "summon",
                vec![CommandNode::arg(
                    "mob",
                    vec![
                        "exploration:shark".into(),
                        "monsters:skeleton".into(),
                        "monsters:zombie".into(),
                    ],
                    Vec::new(),
                )],
            ),
            CommandNode::literal(
                "time",
                vec![
                    leaf("freeze"),
                    CommandNode::literal("set", vec![leaf("day"), leaf("night"), leaf("noon")]),
                    leaf("unfreeze"),
                ],
            ),
        ]
    }

    fn words(line: &str) -> Vec<String> {
        complete(&tree(), line)
            .into_iter()
            .map(|c| c.word)
            .collect()
    }

    #[test]
    fn a_partial_word_narrows_the_list_and_completes_in_place() {
        assert_eq!(words("/time s"), ["set"]);
        assert_eq!(complete(&tree(), "/time s")[0].line, "/time set");
    }

    #[test]
    fn a_word_typed_in_full_offers_the_next_words() {
        assert_eq!(words("/time set"), ["day", "night", "noon"]);
        assert_eq!(complete(&tree(), "/time set")[0].line, "/time set day");
        assert_eq!(words("/summon").len(), 3);
        assert!(words("/time freeze").is_empty());
    }

    #[test]
    fn namespaced_choices_match_on_their_name_too() {
        assert_eq!(
            words("/summon monsters:"),
            ["monsters:skeleton", "monsters:zombie"]
        );
        assert_eq!(words("/summon sha"), ["exploration:shark"]);
        assert_eq!(
            complete(&tree(), "/summon sha")[0].line,
            "/summon exploration:shark"
        );
    }

    #[test]
    fn plain_chat_and_unknown_commands_offer_nothing() {
        assert!(words("hello").is_empty());
        assert!(words("/nope ").is_empty());
        assert!(words("/summon exploration:shark ").is_empty());
    }
}
