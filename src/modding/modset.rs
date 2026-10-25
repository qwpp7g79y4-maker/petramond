use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModSetEntry {
    pub id: String,
    #[serde(default)]
    pub version: String,
    #[serde(default = "affects_world_unknown")]
    pub affects_world: bool,
}

fn affects_world_unknown() -> bool {
    true
}

#[derive(Serialize, Deserialize, Default)]
struct ModsFile {
    mods: Vec<ModSetEntry>,
}

pub fn active(disabled: &BTreeSet<String>) -> Vec<ModSetEntry> {
    let mut mods: Vec<ModSetEntry> = petramond_world::assets::packs()
        .iter()
        .filter_map(|p| {
            let id = p.id.clone()?;
            if disabled.contains(&id) {
                return None;
            }
            Some(ModSetEntry {
                id,
                version: p.version.clone().unwrap_or_default(),
                affects_world: p.touches_world,
            })
        })
        .collect();
    mods.sort_by(|a, b| a.id.cmp(&b.id));
    mods
}

pub fn encode_active(disabled: &BTreeSet<String>) -> Vec<u8> {
    encode(active(disabled))
}

fn encode(mods: Vec<ModSetEntry>) -> Vec<u8> {
    serde_json::to_vec_pretty(&ModsFile { mods }).unwrap_or_default()
}

pub fn check_at_open(save_dir: &Path, disabled: &BTreeSet<String>) -> Vec<ModSetEntry> {
    let Some(recorded) = recorded_enabled(save_dir, disabled) else {
        return Vec::new();
    };
    for line in diff(&recorded, &active(disabled)) {
        log::warn!("{line}");
    }
    absent(recorded, &active(&BTreeSet::new()))
}

pub fn recorded_ids(save_dir: &Path) -> Option<BTreeSet<String>> {
    let bytes = std::fs::read(save_dir.join("mods.json")).ok()?;
    let file = serde_json::from_slice::<ModsFile>(&bytes).ok()?;
    Some(file.mods.into_iter().map(|m| m.id).collect())
}

fn recorded_enabled(save_dir: &Path, disabled: &BTreeSet<String>) -> Option<Vec<ModSetEntry>> {
    let bytes = std::fs::read(save_dir.join("mods.json")).ok()?;
    let recorded = match serde_json::from_slice::<ModsFile>(&bytes) {
        Ok(f) => f.mods,
        Err(e) => {
            log::warn!("save mods.json is unreadable ({e}); mod-set check skipped");
            return None;
        }
    };
    Some(
        recorded
            .into_iter()
            .filter(|r| !disabled.contains(&r.id))
            .collect(),
    )
}

pub fn missing(save_dir: &Path, disabled: &BTreeSet<String>) -> Vec<ModSetEntry> {
    match recorded_enabled(save_dir, disabled) {
        Some(recorded) => absent(recorded, &active(&BTreeSet::new())),
        None => Vec::new(),
    }
}

fn absent(recorded: Vec<ModSetEntry>, installed: &[ModSetEntry]) -> Vec<ModSetEntry> {
    recorded
        .into_iter()
        .filter(|r| !installed.iter().any(|a| a.id == r.id))
        .collect()
}

pub fn diff(recorded: &[ModSetEntry], active: &[ModSetEntry]) -> Vec<String> {
    let mut lines = Vec::new();
    for r in recorded {
        match active.iter().find(|a| a.id == r.id) {
            None => lines.push(format!(
                "mod '{}' (v{}) was active when this world was last saved but is MISSING now; \
                 its blocks, items and mobs are kept but not placed, usable or spawned until \
                 it returns",
                r.id, r.version
            )),
            Some(a) if a.version != r.version => lines.push(format!(
                "mod '{}' changed version since this world was last saved: v{} -> v{}",
                r.id, r.version, a.version
            )),
            Some(_) => {}
        }
    }
    for a in active {
        if !recorded.iter().any(|r| r.id == a.id) {
            lines.push(format!(
                "mod '{}' (v{}) is newly active for this world (not in its last-saved mod set)",
                a.id, a.version
            ));
        }
    }
    lines
}
