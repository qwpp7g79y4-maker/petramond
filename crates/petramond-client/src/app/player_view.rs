//! The inventory's player figure: the local player standing still and facing the viewer, the
//! head (and a little of the body) turned toward the cursor, drawn by the renderer into the
//! document image whose node id and image bind are `player_view`.

use petramond::gui::PLAYER_VIEW_IMAGE;
use petramond_render::PlayerView;
use petramond_ui::UiValue;
use petramond_world::gui_state::GuiKind;

use super::App;

pub(super) const PLAYER_VIEW_ID: &str = "player_view";
/// The cursor one figure-height beside or above the eyes turns the look 45°.
const LOOK_REACH: f32 = 1.0;
const MAX_LOOK_YAW: f32 = 1.0;
const MAX_LOOK_PITCH: f32 = 0.7;
/// The share of the look the body turns with; the head turns the rest.
const BODY_FOLLOW: f32 = 0.35;
/// The eyes' height in the figure's image, down from its top.
const EYES_FROM_TOP: f32 = 0.2;
/// A figure image smaller than this (the narrow layout folds it away) shows nobody.
const MIN_SIDE: i32 = 8;

impl App {
    /// Names the figure's image for the menu about to frame; only the inventory shows one.
    pub(super) fn name_player_view(&mut self, kind: GuiKind) {
        if kind != GuiKind::Inventory || self.session.is_none() {
            self.ui.set_rendered_images(&[]);
            return;
        }
        let size = self.player_view.as_ref().map_or((1, 1), |view| view.size);
        self.ui
            .state_mut()
            .set(PLAYER_VIEW_ID, UiValue::Str(PLAYER_VIEW_IMAGE.into()));
        self.ui
            .set_rendered_images(&[(PLAYER_VIEW_IMAGE.to_owned(), size)]);
    }

    /// Poses the figure for the frame just solved, looking at the cursor.
    pub(super) fn pose_player_view(&mut self, kind: GuiKind) {
        self.player_view = None;
        let (Some(session), Some(rect)) = (
            self.session.as_ref().filter(|_| kind == GuiKind::Inventory),
            self.ui
                .out()
                .rect(PLAYER_VIEW_ID)
                .filter(|r| r.w >= MIN_SIDE && r.h >= MIN_SIDE),
        ) else {
            return;
        };
        let (w, h) = (rect.w.max(1) as f32, rect.h.max(1) as f32);
        let (cx, cy) = self.controls.pointer.cursor();
        let reach = h * LOOK_REACH;
        let dx = cx - (rect.x as f32 + w * 0.5);
        let dy = cy - (rect.y as f32 + h * EYES_FROM_TOP);
        let yaw = (dx / reach).atan().clamp(-MAX_LOOK_YAW, MAX_LOOK_YAW);
        let pitch = (-dy / reach).atan().clamp(-MAX_LOOK_PITCH, MAX_LOOK_PITCH);
        let Some((body, poses)) = session.presentation.animation.preview_body(
            session.game.worn_items(),
            yaw * BODY_FOLLOW,
            yaw * (1.0 - BODY_FOLLOW),
            pitch,
        ) else {
            return;
        };
        self.player_view = Some(PlayerView {
            body,
            poses,
            size: (rect.w.max(1) as u32, rect.h.max(1) as u32),
        });
    }
}
