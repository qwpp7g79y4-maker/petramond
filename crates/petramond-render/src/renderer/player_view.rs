//! The local player drawn into a GUI image: the inventory's figure, which the client poses to
//! face the viewer and look at the cursor. Its own skin batch, held-item streams and front
//! orthographic camera over a target the document image samples, drawn at the scene's sample
//! count each frame a document shows it, lit the same at any hour or depth.

use super::dynamic_bake::{bake_player_body, HeldStreams};
use super::*;

/// The figure's frame in blocks: the feet a sliver above the bottom edge, room above the head
/// for a helmet.
const VIEW_BOTTOM: f32 = -0.08;
const VIEW_TOP: f32 = 2.05;
const CAMERA_BACK: f32 = 4.0;
/// From in front, above and the viewer's left.
const VIEW_SUN: glam::Vec3 = glam::Vec3::new(-0.35, 0.75, 0.55);
const MAX_SIDE: u32 = 1024;

pub(super) struct PlayerViewPass {
    skin: SkinFrame,
    sprites: DynamicDraw,
    models: DynamicDraw,
    blocks: DynamicDraw,
    streams: HeldStreams,
    uniform_buf: wgpu::Buffer,
    uniform_bind: wgpu::BindGroup,
    view: Option<PlayerView>,
    target: Option<Target>,
}

struct Target {
    size: (u32, u32),
    samples: u32,
    resolve: wgpu::TextureView,
    multisample: Option<wgpu::TextureView>,
    depth: wgpu::TextureView,
    bind: wgpu::BindGroup,
}

fn target_texture(
    device: &wgpu::Device,
    label: &str,
    (w, h): (u32, u32),
    format: wgpu::TextureFormat,
    samples: u32,
    sampled: bool,
) -> wgpu::Texture {
    crate::gpu_mem::create_texture(
        device,
        &wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: if sampled {
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING
            } else {
                wgpu::TextureUsages::RENDER_ATTACHMENT
            },
            view_formats: &[],
        },
    )
}

impl PlayerViewPass {
    pub(super) fn new(
        device: &wgpu::Device,
        actor: &ActorPass,
        uniform_bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("player view uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uv_rects = crate::uniforms::uv_rects_placeholder(device, "player view uv table");
        Self {
            skin: actor.skin.another(device),
            sprites: DynamicDraw::new(
                device,
                actor.item_draw.pipeline.clone(),
                "player view sprites",
            ),
            models: DynamicDraw::new(
                device,
                actor.model_item_draw.pipeline.clone(),
                "player view models",
            ),
            blocks: DynamicDraw::new(
                device,
                actor.block_item_draw.pipeline.clone(),
                "player view blocks",
            ),
            streams: HeldStreams::default(),
            uniform_bind: crate::selection_highlight::inactive_bind(
                device,
                uniform_bgl,
                &uniform_buf,
                &uv_rects,
            ),
            uniform_buf,
            view: None,
            target: None,
        }
    }

    /// Takes this frame's view and answers the image the UI samples for it, its target sized to
    /// the view.
    pub(super) fn prepare(
        &mut self,
        gpu: &super::doc_ui::UiGpu<'_>,
        format: wgpu::TextureFormat,
        samples: u32,
        view: Option<&PlayerView>,
    ) -> Option<wgpu::BindGroup> {
        self.view = view.cloned();
        let view = self.view.as_ref()?;
        let size = (
            view.size.0.clamp(1, MAX_SIDE),
            view.size.1.clamp(1, MAX_SIDE),
        );
        if self
            .target
            .as_ref()
            .is_none_or(|t| t.size != size || t.samples != samples)
        {
            let resolve = target_texture(gpu.device, "player view", size, format, 1, true);
            self.target = Some(Target {
                size,
                samples,
                multisample: (samples > 1).then(|| {
                    target_texture(
                        gpu.device,
                        "player view multisample",
                        size,
                        format,
                        samples,
                        false,
                    )
                    .create_view(&wgpu::TextureViewDescriptor::default())
                }),
                depth: target_texture(
                    gpu.device,
                    "player view depth",
                    size,
                    wgpu::TextureFormat::Depth32Float,
                    samples,
                    false,
                )
                .create_view(&wgpu::TextureViewDescriptor::default()),
                bind: gpu.nearest_bind(&resolve, "player view image"),
                resolve: resolve.create_view(&wgpu::TextureViewDescriptor::default()),
            });
        }
        self.target.as_ref().map(|t| t.bind.clone())
    }

    fn camera(size: (u32, u32)) -> Uniforms {
        let half_h = (VIEW_TOP - VIEW_BOTTOM) * 0.5;
        let half_w = half_h * size.0 as f32 / size.1.max(1) as f32;
        let centre = glam::Vec3::new(0.0, VIEW_BOTTOM + half_h, 0.0);
        let eye = centre + glam::Vec3::Z * CAMERA_BACK;
        let view_proj =
            glam::Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, 0.1, CAMERA_BACK * 2.0)
                * glam::Mat4::look_at_rh(eye, centre, glam::Vec3::Y);
        let sun = VIEW_SUN.normalize();
        Uniforms {
            view_proj: view_proj.to_cols_array_2d(),
            cam_pos: [eye.x, eye.y, eye.z, 0.0],
            fog: [1.0e4, 2.0e4, 0.0, 0.0],
            fog_color: [0.0, 0.0, 0.0, 1.0],
            inv_view_proj: view_proj.inverse().to_cols_array_2d(),
            render_origin: [0; 4],
            atlas_layout: crate::atlas::atlas_layout_uniform(),
            sky_color: [1.0, 1.0, 1.0, 0.0],
            sun_dir: [sun.x, sun.y, sun.z, 1.0],
            volume_tint: [1.0, 1.0, 1.0, 0.0],
            water_absorb: [0.0; 4],
            water_glow: [0.0; 4],
            water_fog: [0.0; 4],
            waterline: [0.0; 4],
        }
    }
}

impl Renderer {
    pub(super) fn encode_player_view(&mut self, enc: &mut wgpu::CommandEncoder) {
        let Some(rig) =
            petramond::player::rigs::presented(petramond::player::Presenter::Body).map(|(_, r)| r)
        else {
            return;
        };
        let Self {
            player_view: pv,
            actor,
            binds,
            device,
            queue,
            ..
        } = self;
        let (Some(view), Some(target)) = (pv.view.as_ref(), pv.target.as_ref()) else {
            return;
        };
        pv.skin.batch.clear();
        let streams = &mut pv.streams;
        streams.block_verts.clear();
        streams.block_indices.clear();
        streams.sprite_verts.clear();
        streams.sprite_indices.clear();
        streams.model_verts.clear();
        streams.model_indices.clear();
        bake_player_body(
            rig,
            &view.body,
            view.body.body.pose.of(&view.poses),
            glam::IVec3::ZERO,
            &mut pv.skin.batch,
            streams,
            crate::lighting::LightEnv::IDENTITY,
        );
        let drawn = 0..pv.skin.batch.next_instance();
        pv.skin.upload(device, queue);
        let prebuilt = |_: &mut Vec<_>, i: &mut Vec<u32>| i.len() as u32;
        pv.sprites.bake(
            device,
            queue,
            &mut streams.sprite_verts,
            &mut streams.sprite_indices,
            prebuilt,
        );
        pv.models.bake(
            device,
            queue,
            &mut streams.model_verts,
            &mut streams.model_indices,
            prebuilt,
        );
        pv.blocks.bake(
            device,
            queue,
            &mut streams.block_verts,
            &mut streams.block_indices,
            |_: &mut Vec<_>, i: &mut Vec<u32>| i.len() as u32,
        );
        queue.write_buffer(
            &pv.uniform_buf,
            0,
            bytemuck::bytes_of(&PlayerViewPass::camera(target.size)),
        );

        let (view, resolve_target) = match &target.multisample {
            Some(multisample) => (multisample, Some(&target.resolve)),
            None => (&target.resolve, None),
        };
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("player view"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: if resolve_target.is_some() {
                        wgpu::StoreOp::Discard
                    } else {
                        wgpu::StoreOp::Store
                    },
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let samples = target.samples;
        pass.set_bind_group(0, &pv.uniform_bind, &[]);
        pass.set_bind_group(1, &actor.player_gpu.bind, &[]);
        pv.skin
            .draw(&mut pass, samples, &actor.player_gpu.mesh, drawn);
        for (draw, atlas) in [
            (&pv.sprites, &binds.atlas),
            (&pv.models, &binds.model_atlas),
            (&pv.blocks, &binds.atlas_array),
        ] {
            if draw.index_count > 0 {
                pass.set_bind_group(1, atlas, &[]);
                draw.draw(&mut pass, samples);
            }
        }
    }
}
