//! Which mob mans which kind of post: a mob row whose `monsters:post_roles`
//! data lists post role names (`["yard", "watch"]`, `["crew"]`) is sent to
//! fill those posts, and becomes one of the garrisoned kinds the skeleton
//! systems drive. The first row naming a role keeps it.

use mod_sdk::*;
use post_marker::PostRole;

use super::keys::POST_ROLES_DATA;

/// The garrisoned kinds, and the kind (and its key) each post role sends, by role code.
pub fn load() -> (Vec<MobId>, FxHashMap<u8, (MobId, String)>) {
    let rows = mobs_with_data_as::<Vec<String>>(POST_ROLES_DATA);
    let names = mob_names(rows.iter().map(|(kind, _)| *kind).collect());
    let mut kinds = Vec::new();
    let mut guards = FxHashMap::default();
    for ((kind, roles), name) in rows.into_iter().zip(names) {
        let Some(name) = name else {
            continue;
        };
        for role in &roles {
            match PostRole::from_key(role) {
                Some(role) => {
                    guards
                        .entry(role.code())
                        .or_insert_with(|| (kind, name.clone()));
                }
                None => log(&format!(
                    "skeletons: mob '{name}' names unknown post role '{role}'"
                )),
            }
        }
        kinds.push(kind);
    }
    (kinds, guards)
}
