use std::io::{self, Read, Write};
mod artifact;
mod computation;
mod images;
mod opc_repair;
mod pptx;
mod raster;
mod text;
mod worker;

fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match run(&args) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            if args
                .first()
                .is_some_and(|s| s == "compute" || s == "compute-schema")
            {
                eprintln!("{}", computation::failure_json(error));
            } else {
                eprintln!("Error: {error:?}");
            }
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(args: &[std::ffi::OsString]) -> Result<(), Box<dyn std::error::Error>> {
    let output = match args {
        [command, source] if command == "opc-repair-inspect" => opc_repair::run(source, None)?,
        [command, source, digest, output] if command == "opc-repair-apply" => opc_repair::run(source, Some((digest, output)))?,
        [command, request] if command == "template" => {
            let request = artifact::read_limited(request, mo_embedded_sdk::template::TemplateLimits::default().max_bytes)?;
            mo_embedded_sdk::template::compute_template_json(std::str::from_utf8(&request)?, Default::default(), &||false)
        },
        [command, rest @ ..] if command == "compute" => computation::run(rest)?,
        [command, schema] if command == "compute-schema" => {
            let id = schema.to_str().ok_or("schema identifier must be UTF-8")?;
            mo_embedded_sdk::operation::computation_schema_json(&serde_json::to_string(id)?)?
        },
        [command, request, source] if command == "pptx-import" => {
            let request = artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
            let source = artifact::read_limited(source, mo_kernel_api::MAX_INLINE_RESOURCE_BYTES)?;
            mo_kernel_api::import_pptx_document_json(std::str::from_utf8(&request)?, &source)
        },
        [command, request, contents] if command == "delivery-inspect" => {
            let request = artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
            let file = std::fs::File::open(contents)?;
            let metadata = file.metadata()?;
            if !metadata.is_file() { return Err("input must be a regular file".into()); }
            let response = mo_kernel_api::inspect_delivery_at(std::str::from_utf8(&request)?,
                &rawzip::FileReader::from(file), metadata.len(), &|| false);
            serde_json::to_string(&response)?
        },
        [command, request, contents] if command == "delivery-playback" => {
            let request = artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
            let file = std::fs::File::open(contents)?;
            let metadata = file.metadata()?;
            if !metadata.is_file() { return Err("input must be a regular file".into()); }
            let response = mo_kernel_api::prepare_delivery_playback_at(std::str::from_utf8(&request)?,
                &rawzip::FileReader::from(file), metadata.len(), &|| false);
            serde_json::to_string(&response)?
        },
        [command,request] if command=="compile-playback-page"=>{
            let request=artifact::read_limited(request,mo_kernel_api::MAX_REQUEST_BYTES)?;
            mo_kernel_api::compile_playback_page_json(std::str::from_utf8(&request)?,&||false)
        },
        [command,request,output] if command=="render-playback-page"=>raster::render(request,output,raster::Mode::PlaybackPage)?,
        [command, request] if command == "evaluate-timeline" => {
            let request = artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
            mo_kernel_api::evaluate_timeline_json(std::str::from_utf8(&request)?, &|| false)
        },
        [command, request, source] if command == "pptx-timing" => {
            let request = artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
            let source = artifact::read_limited(source, mo_kernel_api::MAX_INLINE_RESOURCE_BYTES)?;
            mo_kernel_api::inspect_pptx_timing_json(std::str::from_utf8(&request)?, &source)
        },
        [command, request, source, output] if command == "pptx-extract-images" => images::extract(request, source, output)?,
        [command, request, source] if command == "pptx-charts" => {
            let request = artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
            let source = artifact::read_limited(source, mo_kernel_api::MAX_INLINE_RESOURCE_BYTES)?;
            mo_kernel_api::inspect_pptx_charts_json(std::str::from_utf8(&request)?, &source)
        },
        [command, request, source] if command == "pptx-chart-paints" => {
            let request = artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
            let source = artifact::read_limited(source, mo_kernel_api::MAX_INLINE_RESOURCE_BYTES)?;
            mo_kernel_api::compute_pptx_chart_paints_json(std::str::from_utf8(&request)?, &source)
        },
        [command, request, source] if command == "compile-pptx-chart-geometry" => {
            let request = artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
            let source = artifact::read_limited(source, mo_kernel_api::MAX_INLINE_RESOURCE_BYTES)?;
            mo_kernel_api::compile_pptx_chart_geometry_json(std::str::from_utf8(&request)?, &source)
        },
        [command, request] if command == "compile-chart-geometry" => {
            let request = artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
            mo_kernel_api::compile_chart_geometry_json(std::str::from_utf8(&request)?, &|| false)
        }
        [command, request, source] if command == "pptx-chart-labels" => {
            let request = artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
            let source = artifact::read_limited(source, mo_kernel_api::MAX_INLINE_RESOURCE_BYTES)?;
            mo_kernel_api::compute_pptx_chart_labels_json(std::str::from_utf8(&request)?, &source)
        }
        [command, request, source] if command == "compile-pptx-chart-plot" => {
            let request = artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
            let source = artifact::read_limited(source, mo_kernel_api::MAX_INLINE_RESOURCE_BYTES)?;
            mo_kernel_api::compile_pptx_chart_plot_json(std::str::from_utf8(&request)?, &source)
        }
        [command, request] if command == "layout-chart-sectors" => {
            let request = artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
            mo_kernel_api::layout_chart_sectors_json(std::str::from_utf8(&request)?, &|| false)
        },
        [command, request, source] if command == "pptx-images" => {
            let request = artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
            let source = artifact::read_limited(source, mo_kernel_api::MAX_INLINE_RESOURCE_BYTES)?;
            mo_kernel_api::inspect_pptx_images_json(std::str::from_utf8(&request)?, &source)
        },
        [command, request, source] if command == "layout-pptx-radial" => {
            let request = artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
            let source = artifact::read_limited(source, mo_kernel_api::MAX_INLINE_RESOURCE_BYTES)?;
            mo_kernel_api::layout_pptx_radial_json(std::str::from_utf8(&request)?, &source)
        },
        [command, request, output] if command == "render-paths" => raster::render(request, output, raster::Mode::Paths)?,
        [command, request, output] if command == "render-scene" => raster::render(request, output, raster::Mode::Scene)?,
        [command, request, source, fonts, output] if command == "render-pptx-resource-page" => raster::render(request, output, raster::Mode::SourceResourcePage(source, fonts))?,
        [command, request, source, fonts, output] if command == "render-pptx-playback-page" => raster::render(request, output, raster::Mode::SourcePlaybackPage(source, fonts))?,
        [command, request, source, fonts, output] if command == "render-pptx-text-page" => raster::render(request, output, raster::Mode::SourceTextPage(source, fonts))?,
        [command, request, source, output] if command == "render-pptx-page" => raster::render(request, output, raster::Mode::SourcePage(source))?,
        [command, request, output] if command == "render-page" => raster::render(request, output, raster::Mode::Page)?,
        [command, request_path] if command == "compile-page" => {
            let request = artifact::read_limited(request_path, mo_kernel_api::MAX_REQUEST_BYTES)?;
            mo_kernel_api::compile_page_json(std::str::from_utf8(&request)?, &|| false)
        },
        [] => compute()?,
        [command, request_path] if command == "page-placements" => {
            let request = artifact::read_limited(request_path, mo_kernel_api::MAX_REQUEST_BYTES)?;
            mo_kernel_api::page_placements_json(std::str::from_utf8(&request)?, &|| false)
        },
        [command, request_path] if command == "text-analyze" || command == "bidi-analyze" || command == "itemize-paragraph" || command == "line-break-analyze" => {
            let request = artifact::read_limited(request_path, mo_kernel_api::MAX_REQUEST_BYTES)?;
            let execute = match command.to_str() { Some("line-break-analyze") => mo_kernel_api::analyze_line_breaks_json, Some("bidi-analyze") => mo_kernel_api::analyze_bidi_json, Some("itemize-paragraph") => mo_kernel_api::itemize_paragraph_json, _ => mo_kernel_api::analyze_text_json };
            execute(std::str::from_utf8(&request)?)
        },
        [command, request_path, font_path] if command == "shape-text" || command == "shape-cascade" || command == "shape-paragraph" || command == "font-carets" || command == "font-metrics" || command == "font-outlines" || command == "shape-lines" || command == "layout-lines" || command == "layout-paragraph" || command == "paragraph-paths" => {
            let request = artifact::read_limited(request_path, mo_kernel_api::MAX_REQUEST_BYTES)?;
            let bytes = artifact::read_limited(font_path, mo_kernel_api::MAX_INLINE_FONT_BYTES)?;
            text::shape(request,bytes,match command.to_str() { Some("paragraph-paths") => text::Mode::Paths, Some("layout-paragraph") => text::Mode::Layout, Some("layout-lines") => text::Mode::Geometry, Some("font-outlines") => text::Mode::Outlines, Some("font-carets") => text::Mode::Carets, Some("font-metrics") => text::Mode::Metrics, Some("shape-lines") => text::Mode::Lines, Some("shape-cascade") => text::Mode::Cascade, Some("shape-paragraph") => text::Mode::Paragraph, _ => text::Mode::Text })?
        },
        [command, request_path, font_path] if command == "font-inspect" => {
            let request = artifact::read_limited(request_path, mo_kernel_api::MAX_REQUEST_BYTES)?;
            let bytes = artifact::read_limited(font_path, mo_kernel_api::MAX_INLINE_FONT_BYTES)?;
            mo_kernel_api::inspect_font_json(std::str::from_utf8(&request)?, &bytes)
        },
        [command, request_path, source_path] if command == "pptx-text-bodies" || command == "pptx-colors" || command == "pptx-fills" || command == "pptx-fill-colors" || command == "pptx-table-borders" || command == "pptx-lines" || command == "pptx-line-colors" || command == "pptx-geometry" || command == "pptx-paths" || command == "pptx-placements" || command == "compile-pptx-page" => {
            let request = artifact::read_limited(request_path, mo_kernel_api::MAX_REQUEST_BYTES)?;
            let source = artifact::read_limited(source_path, mo_kernel_api::MAX_INLINE_RESOURCE_BYTES)?;
            let execute = match command.to_str() { Some("pptx-text-bodies") => mo_kernel_api::resolve_pptx_text_bodies_json, Some("compile-pptx-page") => mo_kernel_api::compile_pptx_page_json, Some("pptx-placements") => mo_kernel_api::place_pptx_objects_json, Some("pptx-paths") => mo_kernel_api::compile_pptx_paths_json, Some("pptx-geometry") => mo_kernel_api::evaluate_pptx_geometry_json, Some("pptx-fill-colors") => mo_kernel_api::resolve_pptx_fill_colors_json, Some("pptx-table-borders") => mo_kernel_api::resolve_pptx_table_borders_json, Some("pptx-fills") => mo_kernel_api::resolve_pptx_fills_json, Some("pptx-lines") => mo_kernel_api::resolve_pptx_lines_json, Some("pptx-line-colors") => mo_kernel_api::resolve_pptx_line_colors_json, _ => mo_kernel_api::resolve_pptx_colors_json };
            execute(std::str::from_utf8(&request)?, &source)
        },
        [command, request_path, bundle_path, output_path] if command == "pptx-export" => {
            pptx::export(request_path,bundle_path,output_path)?
        },
        [command, request_path, source_path, output_path] if command == "pptx-edit-text" || command == "pptx-edit-transforms" => {
            let mode = if command == "pptx-edit-transforms" { pptx::EditMode::Transforms } else { pptx::EditMode::Text };
            pptx::edit(request_path, source_path, output_path, mode)?
        },
        [command, path] if command == "pptx-inspect" => {
            let file = std::fs::File::open(path)?;
            let metadata = file.metadata()?;
            if !metadata.is_file() { return Err("input must be a regular file".into()); }
            mo_kernel_api::pptx_source_response_json(&mo_kernel_api::inspect_pptx(
                rawzip::FileReader::from(file), metadata.len(), mo_kernel_api::SourceLimits::default(), &|| false,
            ))
        },
        [command, path] if command == "opc-inspect" => {
            let file = std::fs::File::open(path)?;
            let metadata = file.metadata()?;
            if !metadata.is_file() { return Err("input must be a regular file".into()); }
            let response = mo_kernel_api::inspect_package(
                rawzip::FileReader::from(file), metadata.len(), mo_opc::PackageLimits::default(), &|| false,
            );
            mo_kernel_api::package_response_json(&response)
        },
        _ => return Err("usage: mo-cli opc-repair-inspect <package> | mo-cli opc-repair-apply <package> <reviewed-source-sha256> <new-output> | mo-cli compute <invocation.json> <inputs.json> <protected-spool-directory> <new-output-directory> [worker worker-sha256] | mo-cli compute-schema <schema-id> | mo-cli template <request.json> | mo-cli < request.json | mo-cli pptx-import <request.json> <source.pptx> | mo-cli delivery-inspect <request.json> <contents.bin> | mo-cli delivery-playback <request.json> <contents.bin> | mo-cli compile-playback-page <request.json> | mo-cli render-playback-page <request.json> <new-output.rgba> | mo-cli evaluate-timeline <request.json> | mo-cli pptx-timing <request.json> <source.pptx> | mo-cli layout-pptx-radial <request.json> <source.pptx> | mo-cli compile-pptx-chart-plot <request.json> <source.pptx> | mo-cli compile-pptx-chart-geometry <request.json> <source.pptx> | mo-cli compile-chart-geometry <request.json> | mo-cli layout-chart-sectors <request.json> | mo-cli pptx-chart-labels <request.json> <source.pptx> | mo-cli pptx-chart-paints <request.json> <source.pptx> | mo-cli pptx-charts <request.json> <source.pptx> | mo-cli pptx-images <request.json> <source.pptx> | mo-cli pptx-extract-images <request.json> <source.pptx> <new-resources.bin> | mo-cli render-pptx-playback-page <request.json> <source.pptx> <fonts.bin> <new-output.rgba> | mo-cli render-pptx-resource-page <request.json> <source.pptx> <fonts.bin> <new-output.rgba> | mo-cli render-pptx-text-page <request.json> <source.pptx> <fonts.bin> <new-output.rgba> | mo-cli compile-pptx-page <request.json> <source.pptx> | mo-cli render-pptx-page <request.json> <source.pptx> <new-output.rgba> | mo-cli compile-page <request.json> | mo-cli render-page <request.json> <new-output.rgba> | mo-cli page-placements <request.json> | mo-cli render-scene <request.json> <new-output.rgba> | mo-cli render-paths <request.json> <new-output.rgba> | mo-cli paragraph-paths <request.json> <fonts.bin> | mo-cli layout-paragraph <request.json> <fonts.bin> | mo-cli layout-lines <request.json> <fonts.bin> | mo-cli shape-lines <request.json> <fonts.bin> | mo-cli font-outlines <request.json> <font.ttf/otf/ttc> | mo-cli font-carets <request.json> <font.ttf/otf/ttc> | mo-cli font-metrics <request.json> <font.ttf/otf/ttc> | mo-cli line-break-analyze <request.json> | mo-cli itemize-paragraph <request.json> | mo-cli shape-paragraph <request.json> <fonts.bin> | mo-cli text-analyze <request.json> | mo-cli bidi-analyze <request.json> | mo-cli shape-text <request.json> <font.ttf/otf/ttc> | mo-cli shape-cascade <request.json> <fonts.bin> | mo-cli font-inspect <request.json> <font.ttf/otf/ttc> | mo-cli opc-inspect <package> | mo-cli pptx-inspect <pptx> | mo-cli pptx-table-borders <request.json> <source.pptx> | mo-cli pptx-fill-colors <request.json> <source.pptx> | mo-cli pptx-fills <request.json> <source.pptx> | mo-cli pptx-text-bodies <request.json> <source.pptx> | mo-cli pptx-lines <request.json> <source.pptx> | mo-cli pptx-line-colors <request.json> <source.pptx> | mo-cli pptx-geometry <request.json> <source.pptx> | mo-cli pptx-placements <request.json> <source.pptx> | mo-cli pptx-paths <request.json> <source.pptx> | mo-cli pptx-colors <request.json> <source.pptx> | mo-cli pptx-export <request.json> <resources.bin> <new-output.pptx> | mo-cli pptx-edit-transforms <edits.json> <source.pptx> <new-output.pptx> | mo-cli pptx-edit-text <edits.json> <source.pptx> <new-output.pptx>".into()),
    };
    let mut stdout = io::stdout().lock();
    stdout.write_all(output.as_bytes())?;
    stdout.write_all(b"\n")?;
    Ok(())
}

fn compute() -> Result<String, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((mo_kernel_api::MAX_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    let input = std::str::from_utf8(&bytes)?;
    Ok(mo_kernel_api::dispatch_json(input))
}
