// petramond::underwater — what the eye sees from inside a fluid medium that has optics
// (water; the frame fills the water lanes whenever the eye is in it or at its surface, and sets
// fog.w = 2 while the eye is inside). A ray sets out in the water when it meets the near plane
// under the surface (`underwater_ray`), so an eye at the waterline sees water below the line
// and air above it.
//
// Light from a surface loses red first as it travels (per-channel Beer–Lambert) while the
// water's own glow scatters in, so near things keep their colour and distance turns to the
// water's hue. The glow is brighter looking up toward the surface and toward the sun bent
// through it, darker looking down; the CPU has already folded the biome's water colour, the
// eye's depth and the time of day into it. Daylight dims by `depth_dim` per block of depth: the
// glow at the eye's depth, the light reaching a surface at its own (`underwater_sky_curve`).
// Skylight that reached a surface crossed the water above it, so each shader shifts its sky term
// toward the water's hue with depth (`underwater_hue_at`); block light starts beside what it
// lights and keeps its colour.
//
// Every world shader fogs through these, so terrain, mobs, particles and the sky agree.

struct Underwater {
    absorb: vec3<f32>,
    shift: f32,
    glow: vec3<f32>,
    eye_depth: f32,
    fog_start: f32,
    fog_end: f32,
    // The share of daylight lost per block of depth.
    depth_dim: f32,
};

const UNDERWATER_LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

// The frame's water lanes: `absorb` = (absorption per block, hue shift per block of depth),
// `glow` = (glow, eye depth), `fog` = (the water's fog start, fog end, daylight lost per block of
// depth).
fn underwater(absorb: vec4<f32>, glow: vec4<f32>, fog: vec4<f32>) -> Underwater {
    return Underwater(absorb.rgb, absorb.w, glow.rgb, glow.w, fog.x, fog.y, fog.z);
}

// Whether any ray of the frame can set out under the water. `line` = the frame's `waterline`
// lane (the view's depth gradient over the near distance, the surface above the eye); a zero
// lane has none.
fn underwater_rays(line: vec4<f32>) -> bool {
    return any(line.xyz != vec3<f32>(0.0));
}

// Whether the ray toward `view` (a point less the eye) sets out under the water: it meets the near
// plane under the surface. A point short of the near plane, or behind the eye, stands at its own
// height: that is where the near plane cuts what reaches it. Mirrored by
// `render::waterline::Waterline::under`.
fn underwater_ray(line: vec4<f32>, view: vec3<f32>) -> bool {
    return underwater_rays(line) && view.y < line.w * max(dot(line.xyz, view), 1.0);
}

// The water's own colour seen along unit `dir`.
fn underwater_glow(w: Underwater, dir: vec3<f32>, sun_dir: vec3<f32>, daylight: f32) -> vec3<f32> {
    let vertical = mix(0.4, 1.3, smoothstep(-0.85, 0.8, dir.y));
    // Sunlight bends toward the zenith as it enters and scatters forward along that path.
    let refracted = normalize(vec3<f32>(sun_dir.x * 0.75, max(sun_dir.y, 0.2), sun_dir.z * 0.75));
    let toward = max(dot(dir, refracted), 0.0);
    let t2 = toward * toward;
    let t4 = t2 * t2;
    let t8 = t4 * t4;
    let halo = (t8 * t8 * 1.1 + t4 * 0.22) * daylight;
    return w.glow * (vertical + halo);
}

// All glow by the water's fog end: nothing seen through more water than that may still show.
fn underwater_terminal(w: Underwater, dist: f32) -> f32 {
    return smoothstep(mix(w.fog_start, w.fog_end, 0.55), w.fog_end, dist);
}

// How far a surface `dist` away has gone into the glow, 0..1, for shaders that fade a
// multiplier (shadows, cracks) rather than a colour.
fn underwater_amount(w: Underwater, dist: f32) -> f32 {
    let lost = 1.0 - dot(exp(-w.absorb * dist), UNDERWATER_LUMA);
    return max(lost, underwater_terminal(w, dist));
}

// `color` arriving along unit `dir` after crossing `path` blocks of water.
fn underwater_through(
    w: Underwater,
    color: vec3<f32>,
    path: f32,
    dir: vec3<f32>,
    sun_dir: vec3<f32>,
    daylight: f32,
) -> vec3<f32> {
    let glow = underwater_glow(w, dir, sun_dir, daylight);
    let through = exp(-w.absorb * path);
    let seen = color * through + glow * (vec3<f32>(1.0) - through);
    return mix(seen, glow, underwater_terminal(w, path));
}

// The sky from under the water: what lies above the surface, through the water between the eye
// and the surface, looking up; the water's own glow looking down.
fn underwater_sky(
    w: Underwater,
    sky: vec3<f32>,
    ray: vec3<f32>,
    sun_dir: vec3<f32>,
    daylight: f32,
) -> vec3<f32> {
    let glow = underwater_glow(w, ray, sun_dir, daylight);
    let up = max(ray.y, 0.0);
    let through = underwater_through(w, sky, w.eye_depth / max(up, 0.05), ray, sun_dir, daylight);
    return mix(glow, through, smoothstep(0.0, 0.12, up));
}

// The tint of skylight that crossed `depth` blocks of water, its strongest channel kept whole
// (mirrored by `render::lighting::WaterLight::hue_at`).
fn underwater_hue(w: Underwater, depth: f32) -> vec3<f32> {
    let filtered = exp(-w.absorb * (max(depth, 0.0) * w.shift));
    return filtered / max(filtered.r, max(filtered.g, filtered.b));
}

// `underwater_hue` of skylight reaching absolute height `y`, with the eye at absolute `cam_y`.
fn underwater_hue_at(w: Underwater, cam_y: f32, y: f32) -> vec3<f32> {
    return underwater_hue(w, w.eye_depth + cam_y - y);
}

// The skylight flood loses 2 of 30 levels per block of water, so open water goes dark about 15
// blocks down, far steeper than water really dims. With the eye under water, a surface `depth`
// below the surface instead gets what open water passes at that depth (losing `depth_dim` per
// block, as the glow does), times the share of open water's flood it actually received to the
// 1.5th power: light in water is diffuse, so shade under an overhang falls off gentler than the
// cube a cave gets in air, and still reaches nothing where the flood does. Past the flood's reach, a point in deep open water keeps
// the smallest skylight step (meshing and entity light find the open column above it) and a
// cave keeps none, so open water's flood is never taken below that step. Returns the light
// curve (the cube of `sky` above water).
const UNDERWATER_SKY_STEP: f32 = 2.0 / 30.0;
// The 6-bit skylight deep open water keeps (mirrored by `render::lighting`).
const UNDERWATER_DEEP_SKY6: f32 = 2.0;

fn underwater_sky_curve(w: Underwater, sky: f32, depth: f32) -> f32 {
    let open = max(1.0 - depth * UNDERWATER_SKY_STEP, UNDERWATER_DEEP_SKY6 / 63.0);
    let reached = clamp(sky / open, 0.0, 1.0);
    return reached * sqrt(reached) * exp(-depth * w.depth_dim);
}
