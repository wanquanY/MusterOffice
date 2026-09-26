//! Verification host for real PPTX -> relationship resolution -> decoding ->
//! local image geometry. It does not claim to be a completed page renderer.
use mo_opc::{Package, PackageLimits};
use mo_pptx::source::{SourceLimits, images::*, inspect_source};
use std::io::{Read, Write};
fn audit(
    index: &mo_pptx::source::SourceIndex,
    catalog: &SourceImageResources,
    decoded: &[mo_image::DecodedImage],
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    use mo_pptx::source::fill::resolve::FillTarget;
    use mo_presentation_compile::source_image_layout::layout_source;
    let mut rejected = vec![];
    for name in [
        "source-digest",
        "surface",
        "decoded-digest",
        "resource-index",
        "unresolved-target",
        "root-box",
        "target-limit",
        "resource-limit",
    ] {
        let mut c = catalog.clone();
        match name {
            "source-digest" => c.source_sha256 = mo_common::Digest::from_sha256([0; 32]),
            "surface" => c.surface = "/ppt/slides/missing.xml".into(),
            "decoded-digest" => c.resources[0].sha256 = mo_common::Digest::from_sha256([0; 32]),
            "resource-index" => {
                if let SourceImageOutcome::Available { resource, .. } = &mut c.targets[0].outcome {
                    *resource = u32::MAX;
                }
            }
            "unresolved-target" => c.targets[1].outcome = SourceImageOutcome::NotImage {},
            "root-box" => c.targets[0].target = FillTarget::RootGroup {},
            "target-limit" => c.targets.resize(4097, c.targets[0].clone()),
            "resource-limit" => c.resources.resize(4097, c.resources[0].clone()),
            _ => unreachable!(),
        }
        let error =
            layout_source(index, &c, decoded, &|| false).expect_err("malformed binding must fail");
        rejected.push(serde_json::json!({"name":name,"error":error.to_string()}));
    }
    let error =
        layout_source(index, catalog, &[], &|| false).expect_err("decoded cardinality must fail");
    rejected.push(serde_json::json!({"name":"decoded-cardinality","error":error.to_string()}));
    let calls = std::cell::Cell::new(0);
    layout_source(index, catalog, decoded, &|| {
        calls.set(calls.get() + 1);
        false
    })?;
    let checkpoints = calls.get();
    for at in 1..=checkpoints {
        let calls = std::cell::Cell::new(0);
        layout_source(index, catalog, decoded, &|| {
            calls.set(calls.get() + 1);
            calls.get() == at
        })
        .expect_err("cancelled source batch must fail");
    }
    Ok(serde_json::json!({"rejected":rejected,"cancelledCheckpoints":checkpoints}))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 && !(args.len() == 3 && matches!(args[2].as_str(), "--audit" | "--paint")) {
        return Err("expected explicit request JSON and owned PPTX path".into());
    }
    let mut bytes = vec![];
    std::fs::File::open(&args[1])?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("source size".into());
    }
    let mut request = String::new();
    std::fs::File::open(&args[0])?
        .take(8 * 1024 * 1024 + 1)
        .read_to_string(&mut request)?;
    if request.len() > 8 * 1024 * 1024 {
        return Err("request size".into());
    }
    let q: SourceImageQuery = mo_common::from_json_str(&request)?;
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )?;
    let index = inspect_source(&package, SourceLimits::default(), &|| false)?;
    let limits = SourceImageLimits::default();
    let catalog = query(&package, &index, &q, limits, &|| false)?;
    let encoded = extract(&package, &catalog, limits, &|| false)?;
    let mut backend = mo_skia_sys::NativeRaster;
    let mut decoded = vec![];
    let mut infos = vec![];
    let mut byte_length = 0usize;
    for r in &catalog.resources {
        let start = usize::try_from(r.offset.get())?;
        let end = start
            .checked_add(usize::try_from(r.byte_length.get())?)
            .ok_or("resource span")?;
        let image = mo_image::decode(
            encoded.get(start..end).ok_or("resource span")?,
            &r.sha256,
            &mut backend,
            &|| false,
        )?;
        byte_length = byte_length
            .checked_add(image.pixels().len())
            .filter(|n| *n <= mo_image::MAX_PIXEL_BYTES)
            .ok_or("decoded bytes")?;
        infos.push(image.info().clone());
        decoded.push(image);
    }
    let result = mo_presentation_compile::source_image_layout::layout_source(
        &index,
        &catalog,
        &decoded,
        &|| false,
    );
    let paint = if args.get(2).is_some_and(|v| v == "--paint") {
        Some(match &result {
            Ok(plans) => {
                let mut output = vec![];
                let mut error = None;
                for plan in plans {
                    match mo_presentation_compile::source_image_paint::compile(
                        plan,
                        mo_raster::ImageSampling::Nearest,
                        &|| false,
                    ) {
                        Ok(p) => output.push(p),
                        Err(e) => {
                            error = Some((plan.target.clone(), e.to_string()));
                            break;
                        }
                    }
                }
                match error {
                    Some((target, message)) => {
                        serde_json::json!({"status":"error","target":target,"message":message})
                    }
                    None => serde_json::json!({"status":"compiled","paints":output}),
                }
            }
            Err(e) => serde_json::json!({"status":"error","message":e.to_string()}),
        })
    } else {
        None
    };
    let result = match result {
        Ok(plans) => serde_json::json!({"status":"laidOut","plans":plans}),
        Err(error) => serde_json::json!({"status":"error","message":error.to_string()}),
    };
    let checks = if args.get(2).is_some_and(|v| v == "--audit") {
        Some(audit(&index, &catalog, &decoded)?)
    } else {
        None
    };
    let mut output =
        serde_json::json!({"catalog":catalog,"decoded":infos,"result":result,"checks":checks});
    if let Some(paint) = paint {
        output["paint"] = paint;
    }
    std::io::stdout()
        .lock()
        .write_all(&serde_json::to_vec(&output)?)?;
    Ok(())
}
