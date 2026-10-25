use mod_api::{BlockId, ItemId, MobId};
use serde::de::DeserializeOwned;

use crate::{block_names, blocks_with_data, item_names, items_with_data, log};
use crate::{mob_names, mobs_with_data};

pub fn parse_row_data<T: DeserializeOwned>(value: &str) -> Result<T, String> {
    serde_json::from_str(value).map_err(|error| error.to_string())
}

pub fn items_with_data_as<T: DeserializeOwned>(key: &str) -> Vec<(ItemId, T)> {
    typed_rows(key, items_with_data(key), |ids| item_names(ids.to_vec()))
}

pub fn blocks_with_data_as<T: DeserializeOwned>(key: &str) -> Vec<(BlockId, T)> {
    typed_rows(key, blocks_with_data(key), |ids| block_names(ids.to_vec()))
}

pub fn mobs_with_data_as<T: DeserializeOwned>(key: &str) -> Vec<(MobId, T)> {
    typed_rows(key, mobs_with_data(key), |ids| mob_names(ids.to_vec()))
}

fn typed_rows<Id: Copy + std::fmt::Debug, T: DeserializeOwned>(
    key: &str,
    rows: Vec<(Id, String)>,
    names: impl FnOnce(&[Id]) -> Vec<Option<String>>,
) -> Vec<(Id, T)> {
    let (parsed, rejected) = split_rows(rows);
    if !rejected.is_empty() {
        let ids: Vec<Id> = rejected.iter().map(|(id, _)| *id).collect();
        let names = names(&ids);
        for (index, (id, reason)) in rejected.iter().enumerate() {
            let row = names
                .get(index)
                .cloned()
                .flatten()
                .unwrap_or_else(|| format!("{id:?}"));
            log(&row_error(key, &row, reason));
        }
    }
    parsed
}

type SplitRows<Id, T> = (Vec<(Id, T)>, Vec<(Id, String)>);

fn split_rows<Id, T: DeserializeOwned>(rows: Vec<(Id, String)>) -> SplitRows<Id, T> {
    let mut parsed = Vec::with_capacity(rows.len());
    let mut rejected = Vec::new();
    for (id, raw) in rows {
        match parse_row_data::<T>(&raw) {
            Ok(value) => parsed.push((id, value)),
            Err(reason) => rejected.push((id, reason)),
        }
    }
    (parsed, rejected)
}

pub fn pack_rows_with_data(document: &str, table: &str, key: &str) -> Vec<(String, String)> {
    let Ok(document) = serde_json::from_str::<serde_json::Value>(document) else {
        return Vec::new();
    };
    let name_field = table.strip_suffix('s').unwrap_or(table);
    document
        .get(table)
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let entry = row.get("data")?.get(key)?;
            let name = [name_field, "patch"]
                .iter()
                .find_map(|field| row.get(*field).and_then(serde_json::Value::as_str))
                .unwrap_or("?");
            Some((name.to_owned(), entry.to_string()))
        })
        .collect()
}

pub fn row_error(key: &str, row: &str, reason: &str) -> String {
    format!("row data '{key}' on '{row}' is ignored: {reason}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pack_documents_rows_are_listed_with_their_names() {
        let document = r#"{"items": [
            {"item": "a:bucket", "data": {"x:vessel": {"filled": "a:full"}}},
            {"item": "a:stick"},
            {"item": "a:jar", "data": {"x:vessel": {"filled": "a:jam"}, "y:other": 1}},
            {"patch": "b:pot", "data": {"x:vessel": {"filled": "b:stew"}}}
        ]}"#;
        let rows = pack_rows_with_data(document, "items", "x:vessel");
        assert_eq!(
            rows,
            vec![
                ("a:bucket".to_owned(), r#"{"filled":"a:full"}"#.to_owned()),
                ("a:jar".to_owned(), r#"{"filled":"a:jam"}"#.to_owned()),
                ("b:pot".to_owned(), r#"{"filled":"b:stew"}"#.to_owned()),
            ]
        );
        assert!(pack_rows_with_data("not json", "items", "x:vessel").is_empty());
    }
}
