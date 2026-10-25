//! First-person overlays of worn items (a diving helmet's faceplate): one atlas row per item
//! whose wear row names an overlay. A row is [`SPAN`] overlay widths wide, the overlay in the
//! middle and its edge columns carried out to both sides, so a screen up to that many times
//! wider than tall shows the overlay at full height with its sides continued.

use image::RgbaImage;
use petramond_world::item::ItemType;

pub const CELL_PX: u32 = 128;
pub const SPAN: u32 = 3;

fn overlay_items() -> impl Iterator<Item = (ItemType, &'static str)> {
    ItemType::all()
        .iter()
        .filter_map(|&item| Some((item, item.wear()?.overlay?)))
}

/// The atlas row holding `item`'s overlay, and the row count.
pub fn row_of(item: ItemType) -> Option<(usize, usize)> {
    let mut rows = overlay_items().map(|(i, _)| i);
    let row = rows.position(|i| i == item)?;
    Some((row, row + 1 + rows.count()))
}

pub fn compose_atlas() -> Option<RgbaImage> {
    let items: Vec<_> = overlay_items().collect();
    if items.is_empty() {
        return None;
    }
    let side = (SPAN - 1) / 2 * CELL_PX;
    let mut atlas = RgbaImage::new(CELL_PX * SPAN, CELL_PX * items.len() as u32);
    for (row, (item, path)) in items.iter().enumerate() {
        let Some(img) = load_rgba(path) else {
            log::warn!(
                "item '{}': wear overlay '{path}' missing or unreadable",
                item.key()
            );
            continue;
        };
        let img =
            image::imageops::resize(&img, CELL_PX, CELL_PX, image::imageops::FilterType::Nearest);
        let y0 = row as u32 * CELL_PX;
        for y in 0..CELL_PX {
            for x in 0..atlas.width() {
                let src = x.saturating_sub(side).min(CELL_PX - 1);
                atlas.put_pixel(x, y0 + y, *img.get_pixel(src, y));
            }
        }
    }
    Some(atlas)
}

fn load_rgba(rel: &str) -> Option<RgbaImage> {
    let (bytes, _path) = petramond_world::assets::read_bytes(rel)?;
    Some(image::load_from_memory(&bytes).ok()?.to_rgba8())
}
