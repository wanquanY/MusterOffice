#[allow(dead_code)]
#[path = "../../../tools/test-support/paint_admission.rs"]
mod support;
use mo_image::{DecoderReply, ImageDecoder, ImageError};
use mo_opc::{Package, PackageLimits};
use mo_presentation_compile::source_resource_page::*;
use mo_raster::Brush;
use support::*;
#[derive(Default)]
struct Decoder(usize);
impl ImageDecoder for Decoder {
    fn decode(&mut self, bytes: &[u8]) -> Result<DecoderReply, ImageError> {
        assert_eq!(bytes, PNG);
        self.0 += 1;
        Ok(DecoderReply {
            status: 0,
            words: [2, 2, 2, 2, 1, 1, 8, 0, 16],
            pixels: vec![255; 16],
        })
    }
    fn invalidate(&mut self) {
        panic!("valid fixture decoder invalidated")
    }
}
#[test]
fn native_paint_budget_precedes_decode_and_stops_share_across_all_paths() {
    for (name, count, unpainted, bg, success) in [
        ("at-limit", 64, false, false, true),
        ("over-limit", 65, false, false, false),
        ("unpainted", 65, true, false, true),
        ("background", 64, false, true, false),
    ] {
        let bytes = source(count, unpainted, bg);
        let index = read(&bytes);
        let q = request(&index);
        let package = Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            PackageLimits::default(),
            &|| false,
        )
        .unwrap();
        let mut decoder = Decoder::default();
        let prepared = prepare(&package, &index, &q, &mut decoder, None, options(), &|| {
            false
        });
        let expected = if success {
            let plan = prepared
                .unwrap_or_else(|e| panic!("{name}: {e:?}"))
                .plan(&|| false)
                .unwrap();
            let ramps: Vec<_> = plan
                .page
                .raster
                .scene
                .instances
                .iter()
                .filter_map(|i| match &i.brush {
                    Brush::Gradient { gradient } => Some(&gradient.stops),
                    _ => None,
                })
                .collect();
            assert_eq!(ramps.len(), 64);
            assert!(ramps.iter().all(|s| s.as_ptr() == ramps[0].as_ptr()));
            assert_eq!(plan.page.device_work.compiled_paths, 3);
            assert_eq!(decoder.0, 1);
            None
        } else {
            let error = match prepared {
                Err(e) => e,
                Ok(_) => panic!("{name}: oversized page prepared"),
            };
            assert!(
                error.to_string().contains("gradient input stops"),
                "{error}"
            );
            assert_eq!(decoder.0, 0);
            Some(error.to_string())
        };
        if let Some(dir) = std::env::var_os("MO_PAINT_ADMISSION_DIR") {
            let dir = std::path::Path::new(&dir);
            assert!(dir.is_absolute());
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(dir.join(format!("{name}.pptx")), bytes).unwrap();
            std::fs::write(
                dir.join(format!("{name}.page.json")),
                serde_json::to_vec_pretty(&q).unwrap(),
            )
            .unwrap();
            std::fs::write(
                dir.join(format!("{name}.expected.json")),
                serde_json::to_vec(
                    &serde_json::json!({"success":success,"error":expected,"decodes":decoder.0}),
                )
                .unwrap(),
            )
            .unwrap();
        }
    }
}
