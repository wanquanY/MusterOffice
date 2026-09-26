//! Developer measurement of stop ownership, not a full-page/product benchmark.
use mo_geometry::{Fixed, Point};
use mo_raster::*;
use std::{collections::BTreeSet, hint::black_box, time::Instant};
fn main() {
    let mode = std::env::args()
        .nth(1)
        .expect("shared or copied-array-model");
    let count = 64usize;
    let stops: Vec<_> = (0..4096)
        .map(|i| GradientStop {
            position: f64::from(i) / 4095.0,
            srgb: [f64::from(i % 256) / 255.0, 0.0, 0.0, 1.0],
        })
        .collect();
    let element_bytes = std::mem::size_of::<GradientStop>();
    let origin = Point {
        x: Fixed::ZERO,
        y: Fixed::ZERO,
    };
    let base = Brush::Gradient {
        gradient: Gradient {
            geometry: GradientGeometry::Linear {
                start: origin,
                end: Point {
                    x: Fixed::from_raw(32 << 32),
                    y: Fixed::ZERO,
                },
            },
            stops: stops.clone().into(),
            tile: GradientTile::Clamp,
            interpolation: GradientInterpolation::Srgb,
            alpha: GradientAlpha::Straight,
        },
    };
    let started = Instant::now();
    let (elapsed, allocations, checksum) = match mode.as_str() {
        "shared" => {
            let held: Vec<_> = (0..count).map(|_| base.rebased(origin).unwrap()).collect();
            let elapsed = started.elapsed();
            let arrays: Vec<_> = held
                .iter()
                .map(|b| match b {
                    Brush::Gradient { gradient } => gradient.stops.as_ref(),
                    _ => unreachable!(),
                })
                .collect();
            let unique: BTreeSet<_> = arrays.iter().map(|a| a.as_ptr()).collect();
            let checksum: f64 = arrays.iter().map(|a| a[17].srgb[0]).sum();
            black_box(&held);
            (elapsed, unique.len(), checksum)
        }
        "copied-array-model" => {
            let held: Vec<_> = (0..count).map(|_| stops.clone()).collect();
            let elapsed = started.elapsed();
            let unique: BTreeSet<_> = held.iter().map(|a| a.as_ptr()).collect();
            let checksum: f64 = held.iter().map(|a| a[17].srgb[0]).sum();
            black_box(&held);
            (elapsed, unique.len(), checksum)
        }
        _ => panic!("unknown benchmark mode"),
    };
    println!(
        "{}",
        serde_json::json!({"mode":mode,"instances":count,"stopsPerInstance":4096,"stopBytes":element_bytes,"retainedStopBuffers":allocations,"retainedStopPayloadBytes":allocations*4096*element_bytes,"constructionNanos":elapsed.as_nanos(),"checksum":checksum})
    );
}
