//! A minimal software rasteriser for the demo report.
//!
//! `gui` deliberately has no framebuffer, window, or GPU binding — its own
//! module docs say so directly, and that boundary is a documented decision,
//! not an oversight, since the testable content there is the geometry, not
//! the pixels. This module does not change that: it is a *consumer* of
//! `gui`'s geometry and amplitude types, living in the demo binary, that
//! samples the same [`gui::GeodesicEdge`]s and reads the same
//! [`gui::TetryenVisualisation`]/[`gui::LoadVisualisation`] amplitudes the
//! report already prints as numbers, and draws them instead.
//!
//! No new dependency. PPM (P6) is a format this workspace already reads —
//! [`crystallisation::decode_ppm`] — this just writes it, by hand. The GIF
//! encoder in [`crate::gif`] is the same idea applied to a format nothing
//! here read before.
//!
//! Every frame draws from one fixed 16-colour palette rather than being
//! quantised after the fact, so the animation's frames are GIF-ready as
//! produced — no per-frame colour reduction step needed.

use gui::{LoadVisualisation, Tetryen, TetryenVisualisation};

type Rgb = [u8; 3];

const BACKGROUND: Rgb = [246, 246, 248];
const WIREFRAME: Rgb = [140, 140, 150];
const BOUNDARY: Rgb = [205, 205, 210];
const COLD: Rgb = [30, 60, 150]; // value 0
const HOT: Rgb = [235, 90, 40]; // value 1

const IDX_BG: u8 = 0;
const IDX_WIRE: u8 = 1;
const IDX_BOUNDARY: u8 = 2;
const IDX_RAMP0: u8 = 3;
const RAMP_LEN: usize = 13; // 3 fixed + 13 ramp = 16, an exact GIF table size

fn lerp(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|i| (a[i] as f64 + (b[i] as f64 - a[i] as f64) * t).round() as u8)
}

/// The fixed 16-colour palette every frame draws from.
pub fn palette() -> Vec<Rgb> {
    let mut p = vec![BACKGROUND, WIREFRAME, BOUNDARY];
    for i in 0..RAMP_LEN {
        p.push(lerp(COLD, HOT, i as f64 / (RAMP_LEN - 1) as f64));
    }
    p
}

fn ramp_index(value: f64) -> u8 {
    let t = value.clamp(0.0, 1.0);
    IDX_RAMP0 + (t * (RAMP_LEN - 1) as f64).round() as u8
}

/// Only the ball's first two of four coordinates are drawn — an honest 2D
/// shadow of a 4D shape, not a claim of full fidelity. Every ball point has
/// `||u|| < 1`, so this always lands inside the drawn boundary circle.
fn project(coords: &[f64; 4], size: usize, margin: f64) -> (f64, f64) {
    let radius = (size as f64 - 2.0 * margin) / 2.0;
    let centre = margin + radius;
    (centre + coords[0] * radius, centre - coords[1] * radius)
}

fn set_pixel(buf: &mut [u8], size: usize, x: i64, y: i64, idx: u8) {
    if x < 0 || y < 0 || x as usize >= size || y as usize >= size {
        return;
    }
    buf[y as usize * size + x as usize] = idx;
}

fn draw_line(buf: &mut [u8], size: usize, (x0, y0): (f64, f64), (x1, y1): (f64, f64), idx: u8) {
    let steps = ((x1 - x0).abs().max((y1 - y0).abs()).ceil() as i64).max(1);
    for s in 0..=steps {
        let t = s as f64 / steps as f64;
        set_pixel(
            buf,
            size,
            (x0 + (x1 - x0) * t).round() as i64,
            (y0 + (y1 - y0) * t).round() as i64,
            idx,
        );
    }
}

fn draw_disc(buf: &mut [u8], size: usize, (cx, cy): (f64, f64), radius: i64, idx: u8) {
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy <= radius * radius {
                set_pixel(buf, size, cx.round() as i64 + dx, cy.round() as i64 + dy, idx);
            }
        }
    }
}

fn draw_circle_outline(buf: &mut [u8], size: usize, centre: (f64, f64), radius: f64, idx: u8) {
    let steps = 360;
    for i in 0..steps {
        let theta = i as f64 / steps as f64 * std::f64::consts::TAU;
        let p = (centre.0 + radius * theta.cos(), centre.1 + radius * theta.sin());
        set_pixel(buf, size, p.0.round() as i64, p.1.round() as i64, idx);
    }
}

/// One frame, indexed into [`palette`]. `node_value(i)`/`bar_value(c)` give
/// each node's/core's displayed intensity in `[0, 1]` for this frame,
/// driving both colour and (for nodes) disc radius — the same panel serves
/// a single still (values sampled at `t = 0`) and a full animation (values
/// sampled across time), only the closures differ.
fn frame(
    size: usize,
    tetryen: &Tetryen,
    node_value: impl Fn(usize) -> f64,
    load: &LoadVisualisation,
    bar_value: impl Fn(usize) -> f64,
) -> Vec<u8> {
    let scale = size as f64 / 640.0;
    let margin = scale * 48.0;
    let mut buf = vec![IDX_BG; size * size];

    let ball_radius = (size as f64 - 2.0 * margin) / 2.0;
    draw_circle_outline(&mut buf, size, project(&[0.0; 4], size, margin), ball_radius, IDX_BOUNDARY);

    let edges = tetryen
        .edges()
        .expect("a Tetryen's own edges are never degenerate");
    for edge in &edges {
        let samples = edge
            .sample(24)
            .expect("sampling a non-degenerate edge cannot fail");
        for pair in samples.windows(2) {
            draw_line(
                &mut buf,
                size,
                project(pair[0].coords(), size, margin),
                project(pair[1].coords(), size, margin),
                IDX_WIRE,
            );
        }
    }

    for (i, node) in tetryen.nodes().iter().enumerate() {
        let v = node_value(i);
        let radius = (scale * (4.0 + v * 14.0)).round().max(1.0) as i64;
        draw_disc(&mut buf, size, project(node.coords(), size, margin), radius, ramp_index(v));
    }

    // A load strip beneath the tetryen: one bar per core, height and colour
    // by displayed intensity — the same numbers the report's "load field"
    // line already states, drawn instead of stated.
    let bar_bottom = size as f64 - scale * 12.0;
    let bar_top = size as f64 - scale * 56.0;
    let cores = load.cores();
    if cores > 0 {
        let bar_w = (size as f64 - 2.0 * margin) / cores as f64;
        for c in 0..cores {
            let v = bar_value(c);
            let x0 = margin + c as f64 * bar_w + 1.0;
            let x1 = margin + (c as f64 + 1.0) * bar_w - 1.0;
            let y0 = bar_bottom - (bar_bottom - bar_top) * v;
            let idx = ramp_index(v);
            let mut y = y0;
            while y <= bar_bottom {
                draw_line(&mut buf, size, (x0, y), (x1, y), idx);
                y += 1.0;
            }
        }
    }

    buf
}

fn to_rgb(idx: &[u8]) -> Vec<u8> {
    let pal = palette();
    let mut out = Vec::with_capacity(idx.len() * 3);
    for &i in idx {
        out.extend_from_slice(&pal[i as usize]);
    }
    out
}

pub const STATIC_SIZE: usize = 640;

/// A single still, sampled at `t = 0`. At `t = 0`, `2*A*sin(k*x0)*cos(0) = A`
/// exactly at `x0` below, so this is the same value the report already
/// prints as `tetryen.peak()`/`load.peak()`, just drawn.
pub fn render_still(
    tetryen: &Tetryen,
    tetryen_vis: &TetryenVisualisation,
    load: &LoadVisualisation,
    k: f64,
) -> Vec<u8> {
    let x0 = std::f64::consts::FRAC_PI_2 / k;
    let idx = frame(
        STATIC_SIZE,
        tetryen,
        |i| tetryen_vis.at(i, x0, 0.0).abs() / 2.0,
        load,
        |c| load.at(c, x0, 0.0).abs() / 2.0,
    );
    to_rgb(&idx)
}

pub fn write_ppm(path: &str, size: usize, rgb: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    write!(f, "P6\n{size} {size}\n255\n")?;
    f.write_all(rgb)?;
    Ok(())
}

pub const ANIM_SIZE: usize = 320;
pub const ANIM_FRAMES: usize = 24;
pub const ANIM_DELAY_CS: u16 = 5; // centiseconds per frame

/// A full period of real standing-wave motion, sampled at [`ANIM_FRAMES`]
/// points across `t in [0, 2*pi/omega)`.
///
/// `x0 = pi/(2k)` is the point where `sin(k*x0) = 1`, so
/// `wave.at(x0, t) = 2*amplitude*cos(omega*t)` exactly — every node's own
/// amplitude sets how far it swings, and all nodes share the same `cos(wt)`
/// phase, which is what "standing" means: the shape doesn't travel, each
/// point's own envelope just breathes in place. This reuses
/// `TetryenVisualisation::at`/`LoadVisualisation::at` rather than
/// reimplementing the formula — composing existing law, not reinventing it.
pub fn render_animation(
    tetryen: &Tetryen,
    tetryen_vis: &TetryenVisualisation,
    load: &LoadVisualisation,
    k: f64,
    omega: f64,
) -> Vec<Vec<u8>> {
    let x0 = std::f64::consts::FRAC_PI_2 / k;
    let period = std::f64::consts::TAU / omega;
    (0..ANIM_FRAMES)
        .map(|f| {
            let t = period * f as f64 / ANIM_FRAMES as f64;
            frame(
                ANIM_SIZE,
                tetryen,
                |i| tetryen_vis.at(i, x0, t).abs() / 2.0,
                load,
                |c| load.at(c, x0, t).abs() / 2.0,
            )
        })
        .collect()
}
