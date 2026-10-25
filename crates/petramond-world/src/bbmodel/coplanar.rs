//! Faces of two cubes lying in one plane over a shared, exposed area. The depth test cannot
//! order them, so the overlap flickers between the two textures (z-fighting).

use std::cell::OnceCell;

use glam::{Mat4, Vec2, Vec3};
use petramond_math::face::Face;

use super::{face_corners, Cube, FaceUv, Model};

/// Planes this close (model units, a sixteenth of a block) are one plane.
const PLANE_EPS: f32 = 1e-3;
/// Texels below this alpha are cut out (the pick and AO threshold).
const CUTOUT_ALPHA: u8 = 128;
/// Normals this parallel (about a degree apart) face one way.
const PARALLEL: f32 = 1.0 - 1.5e-4;
/// Overlaps smaller than this (square model units) are an edge or a corner, not an area.
const AREA_EPS: f32 = 1e-2;
/// How far off the plane the overlap is probed for a cube burying it.
const PROBE: f32 = 1e-2;

/// Whether a cube is drawn, for a row that shows parts of a model.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Shown {
    Always,
    Never,
    /// An optional part, shown or not per placed instance. Two different parts may never show
    /// together, so they are not compared.
    Part(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoplanarFaces {
    pub a: (String, Face),
    pub b: (String, Face),
    /// The two cubes' indices in [`Model::cubes`].
    pub cubes: (usize, usize),
    pub area: f32,
}

struct Quad {
    cube: usize,
    face: Face,
    corners: [Vec3; 4],
    normal: Vec3,
}

impl Model {
    /// Every pair of textured faces from different cubes that share a plane over an exposed
    /// area at the rest pose. Drawn double-sided, two faces back to back fight as well.
    pub fn coplanar_faces(&self) -> Vec<CoplanarFaces> {
        self.coplanar_faces_where(|_| Shown::Always)
    }

    /// [`Model::coplanar_faces`] for a row that shows each cube (by name) as `shown` says.
    pub fn coplanar_faces_where(&self, shown: impl Fn(&str) -> Shown) -> Vec<CoplanarFaces> {
        let shown: Vec<Shown> = self.cubes.iter().map(|c| shown(&c.name)).collect();
        let pose = self.rest_pose();
        let transforms: Vec<Mat4> = self
            .cubes
            .iter()
            .map(|c| pose.get(c.bone).copied().unwrap_or(Mat4::IDENTITY) * c.local_transform())
            .collect();
        let mut quads = Vec::new();
        for (ci, (cube, m)) in self.cubes.iter().zip(&transforms).enumerate() {
            if shown[ci] == Shown::Never {
                continue;
            }
            for (face, uv) in Face::ALL.into_iter().zip(cube.faces) {
                let Some((uv, (corners, normal))) = uv.zip(placed_face(cube, face, m)) else {
                    continue;
                };
                if self
                    .face_texels(&corners, uv)
                    .all(|(_, a)| a < CUTOUT_ALPHA)
                {
                    continue;
                }
                quads.push(Quad {
                    cube: ci,
                    face,
                    corners,
                    normal,
                });
            }
        }
        let always: Vec<usize> = (0..self.cubes.len())
            .filter(|&ci| shown[ci] == Shown::Always)
            .collect();
        let inverses: Vec<Mat4> = transforms.iter().map(Mat4::inverse).collect();
        let inside = |ci: usize, p: Vec3| {
            let c = &self.cubes[ci];
            let l = inverses[ci].transform_point3(p);
            let (lo, hi) = (c.from.min(c.to), c.from.max(c.to));
            (lo + Vec3::splat(1e-4)).cmplt(l).all() && l.cmplt(hi - Vec3::splat(1e-4)).all()
        };
        // A cube showing a cut-out texel to the open (a decal plate, a vine) is seen through,
        // so what lies inside it stays exposed.
        let see_through: Vec<OnceCell<bool>> = vec![OnceCell::new(); self.cubes.len()];
        let covers = |ci: usize| {
            let cube = &self.cubes[ci];
            !*see_through[ci].get_or_init(|| {
                Face::ALL.into_iter().zip(cube.faces).any(|(face, uv)| {
                    let Some((uv, (corners, normal))) =
                        uv.zip(placed_face(cube, face, &transforms[ci]))
                    else {
                        return false;
                    };
                    self.face_texels(&corners, uv).any(|(p, a)| {
                        a < CUTOUT_ALPHA
                            && !always
                                .iter()
                                .any(|&o| o != ci && inside(o, p + normal * PROBE))
                    })
                })
            })
        };
        let buried = |p: Vec3| always.iter().any(|&ci| inside(ci, p) && covers(ci));
        let mut out = Vec::new();
        for (i, a) in quads.iter().enumerate() {
            for b in &quads[i + 1..] {
                let apart = matches!(
                    (shown[a.cube], shown[b.cube]),
                    (Shown::Part(x), Shown::Part(y)) if x != y
                );
                if a.cube == b.cube || apart || a.normal.dot(b.normal).abs() < PARALLEL {
                    continue;
                }
                let d = a.normal.dot(a.corners[0]);
                if b.corners
                    .iter()
                    .any(|c| (a.normal.dot(*c) - d).abs() > PLANE_EPS)
                {
                    continue;
                }
                let Some((area, centre)) = overlap(a, b) else {
                    continue;
                };
                let exposed = |n: Vec3| !buried(centre + n * PROBE);
                if exposed(a.normal) || exposed(b.normal) {
                    out.push(CoplanarFaces {
                        a: (self.cubes[a.cube].name.clone(), a.face),
                        b: (self.cubes[b.cube].name.clone(), b.face),
                        cubes: (a.cube, b.cube),
                        area,
                    });
                }
            }
        }
        out
    }

    /// Each texel a placed face shows: the texel centre's point on the face and its alpha.
    fn face_texels(
        &self,
        corners: &[Vec3; 4],
        uv: FaceUv,
    ) -> impl Iterator<Item = (Vec3, u8)> + '_ {
        let (w, h) = (self.tex_w.max(1), self.tex_h.max(1));
        let sheet = Vec2::new(w as f32, h as f32);
        let t = uv.corner_uv().map(Vec2::from);
        let texels = |edge: Vec2| ((edge * sheet).abs().max_element().round() as u32).max(1);
        let (ns, nr) = (texels(t[1] - t[0]), texels(t[3] - t[0]));
        let [p0, p1, _, p3] = *corners;
        (0..ns * nr).map(move |i| {
            let s = ((i % ns) as f32 + 0.5) / ns as f32;
            let r = ((i / ns) as f32 + 0.5) / nr as f32;
            let at = (t[0] + (t[1] - t[0]) * s + (t[3] - t[0]) * r) * sheet;
            let (x, y) = ((at.x as u32).min(w - 1), (at.y as u32).min(h - 1));
            let alpha = self.texture_rgba.get(((y * w + x) * 4 + 3) as usize);
            (
                p0 + (p1 - p0) * s + (p3 - p0) * r,
                alpha.copied().unwrap_or(u8::MAX),
            )
        })
    }
}

/// A cube face's corners and outward normal under `m`, unless it has no area.
fn placed_face(cube: &Cube, face: Face, m: &Mat4) -> Option<([Vec3; 4], Vec3)> {
    let corners = face_corners(face, cube.from, cube.to).map(|c| m.transform_point3(c.into()));
    let area = (corners[1] - corners[0]).cross(corners[3] - corners[0]);
    (area.length_squared() >= 1e-8).then(|| {
        (
            corners,
            m.transform_vector3(face.dir().as_vec3()).normalize(),
        )
    })
}

/// The area and centre of the overlap of two coplanar convex quads.
fn overlap(a: &Quad, b: &Quad) -> Option<(f32, Vec3)> {
    let origin = a.corners[0];
    let u = (a.corners[1] - origin).normalize();
    let v = a.normal.cross(u);
    let flat = |p: Vec3| Vec2::new((p - origin).dot(u), (p - origin).dot(v));
    let mut poly: Vec<Vec2> = a.corners.iter().map(|&p| flat(p)).collect();
    let clip: Vec<Vec2> = b.corners.iter().map(|&p| flat(p)).collect();
    let winding = signed_area(&clip).signum();
    for k in 0..clip.len() {
        let (e0, e1) = (clip[k], clip[(k + 1) % clip.len()]);
        let side = |p: Vec2| winding * (e1 - e0).perp_dot(p - e0);
        let mut next = Vec::with_capacity(poly.len() + 1);
        for (j, &p) in poly.iter().enumerate() {
            let q = poly[(j + 1) % poly.len()];
            let (sp, sq) = (side(p), side(q));
            if sp >= 0.0 {
                next.push(p);
            }
            if (sp >= 0.0) != (sq >= 0.0) {
                next.push(p + (q - p) * (sp / (sp - sq)));
            }
        }
        poly = next;
        if poly.len() < 3 {
            return None;
        }
    }
    let area = signed_area(&poly).abs();
    if area < AREA_EPS {
        return None;
    }
    let centre = poly.iter().copied().sum::<Vec2>() / poly.len() as f32;
    Some((area, origin + u * centre.x + v * centre.y))
}

fn signed_area(poly: &[Vec2]) -> f32 {
    (0..poly.len())
        .map(|i| poly[i].perp_dot(poly[(i + 1) % poly.len()]))
        .sum::<f32>()
        * 0.5
}
