#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
// Device-space path raster component, ABI 4. See gradient-raster.md for the wire
// grammar and limits. No document layout, font lookup, file or network access.
// Returned packed RGBA8 premultiplied sRGB pixels belong to this component.
// 0 success, 1 invalid input, 2 allocation failure, 3 budget, 4 instance invalid.
// Some upstream allocation failures trap/abort: the host MUST isolate this
// component and discard the process/module on any trap or allocation failure.
// Output fields stay zero until the complete frame has been drawn.
int32_t mo_skia_raster(const uint32_t *request, uint32_t words,
                       uint8_t **pixels, uint32_t *byte_length);
// Image extension 2 accepts version-5 whole-image and version-6 source-domain
// frames plus a borrowed immutable RGBA bundle. No references to the bundle
// survive this synchronous call. Same ownership/isolation as mo_skia_raster.
int32_t mo_skia_raster_images(const uint32_t *request, uint32_t words,
    const uint8_t *images, uint32_t image_bytes, uint8_t **pixels, uint32_t *byte_length);
uint32_t mo_skia_images_abi(void);
// Optional shared path clipping extension 1. Both entry points accept V7; the
// plain entry requires zero image resources. Existing V4/V5/V6 remain accepted.
uint32_t mo_skia_clips_abi(void);
// Optional composition extension 1: V8 adds bounded immutable draw-prefix
// captures and explicit Source/SourceOver blending. Prior formats unchanged.
uint32_t mo_skia_compositing_abi(void);
uint32_t mo_skia_gradient_planes_abi(void);
// V10 retains V9 grammar and adds explicit Office 15/8 channel interpolation.
uint32_t mo_skia_office_gradients_abi(void);
// V11 adds rectangular scalar fields while keeping prior geometry and ramps.
uint32_t mo_skia_rect_gradients_abi(void);
// V13 adds isolated opacity intervals (first draw, exclusive end, alpha/65535).
// Transparent surfaces have explicit shared memory and pixel-work limits.
uint32_t mo_skia_opacity_groups_abi(void);
// V14 keeps the V13 header and replaces each snapshot word with
// (draw prefix, canvas scope): 0 is output, positive scope is group index + 1.
// Captures occur after closing/opening groups at the prefix. The chosen group
// must be active there; immutable captured pixels may be used after it closes.
uint32_t mo_skia_snapshot_scopes_abi(void);
// V12 adds bounded elliptic fields. Status 3 also covers numerical precision
// exhaustion. All failures retain zero output ownership; instance stays valid.
uint32_t mo_skia_elliptic_gradients_abi(void);
// Optional retained execution extension 1. Borrow request/images immutably until
// drop; they must remain alive and cannot alias output. Handles are single-owner
// host capabilities, never serialized or shared between component instances.
// begin validates the whole grammar before allocating a task. step performs at
// most work_units state transitions (one paint, <=4096 path commands,
// <=65536 pixels of image normalization/clear/snapshot copy, allocation,
// draw setup or a retained Skia scan interval/row). Work units are NOT time
// guarantees. Filled paths retain edges, AA accumulators and blitters across
// steps. Geometry/stroke preparation, a complex scan interval, hairlines, clips,
// small mask commits, validation, allocations and input/output transfers can
// still be synchronous. No path re-clipping or partial output is used to yield.
typedef struct MoSkiaRasterTask MoSkiaRasterTask;
uint32_t mo_skia_execution_abi(void);
int32_t mo_skia_raster_begin(const uint32_t* request, uint32_t words,
    const uint8_t* images, uint32_t image_bytes, uint32_t with_images, MoSkiaRasterTask** task);
int32_t mo_skia_raster_step(MoSkiaRasterTask* task, uint32_t work_units, uint32_t* complete);
// Only a completed successful task can transfer its pixels, exactly once.
int32_t mo_skia_raster_take(MoSkiaRasterTask* task, uint8_t** pixels, uint32_t* byte_length);
// Cancel or release. No pixel ownership escapes; null is permitted. An invalid
// component must instead be quarantined by terminating its process/Worker.
void mo_skia_raster_drop(MoSkiaRasterTask* task);
// Geometry-only point queries; returned bitmaps use the shared allocator.
uint32_t mo_skia_pick_abi(void);
int32_t mo_skia_pick(const uint32_t* request, uint32_t words, uint32_t** output, uint32_t* output_words);
void mo_skia_free(void *pixels);
uint32_t mo_skia_abi(void);
#ifdef __cplusplus
}
#endif
