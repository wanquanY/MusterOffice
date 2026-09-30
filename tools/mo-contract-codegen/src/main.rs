use mo_kernel_api::{BidiAnalysisRequest, BidiAnalysisResponse};
use mo_kernel_api::{CascadeRequest, CascadeResponse, TextAnalysisRequest, TextAnalysisResponse};
use mo_kernel_api::{
    ChartGeometryRequest, ChartGeometryResponse, ChartSectorsResponse, SectorRequest,
};
use mo_kernel_api::{FontMetricsRequest, FontMetricsResponse};
use mo_kernel_api::{FontOutlinesRequest, FontOutlinesResponse};
use mo_kernel_api::{
    FontRequest, FontResponse, KernelRequest, KernelResponse, PackageInspectionResponse,
    PptxColorResponse, PptxExportRequest, PptxSourceResponse, ShapeRequest, ShapeResponse,
    SourceColorQuery, SourceTextEdits, SourceTransformEdits,
};
use mo_kernel_api::{ImageDecodeRequest, ImageDecodeResponse};
use mo_kernel_api::{ImageRasterRequest, ImageRasterResponse};
use mo_kernel_api::{ImageSceneRasterRequest, ImageSceneRasterResponse};
use mo_kernel_api::{
    ItemizationRequest, ItemizationResponse, LineShapeRequest, LineShapeResponse,
    ParagraphShapeRequest, ParagraphShapeResponse,
};
use mo_kernel_api::{LineBreakRequest, LineBreakResponse};
use mo_kernel_api::{LineGeometryRequest, LineGeometryResponse};
use mo_kernel_api::{PageCompileResponse, PageRasterResponse, PageRenderRequest};
use mo_kernel_api::{PagePlacementRequest, PagePlacementResponse};
use mo_kernel_api::{ParagraphLayoutRequest, ParagraphLayoutResponse};
use mo_kernel_api::{ParagraphPathsRequest, ParagraphPathsResponse};
use mo_kernel_api::{PathRasterRequest, PathRasterResponse};
use mo_kernel_api::{PptxChartGeometryResponse, SourceCircularRequest};
use mo_kernel_api::{PptxChartsResponse, PptxImagesResponse, SourceChartQuery, SourceImageQuery};
use mo_kernel_api::{
    PptxFillColorResponse, PptxFillResponse, SourceFillColorQuery, SourceFillQuery,
};
use mo_kernel_api::{PptxGeometryResponse, SourceGeometryQuery};
use mo_kernel_api::{
    PptxLineColorResponse, PptxLineResponse, SourceLineColorQuery, SourceLineQuery,
};
use mo_kernel_api::{PptxPageCompileResponse, PptxPageRasterResponse, SourcePageRequest};
use mo_kernel_api::{PptxPathsResponse, SourceNativePathsQuery};
use mo_kernel_api::{PptxPlacementResponse, SourcePlacementQuery};
use mo_kernel_api::{PptxRadialLayoutRequest, PptxRadialLayoutResponse};
use mo_kernel_api::{
    PptxResourcePageRasterResponse, PptxResourcePageRequest, PptxTextPageRasterResponse,
    PptxTextPageRequest,
};
use mo_kernel_api::{PptxTextBodyResponse, SourceTextBodyQuery};
use mo_kernel_api::{SceneRasterRequest, SceneRasterResponse};
use mo_presentation_edit::{Transaction, TransactionReceipt};
use mo_presentation_model::{Document, ValidationReport};
use schemars::{JsonSchema, Schema, schema_for};
use std::{error::Error, fs, path::Path};

fn schema<T: JsonSchema>() -> Schema {
    schema_for!(T)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mode = args
        .next()
        .ok_or("usage: mo-contract-codegen <write|check> <directory>")?;
    let path = args.next().ok_or("missing output directory")?;
    if args.next().is_some() || !matches!(mode.as_str(), "write" | "check") {
        return Err("usage: mo-contract-codegen <write|check> <directory>".into());
    }
    let schemas = [
        (
            "template-definition",
            schema::<mo_presentation_template::TemplateDefinition>(),
        ),
        (
            "template-request",
            schema::<mo_presentation_template::TemplateRequest>(),
        ),
        (
            "template-response",
            schema::<mo_presentation_template::TemplateResponse>(),
        ),
        (
            "computation-invocation",
            schema::<mo_presentation_operations::Invocation>(),
        ),
        (
            "computation-receipt",
            schema::<mo_presentation_operations::ComputationReceipt>(),
        ),
        (
            "computation-request",
            schema::<mo_presentation_operations::OperationRequest>(),
        ),
        (
            "computation-failure",
            schema::<mo_presentation_operations::Failure>(),
        ),
        (
            "computation-mutation-receipt",
            schema::<mo_presentation_operations::MutationReceipt>(),
        ),
        (
            "computation-export-receipt",
            schema::<mo_presentation_operations::ExportReceipt>(),
        ),
        (
            "delivery-inspect-request",
            schema::<mo_kernel_api::DeliveryInspectRequest>(),
        ),
        (
            "delivery-inspect-response",
            schema::<mo_kernel_api::DeliveryInspectResponse>(),
        ),
        (
            "delivery-playback-request",
            schema::<mo_kernel_api::DeliveryPlaybackRequest>(),
        ),
        (
            "delivery-playback-response",
            schema::<mo_kernel_api::DeliveryPlaybackResponse>(),
        ),
        (
            "host-capabilities",
            schema::<mo_operation_service::HostCapabilities>(),
        ),
        (
            "schema-document",
            schema::<mo_operation_service::SchemaDocument>(),
        ),
        (
            "presentation-delivery-settings",
            schema::<mo_presentation_delivery::DeliverySettings>(),
        ),
        (
            "presentation-delivery-bundle",
            schema::<mo_presentation_delivery::DeliveryBundle>(),
        ),
        (
            "presentation-renderer-identity",
            schema::<mo_presentation_delivery::RendererIdentity>(),
        ),
        ("asset-info", schema::<mo_operation_service::AssetInfo>()),
        (
            "asset-binding",
            schema::<mo_operation_service::AssetBinding>(),
        ),
        (
            "upload-request",
            schema::<mo_operation_service::UploadRequest>(),
        ),
        ("upload-info", schema::<mo_operation_service::UploadInfo>()),
        (
            "host-request",
            schema::<mo_operation_service::HostRequest>(),
        ),
        (
            "host-response",
            schema::<mo_operation_service::HostResponse>(),
        ),
        (
            "operation-request",
            schema::<mo_operation_service::OperationRequest>(),
        ),
        ("operation-job", schema::<mo_operation_service::JobInfo>()),
        (
            "pptx-playback-page-request",
            schema::<mo_kernel_api::PptxPlaybackPageRequest>(),
        ),
        (
            "pptx-playback-raster-response",
            schema::<mo_kernel_api::PptxPlaybackRasterResponse>(),
        ),
        (
            "pptx-playback-session-request",
            schema::<mo_kernel_api::PptxPlaybackSessionRequest>(),
        ),
        (
            "pptx-playback-session-response",
            schema::<mo_kernel_api::PptxPlaybackSessionResponse>(),
        ),
        (
            "playback-session-request",
            schema::<mo_kernel_api::PlaybackSessionRequest>(),
        ),
        (
            "playback-session-response",
            schema::<mo_kernel_api::PlaybackSessionResponse>(),
        ),
        (
            "playback-page-request",
            schema::<mo_kernel_api::PlaybackPageRequest>(),
        ),
        (
            "playback-compile-response",
            schema::<mo_kernel_api::PlaybackCompileResponse>(),
        ),
        (
            "playback-raster-response",
            schema::<mo_kernel_api::PlaybackRasterResponse>(),
        ),
        (
            "timeline-evaluate-request",
            schema::<mo_kernel_api::TimelineEvaluateRequest>(),
        ),
        (
            "timeline-evaluate-response",
            schema::<mo_kernel_api::TimelineEvaluateResponse>(),
        ),
        (
            "pptx-timing-query",
            schema::<mo_kernel_api::SourceTimingQuery>(),
        ),
        (
            "pptx-timing-response",
            schema::<mo_kernel_api::PptxTimingResponse>(),
        ),
        (
            "pptx-radial-layout-request",
            schema::<PptxRadialLayoutRequest>(),
        ),
        (
            "pptx-radial-layout-response",
            schema::<PptxRadialLayoutResponse>(),
        ),
        ("image-decode-request", schema::<ImageDecodeRequest>()),
        ("image-decode-response", schema::<ImageDecodeResponse>()),
        ("image-scene-request", schema::<ImageSceneRasterRequest>()),
        ("image-scene-response", schema::<ImageSceneRasterResponse>()),
        ("pptx-chart-query", schema::<SourceChartQuery>()),
        ("pptx-chart-response", schema::<PptxChartsResponse>()),
        ("chart-sector-request", schema::<SectorRequest>()),
        ("chart-sector-response", schema::<ChartSectorsResponse>()),
        ("chart-geometry-request", schema::<ChartGeometryRequest>()),
        ("chart-geometry-response", schema::<ChartGeometryResponse>()),
        (
            "pptx-chart-geometry-request",
            schema::<SourceCircularRequest>(),
        ),
        (
            "pptx-chart-geometry-response",
            schema::<PptxChartGeometryResponse>(),
        ),
        ("pptx-image-query", schema::<SourceImageQuery>()),
        ("pptx-image-response", schema::<PptxImagesResponse>()),
        ("image-raster-request", schema::<ImageRasterRequest>()),
        ("image-raster-response", schema::<ImageRasterResponse>()),
        (
            "pptx-resource-page-request",
            schema::<PptxResourcePageRequest>(),
        ),
        (
            "pptx-resource-page-raster-response",
            schema::<PptxResourcePageRasterResponse>(),
        ),
        ("pptx-text-page-request", schema::<PptxTextPageRequest>()),
        (
            "pptx-text-page-raster-response",
            schema::<PptxTextPageRasterResponse>(),
        ),
        ("pptx-page-request", schema::<SourcePageRequest>()),
        (
            "pptx-page-compile-response",
            schema::<PptxPageCompileResponse>(),
        ),
        (
            "pptx-page-raster-response",
            schema::<PptxPageRasterResponse>(),
        ),
        ("page-render-request", schema::<PageRenderRequest>()),
        ("page-compile-response", schema::<PageCompileResponse>()),
        ("page-raster-response", schema::<PageRasterResponse>()),
        ("page-placement-request", schema::<PagePlacementRequest>()),
        ("page-placement-response", schema::<PagePlacementResponse>()),
        ("path-raster-request", schema::<PathRasterRequest>()),
        ("path-raster-response", schema::<PathRasterResponse>()),
        ("scene-raster-request", schema::<SceneRasterRequest>()),
        ("scene-raster-response", schema::<SceneRasterResponse>()),
        ("paragraph-paths-request", schema::<ParagraphPathsRequest>()),
        (
            "paragraph-paths-response",
            schema::<ParagraphPathsResponse>(),
        ),
        (
            "paragraph-layout-request",
            schema::<ParagraphLayoutRequest>(),
        ),
        (
            "paragraph-layout-response",
            schema::<ParagraphLayoutResponse>(),
        ),
        ("line-geometry-request", schema::<LineGeometryRequest>()),
        ("line-geometry-response", schema::<LineGeometryResponse>()),
        ("font-outlines-request", schema::<FontOutlinesRequest>()),
        ("font-outlines-response", schema::<FontOutlinesResponse>()),
        ("font-metrics-request", schema::<FontMetricsRequest>()),
        ("font-metrics-response", schema::<FontMetricsResponse>()),
        ("line-break-request", schema::<LineBreakRequest>()),
        ("line-break-response", schema::<LineBreakResponse>()),
        ("itemization-request", schema::<ItemizationRequest>()),
        ("itemization-response", schema::<ItemizationResponse>()),
        ("line-shape-request", schema::<LineShapeRequest>()),
        ("line-shape-response", schema::<LineShapeResponse>()),
        ("paragraph-shape-request", schema::<ParagraphShapeRequest>()),
        (
            "paragraph-shape-response",
            schema::<ParagraphShapeResponse>(),
        ),
        ("bidi-analysis-request", schema::<BidiAnalysisRequest>()),
        ("bidi-analysis-response", schema::<BidiAnalysisResponse>()),
        ("cascade-request", schema::<CascadeRequest>()),
        ("cascade-response", schema::<CascadeResponse>()),
        ("text-analysis-request", schema::<TextAnalysisRequest>()),
        ("text-analysis-response", schema::<TextAnalysisResponse>()),
        ("shape-request", schema::<ShapeRequest>()),
        ("shape-response", schema::<ShapeResponse>()),
        ("font-request", schema::<FontRequest>()),
        ("font-response", schema::<FontResponse>()),
        ("kernel-request", schema::<KernelRequest>()),
        ("kernel-response", schema::<KernelResponse>()),
        ("document", schema::<Document>()),
        ("transaction", schema::<Transaction>()),
        ("transaction-receipt", schema::<TransactionReceipt>()),
        ("validation-report", schema::<ValidationReport>()),
        ("package-inspection", schema::<PackageInspectionResponse>()),
        ("pptx-export-request", schema::<PptxExportRequest>()),
        (
            "pptx-import-request",
            schema::<mo_kernel_api::PptxImportRequest>(),
        ),
        (
            "pptx-import-response",
            schema::<mo_kernel_api::PptxImportResponse>(),
        ),
        ("pptx-source-response", schema::<PptxSourceResponse>()),
        ("pptx-text-edits", schema::<SourceTextEdits>()),
        ("pptx-transform-edits", schema::<SourceTransformEdits>()),
        ("pptx-fill-color-query", schema::<SourceFillColorQuery>()),
        (
            "pptx-table-border-query",
            schema::<mo_kernel_api::SourceTableBorderQuery>(),
        ),
        (
            "pptx-table-border-response",
            schema::<mo_kernel_api::PptxTableBorderResponse>(),
        ),
        (
            "pptx-fill-color-response",
            schema::<PptxFillColorResponse>(),
        ),
        ("pptx-fill-query", schema::<SourceFillQuery>()),
        ("pptx-fill-response", schema::<PptxFillResponse>()),
        ("pptx-line-query", schema::<SourceLineQuery>()),
        ("pptx-text-body-query", schema::<SourceTextBodyQuery>()),
        ("pptx-text-body-response", schema::<PptxTextBodyResponse>()),
        ("pptx-geometry-query", schema::<SourceGeometryQuery>()),
        ("pptx-geometry-response", schema::<PptxGeometryResponse>()),
        ("pptx-paths-query", schema::<SourceNativePathsQuery>()),
        ("pptx-paths-response", schema::<PptxPathsResponse>()),
        ("pptx-placement-query", schema::<SourcePlacementQuery>()),
        ("pptx-placement-response", schema::<PptxPlacementResponse>()),
        ("pptx-line-color-query", schema::<SourceLineColorQuery>()),
        (
            "pptx-line-color-response",
            schema::<PptxLineColorResponse>(),
        ),
        ("pptx-line-response", schema::<PptxLineResponse>()),
        ("pptx-color-query", schema::<SourceColorQuery>()),
        ("pptx-color-response", schema::<PptxColorResponse>()),
    ];
    let directory = Path::new(&path);
    if mode == "write" {
        fs::create_dir_all(directory)?;
    }
    for (name, schema) in schemas {
        let schema = mo_common::runtime_schema(name, schema);
        let content = format!("{}\n", serde_json::to_string_pretty(&schema)?);
        let file = directory.join(format!("{name}.schema.json"));
        if mode == "write" {
            fs::write(&file, content)?;
        } else if fs::read_to_string(&file)? != content {
            return Err(format!("generated schema differs: {}", file.display()).into());
        }
        println!("{} {}", mode, file.display());
    }
    Ok(())
}
