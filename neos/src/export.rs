//! Exports the same real geometry and amplitude data [`crate::render`]
//! rasterises, as JSON — so an interactive viewer (outside this process,
//! outside Rust entirely) can draw and animate the *exact* numbers this
//! report already prints and rasterises, rather than a reconstruction of
//! them. No new dependency: JSON here is hand-written, the same discipline
//! [`crate::gif`] already follows for GIF89a — every value written is a real
//! `f64` this binary already computed, not re-derived in a second language.
//!
//! Reuses [`gui::TetryenVisualisation::at`]/[`gui::LoadVisualisation::at`]
//! directly, the same two calls [`crate::render::render_animation`] samples
//! — this is a second consumer of the same real values, not a parallel
//! computation that could drift from what the PPM/GIF outputs show.

use gui::{LoadVisualisation, Tetryen, TetryenVisualisation};
use std::io::Write;

const FRAMES: usize = 24;

fn json_f64(v: f64) -> String {
    // Every value here is a real sinusoid/coordinate; JSON has no `inf`/`nan`
    // literal, so a non-finite value (which would mean a real bug upstream,
    // not an expected case) is written as `null` rather than emitting
    // invalid JSON.
    if v.is_finite() {
        format!("{v}")
    } else {
        "null".to_string()
    }
}

fn json_array(values: &[f64]) -> String {
    let mut out = String::from("[");
    for (i, v) in values.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&json_f64(*v));
    }
    out.push(']');
    out
}

fn json_point(coords: &[f64; 4]) -> String {
    json_array(coords)
}

/// One [`TetryenVisualisation`] panel's real per-node amplitude, sampled at
/// the same [`FRAMES`] points across one real period every animation frame
/// already samples.
fn panel_json(name: &str, vis: &TetryenVisualisation, x0: f64, period: f64) -> String {
    let mut frames = Vec::with_capacity(FRAMES);
    for f in 0..FRAMES {
        let t = period * f as f64 / FRAMES as f64;
        let values: Vec<f64> = (0..4).map(|i| vis.at(i, x0, t).abs()).collect();
        frames.push(json_array(&values));
    }
    format!(
        r#"{{"name":{name:?},"peak":{},"frames":[{}]}}"#,
        json_f64(vis.peak()),
        frames.join(",")
    )
}

/// Writes the real geometry, real per-panel node amplitudes, and the real
/// shared load field to `path` as one JSON object — everything an
/// interactive viewer needs to redraw and animate exactly what
/// [`crate::render`] already rasterised to PPM/GIF for the same panels.
///
/// # Errors
/// Any I/O failure creating or writing `path`.
pub fn write_json_report(
    path: &str,
    geometry: &Tetryen,
    panels: &[(&str, &TetryenVisualisation)],
    load: &LoadVisualisation,
    imbalance: f64,
    k: f64,
    omega: f64,
) -> std::io::Result<()> {
    let x0 = std::f64::consts::FRAC_PI_2 / k;
    let period = std::f64::consts::TAU / omega;

    let nodes_rest: Vec<String> = geometry.nodes().iter().map(|n| json_point(n.coords())).collect();

    let edges = geometry
        .edges()
        .expect("this report's own Tetryen is never degenerate");
    let edges_json: Vec<String> = edges
        .iter()
        .map(|edge| {
            let samples = edge
                .sample(FRAMES)
                .expect("sampling a non-degenerate edge cannot fail");
            let points: Vec<String> = samples.iter().map(|p| json_point(p.coords())).collect();
            format!("[{}]", points.join(","))
        })
        .collect();

    let cores = load.cores();
    let mut load_frames = Vec::with_capacity(FRAMES);
    for f in 0..FRAMES {
        let t = period * f as f64 / FRAMES as f64;
        let values: Vec<f64> = (0..cores).map(|c| load.at(c, x0, t).abs()).collect();
        load_frames.push(json_array(&values));
    }

    let panels_json: Vec<String> = panels.iter().map(|(n, v)| panel_json(n, v, x0, period)).collect();

    let body = format!(
        r#"{{
  "k": {},
  "omega": {},
  "period": {},
  "frames": {FRAMES},
  "nodes_rest": [{}],
  "edges": [{}],
  "cores": {cores},
  "load": {{"peak": {}, "imbalance": {}, "frames": [{}]}},
  "panels": [{}]
}}
"#,
        json_f64(k),
        json_f64(omega),
        json_f64(period),
        nodes_rest.join(","),
        edges_json.join(","),
        json_f64(load.peak()),
        json_f64(imbalance),
        load_frames.join(","),
        panels_json.join(",")
    );

    let mut file = std::fs::File::create(path)?;
    file.write_all(body.as_bytes())
}
