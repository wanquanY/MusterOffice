#include "mo_skia.h"
#include "mo_clip.h"
#include "mo_composite.h"
#include "mo_miter_clip.h"
#include "mo_gradient.h"
#include "mo_image.h"
#include "mo_pixel_work.h"
#include "mo_opacity.h"
#include "include/core/SkCanvas.h"
#include "include/core/SkColorSpace.h"
#include "include/core/SkImageInfo.h"
#include "include/core/SkPaint.h"
#include "include/core/SkPath.h"
#include "include/core/SkPathBuilder.h"
#include "include/core/SkSurface.h"
#include "src/core/SkBitmapDevice.h"
#include "src/core/SkCanvasPriv.h"
#include <atomic>
#include <algorithm>
#include <bit>
#include <cmath>
#include <cstdlib>
#include <memory>
#include <vector>

namespace {
constexpr uint32_t magic = 0x4d4f534b;
constexpr uint32_t max_paths = 4096, max_draws = 65536;
constexpr uint32_t max_commands = 262144, max_draw_work = 1048576;
constexpr uint32_t max_pixels = 16 * 1024 * 1024;
constexpr uint32_t max_words_v4 = 10 + max_paths * 2 + max_commands * 7 + max_paths * 4 + max_draws * 6 + 4096 * 9 + 65536 * 5;
constexpr uint32_t max_words_v5 = max_words_v4 + 2 + 4096 * 14;
constexpr uint32_t max_words_v6 = max_words_v5 + 4096 * 4;
constexpr uint32_t max_words_v7 = max_words_v6 + 1 + 8192 * 4 + max_draws;
constexpr uint32_t max_words_v8 = max_words_v7 + 1 + 64 + max_draws;
constexpr uint32_t max_words_v9 = max_words_v8 + 4096 * 7;
constexpr uint32_t max_words_v11 = max_words_v9 + 4096;
constexpr uint32_t max_words_v12 = max_words_v11 + 4096*2;
constexpr uint32_t max_words_v13 = max_words_v12 + 1 + 4096*3;
constexpr uint32_t max_words_v14 = max_words_v13 + 64;
std::atomic<bool> invalid{false};
float scalar(uint32_t word) { return std::bit_cast<float>(word); }
bool coordinate(uint32_t word) {
    const float value = scalar(word);
    return std::isfinite(value) && std::abs(value) <= 32768.0f;
}
SkColor color(uint32_t rgba) {
    return SkColorSetARGB(rgba >> 24, rgba & 255, (rgba >> 8) & 255, (rgba >> 16) & 255);
}
// Fixed arrays validate the entire grammar before allocating or calling Skia.
struct Plan {
    uint32_t offsets[max_paths];
    uint32_t counts[max_paths];
    float bounds[max_paths][4];
    uint32_t draws;
    uint32_t strokes;
    uint32_t gradients[max_paths];
    uint32_t images = 0, brushes = 0;
    uint32_t image_count = 0, brush_count = 0;
    uint32_t brush_words = 10;
    uint32_t clips = 0, clip_count = 0, draw_words = 6;
    uint32_t snapshots = 0, snapshot_count = 0, snapshot_words = 1;
    uint32_t groups = 0, group_count = 0;
};
int validate(const uint32_t* r, uint32_t words, const uint8_t* data,
             uint32_t bytes, bool with_images, Plan& plan) {
    if (!r || words < 10 || words > max_words_v14 || r[0] != magic) return 1;
    const bool scoped_snapshots = r[1] == 14;
    const bool opacity = r[1] == 13 || scoped_snapshots;
    plan.snapshot_words = scoped_snapshots ? 2 : 1;
    const bool elliptic = r[1] == 12 || opacity;
    const bool rectangular = r[1] == 11 || elliptic;
    const bool office = r[1] == 10 || rectangular;
    const bool planes = r[1] == 9 || office;
    const bool composite = r[1] == 8 || planes;
    const bool clipping = r[1] == 7 || composite;
    const uint32_t header = opacity ? 15 : composite ? 14 : clipping ? 13 : with_images ? 12 : 10;
    if (words < header || words > (scoped_snapshots ? max_words_v14 : opacity ? max_words_v13 : elliptic ? max_words_v12 : rectangular ? max_words_v11 : planes ? max_words_v9 : composite ? max_words_v8 : clipping ? max_words_v7 : with_images ? max_words_v6 : max_words_v4) ||
        (!clipping && (with_images ? (r[1] != 5 && r[1] != 6) : r[1] != 4))) return 1;
    if (with_images || clipping) {
        plan.brush_words = r[1] >= 6 ? 14 : 10;
        plan.image_count = r[10]; plan.brush_count = r[11];
        if (r[10] > 4096 || r[11] > 4096) return 3;
        if (!with_images && (r[10] || r[11])) return 1;
    }
    if (clipping) {
        plan.clip_count = r[12]; plan.draw_words = 7;
        if (r[12] > 8192) return 3;
    }
    if (composite) {
        plan.snapshot_count = r[13]; plan.draw_words = 8;
        if (r[13] > 64) return 3;
    }
    if (opacity) { plan.group_count = r[14]; if (!r[14] || r[14] > 4096) return 3; }
    if (!r[2] || !r[3]) return 1;
    if (r[2] > 8192 || r[3] > 8192 || uint64_t(r[2]) * r[3] > max_pixels ||
        r[5] > max_paths || r[6] > max_draws || r[7] > max_commands || r[8] > max_paths || r[9] > max_paths) return 3;
    if (uint64_t(r[2]) * r[3] * 4 * plan.snapshot_count > 64 * 1024 * 1024) return 3;
    uint32_t pos = header, commands = 0;
    for (uint32_t i = 0; i < r[5]; ++i) {
        if (words - pos < 2 || r[pos] > 1) return 1;
        const uint32_t count = r[pos + 1];
        if (count > max_commands - commands) return 3;
        commands += count;
        plan.offsets[i] = pos;
        plan.counts[i] = count;
        pos += 2;
        if (count > (words - pos) / 7) return 1;
        bool contour = false;
        for (uint32_t j = 0; j < count; ++j, pos += 7) {
            const uint32_t op = r[pos];
            if (op < 1 || op > 5 || (op != 1 && !contour)) return 1;
            const uint32_t values = op <= 2 ? 2 : op == 3 ? 4 : op == 4 ? 6 : 0;
            for (uint32_t k = 0; k < 6; ++k) {
                if (k < values ? !coordinate(r[pos + 1 + k]) : r[pos + 1 + k] != 0) return 1;
            }
            for (uint32_t k = 0; k < values; k += 2) {
                const float x = scalar(r[pos + 1 + k]), y = scalar(r[pos + 2 + k]);
                plan.bounds[i][0] = std::min(plan.bounds[i][0], x);
                plan.bounds[i][1] = std::min(plan.bounds[i][1], y);
                plan.bounds[i][2] = std::max(plan.bounds[i][2], x);
                plan.bounds[i][3] = std::max(plan.bounds[i][3], y);
            }
            if (op == 1) contour = true;
            if (op == 5) contour = false;
        }
    }
    if (commands != r[7] || uint64_t(pos) + uint64_t(r[8]) * 4 > words) return 1;
    plan.strokes = pos;
    for (uint32_t i = 0; i < r[8]; ++i, pos += 4) {
        const float width = scalar(r[pos]), miter = scalar(r[pos + 3]);
        const uint32_t join = r[pos + 2];
        if (!std::isfinite(width) || width < 0 || width > 32768 || r[pos + 1] > 2 || join > 3 ||
            !std::isfinite(miter) || miter < 0 || miter > 1024 ||
            (join == 3 && miter < 1) || ((join == 1 || join == 2) && r[pos + 3] != 0)) return 1;
    }
    uint32_t stops = 0;
    for (uint32_t i = 0; i < r[9]; ++i) {
        plan.gradients[i] = pos;
        if (const int status = mo_validate_gradient(r, words, pos, stops, planes, office, rectangular, elliptic)) return status;
    }
    if (uint64_t(pos) + uint64_t(plan.image_count)*4 + uint64_t(plan.brush_count)*plan.brush_words +
        uint64_t(plan.clip_count)*4 + plan.snapshot_count*plan.snapshot_words + uint64_t(plan.group_count)*3 + uint64_t(r[6])*plan.draw_words != words) return 1;
    plan.images = pos;
    if (const int status = mo_validate_images(r + pos, plan.image_count, data, bytes)) return status;
    pos += plan.image_count*4;
    plan.brushes = pos;
    for (uint32_t i = 0; i < plan.brush_count; ++i, pos += plan.brush_words) {
        if (const int status = mo_validate_image_brush(r + pos, r + plan.images, plan.image_count)) return status;
        if (plan.brush_words == 14) if (const int status = mo_validate_image_domain(r + pos)) return status;
    }
    plan.clips = pos;
    if (const int status = mo_validate_clips(r + pos, plan.clip_count, r[5], plan.counts, plan.bounds)) return status;
    pos += plan.clip_count * 4;
    plan.snapshots = pos;
    for (uint32_t i = 0; i < plan.snapshot_count; ++i, pos += plan.snapshot_words) {
        if (r[pos] >= r[6]) return 1;
        if (scoped_snapshots && r[pos + 1] > plan.group_count) return 1;
        if (i) {
            const auto* previous = r + pos - plan.snapshot_words;
            if (previous[0] > r[pos] || (previous[0] == r[pos] &&
                (!scoped_snapshots || previous[1] >= r[pos + 1]))) return 1;
        }
    }
    plan.groups = pos;
    uint16_t scopes[max_draws]{};
    if (const int status = mo_validate_opacity(r + pos, plan.group_count, r[6], uint64_t(r[2])*r[3], plan.snapshot_count, scopes)) return status;
    pos += plan.group_count * 3;
    // Explicit immutable captures may be used across groups, but the chosen
    // source canvas must be alive at capture, after all boundary transitions.
    if (scoped_snapshots) for (uint32_t i = 0; i < plan.snapshot_count; ++i) {
        const auto* snapshot = r + plan.snapshots + i * plan.snapshot_words;
        if (snapshot[1]) {
            const auto* g = r + plan.groups + (snapshot[1] - 1) * 3;
            if (snapshot[0] < g[0] || snapshot[0] >= g[1]) return 1;
        }
    }
    plan.draws = pos;
    uint64_t work = 0, clip_work = 0, applications = 0;
    MoClipStack clip_stack;
    for (uint32_t i = 0; i < r[6]; ++i, pos += plan.draw_words) {
        if (clipping) {
            if (i && scopes[i] != scopes[i - 1]) clip_stack.length = 0;
            if (r[pos + 6] > plan.clip_count) return 1;
            const uint32_t common = clip_stack.transition(r + plan.clips, r[pos + 6]);
            for (uint32_t k = common; k < clip_stack.length; ++k) {
                ++applications;
                clip_work += plan.counts[r[plan.clips + (clip_stack.nodes[k] - 1) * 4 + 1]];
                if (applications > 262144 || clip_work > 1048576) return 3;
            }
        }
        if (r[pos] >= r[5] || !coordinate(r[pos + 1]) || !coordinate(r[pos + 2]) || r[pos + 4] > r[8] ||
            r[pos + 5] > r[9] + plan.brush_count + plan.snapshot_count || (r[pos + 5] && r[pos + 3])) return 1;
        if (composite && r[pos + 7] > 1) return 1;
        if (r[pos + 5] > r[9] + plan.brush_count) {
            const auto* snapshot = r + plan.snapshots +
                (r[pos + 5] - r[9] - plan.brush_count - 1) * plan.snapshot_words;
            if (snapshot[0] > i || (!scoped_snapshots && scopes[snapshot[0]] != scopes[i])) return 1;
        }
        double inflation = 0;
        if (r[pos + 4] && plan.counts[r[pos]]) {
            const uint32_t s = plan.strokes + (r[pos + 4] - 1) * 4;
            const double width = scalar(r[s]);
            const uint32_t join = r[s + 2];
            const double join_factor = join == 0 ? double(scalar(r[s + 3])) : join == 3 ? double(scalar(r[s + 3])) + 1 : 1;
            const double factor = std::max(r[s + 1] == 2 ? 2.0 : 1.0, join_factor);
            inflation = width == 0 && join != 3 ? 1.0 : (width == 0 ? 1.0 : width) * factor / 2;
        }
        const auto& bounds = plan.bounds[r[pos]];
        for (uint32_t k = 0; k < 4; ++k) {
            if (std::abs(double(bounds[k]) + scalar(r[pos + 1 + k % 2])) + inflation > 32768.0) return 3;
        }
        work += plan.counts[r[pos]];
        if (work > max_draw_work) return 3;
    }
    return 0;
}
int fail_allocation() {
    invalid.store(true, std::memory_order_release);
    return 2;
}
}

extern "C" uint32_t mo_skia_opacity_groups_abi(void) { return 1; }
extern "C" uint32_t mo_skia_snapshot_scopes_abi(void) { return 1; }
extern "C" uint32_t mo_skia_abi(void) { return 4; }
extern "C" uint32_t mo_skia_clips_abi(void) { return 1; }
extern "C" uint32_t mo_skia_gradient_planes_abi(void) { return 1; }
extern "C" uint32_t mo_skia_office_gradients_abi(void) { return 1; }
extern "C" uint32_t mo_skia_rect_gradients_abi(void) { return 1; }
extern "C" uint32_t mo_skia_elliptic_gradients_abi(void) { return 1; }
extern "C" uint32_t mo_skia_compositing_abi(void) { return 1; }
extern "C" uint32_t mo_skia_images_abi(void) { return 2; }
extern "C" void mo_skia_free(void* pixels) { std::free(pixels); }
// Retained only within one host-owned component instance. No scheduler, clock,
// background work, resource fetch or global task registry lives here.
struct MoSkiaRasterTask {
    const uint32_t* r;
    const uint8_t* data;
    Plan plan;
    MoGradientState gradient_state;
    MoPixelTransfer transfer;
    std::vector<sk_sp<SkShader>> shaders;
    std::vector<sk_sp<SkImage>> images;
    std::unique_ptr<uint8_t, decltype(&std::free)> buffer{nullptr, &std::free};
    sk_sp<SkSurface> surface;
    std::vector<SkPath> paths;
    std::unique_ptr<SkPathBuilder> builder;
    SkImageInfo info;
    SkPaint paint;
    MoClipStack clips;
    MoOpacityStack groups;
    enum Phase { Gradients, Images, Brushes, Surface, Clear, Paths, Draws, Complete, Taken, Failed } phase = Gradients;
    uint32_t index = 0, command = 0, next_snapshot = 0;
    int status = 0;
    bool draw_ready = false;
    bool drawing = false;
    mo::RasterStorage drawing_storage;
    // Destroy suspended scan/blitter frames before their canvas, paint, paths
    // and output buffer. Cancellation cannot leave borrowers of a dead device.
    mo::RasterTask drawing_task;

    MoSkiaRasterTask(const uint32_t* request, const uint8_t* pixels, const Plan& parsed)
        : r(request), data(pixels), plan(parsed), groups(request + parsed.groups, parsed.group_count) {}

    int prepare_draw() {
        auto* canvas = groups.surface(surface.get())->getCanvas();
        const uint32_t pos = plan.draws + index * plan.draw_words;
        const uint32_t snapshot = plan.snapshots + next_snapshot * plan.snapshot_words;
        if (next_snapshot < plan.snapshot_count && r[snapshot] == index) {
            if (!transfer.active()) {
                const auto* source = plan.snapshot_words == 2
                    ? groups.snapshot_pixels(buffer.get(), r[snapshot + 1]) : groups.pixels(buffer.get());
                if (!source) return 1; // guarded by complete frame validation
                if (!transfer.begin(source, size_t(r[2]) * r[3] * 4, MoPixelTransfer::Copy))
                    return fail_allocation();
                return 0;
            }
            if (!transfer.advance()) return 0;
            auto shader = mo_snapshot_shader(info, transfer.take());
            if (!shader) return fail_allocation();
            shaders[r[9] + plan.brush_count + next_snapshot++] = std::move(shader);
            return 0;
        }
        if (plan.draw_words >= 7) {
            const uint32_t old_depth = clips.length;
            const uint32_t common = clips.transition(r + plan.clips, r[pos + 6]);
            for (uint32_t k = common; k < old_depth; ++k) canvas->restore();
            for (uint32_t k = common; k < clips.length; ++k) {
                const auto* clip = r + plan.clips + (clips.nodes[k] - 1) * 4;
                canvas->save();
                canvas->translate(scalar(clip[2]), scalar(clip[3]));
                canvas->clipPath(paths[clip[1]], SkClipOp::kIntersect, true);
                canvas->resetMatrix();
            }
        }
        // World-space brushes remain independent of local path placement.
        auto shader = r[pos + 5] ? shaders[r[pos + 5] - 1]->makeWithLocalMatrix(
            SkMatrix::Translate(-scalar(r[pos + 1]), -scalar(r[pos + 2]))) : nullptr;
        if (r[pos + 5] && !shader) return fail_allocation();
        paint.setAntiAlias(true);
        paint.setShader(std::move(shader));
        paint.setColor(r[pos + 5] ? SK_ColorWHITE : color(r[pos + 3]));
        paint.setBlendMode(plan.draw_words == 8 && r[pos + 7] ? SkBlendMode::kSrc : SkBlendMode::kSrcOver);
        if (r[pos + 4]) {
            const uint32_t stroke = plan.strokes + (r[pos + 4] - 1) * 4;
            paint.setStyle(SkPaint::kStroke_Style);
            paint.setStrokeWidth(scalar(r[stroke]));
            paint.setStrokeCap(r[stroke + 1] == 0 ? SkPaint::kButt_Cap : r[stroke + 1] == 1 ? SkPaint::kRound_Cap : SkPaint::kSquare_Cap);
            paint.setStrokeJoin(r[stroke + 2] == 1 ? SkPaint::kRound_Join : r[stroke + 2] == 2 ? SkPaint::kBevel_Join : SkPaint::kMiter_Join);
            paint.setStrokeMiter(scalar(r[stroke + 3]));
        } else paint.setStyle(SkPaint::kFill_Style);
        draw_ready = true;
        return 0;
    }

    int draw() {
        if (!drawing && groups.boundary(index)) {
            // Each surface owns its clip stack. Reset before changing scope;
            // the next draw reapplies its full explicit clip chain once.
            if (clips.length) {
                auto* canvas = groups.surface(surface.get())->getCanvas();
                for (uint32_t k = 0; k < clips.length; ++k) canvas->restore();
                clips.length = 0;
            }
            const auto step = groups.advance(index, info, buffer.get());
            if (step == MoOpacityStack::AllocationFailure) return fail_allocation();
            if (step == MoOpacityStack::Progress) return 0;
        }
        if (index == r[6]) {
            for (uint32_t k = 0; k < clips.length; ++k) surface->getCanvas()->restore();
            surface.reset(); // No references to the output when it is transferred.
            phase = Complete;
            return 0;
        }
        if (!draw_ready) return prepare_draw();
        auto* canvas = groups.surface(surface.get())->getCanvas();
        if (!drawing) {
            canvas->save();
            const uint32_t pos = plan.draws + index * plan.draw_words;
            canvas->translate(scalar(r[pos + 1]), scalar(r[pos + 2]));
            // WrapPixels creates a bitmap device; no layers/filters or other
            // device types are accepted by this component's paint grammar.
            auto* device = static_cast<SkBitmapDevice*>(SkCanvasPriv::TopDevice(canvas));
            device->setMoRasterTaskSink(&drawing_task, &drawing_storage);
            if (r[pos + 4] && r[plan.strokes + (r[pos + 4] - 1) * 4 + 2] == 3)
                mo_draw_miter_clip(*canvas, paths[r[pos]], paint);
            else canvas->drawPath(paths[r[pos]], paint);
            device->setMoRasterTaskSink(nullptr);
            drawing = true;
            // Capture itself does no scan work. Enter the captured computation
            // in this work unit; its real scan suspension points remain intact.
        }
        const auto state = drawing_task.step();
        if (state == mo::RasterTask::Status::AllocationFailure) return fail_allocation();
        if (const auto failure = gradient_state.failure.load(std::memory_order_relaxed)) return failure;
        if (state == mo::RasterTask::Status::Yielded) return 0;
        drawing_task = {};
        canvas->restore();
        if (const auto failure = gradient_state.failure.load(std::memory_order_relaxed)) return failure;
        ++index; draw_ready = false; drawing = false;
        return 0;
    }

    int advance() {
        switch (phase) {
            case Gradients:
                if (index < r[9]) {
                    auto shader = mo_make_gradient(r + plan.gradients[index++], &gradient_state);
                    if (!shader) return fail_allocation();
                    shaders.push_back(std::move(shader));
                } else { phase = Images; index = 0; }
                break;
            case Images:
                if (index < plan.image_count) {
                    const auto* descriptor = r + plan.images + 4 * index;
                    const size_t bytes = size_t(descriptor[1]) * descriptor[2] * 4;
                    sk_sp<SkData> pixels;
                    if (descriptor[3]) {
                        pixels = SkData::MakeWithoutCopy(data + descriptor[0], bytes);
                    } else {
                        // Interpolating straight channels would tint translucent
                        // edges. Retain exact normalization, once per resource.
                        if (!transfer.active()) {
                            if (!transfer.begin(data + descriptor[0], bytes, MoPixelTransfer::Premultiply))
                                return fail_allocation();
                            break;
                        }
                        if (!transfer.advance()) break;
                        pixels = transfer.take();
                    }
                    auto image = mo_make_image(descriptor, std::move(pixels));
                    if (!image) return fail_allocation();
                    images.push_back(std::move(image)); ++index;
                } else { phase = Brushes; index = 0; }
                break;
            case Brushes:
                if (index < plan.brush_count) {
                    const auto* brush = r + plan.brushes + plan.brush_words * index++;
                    auto shader = plan.brush_words == 14 ? mo_make_image_domain(brush, images[brush[0]])
                                                        : mo_make_image_brush(brush, images[brush[0]]);
                    if (!shader) return fail_allocation();
                    shaders.push_back(std::move(shader));
                } else {
                    shaders.resize(r[9] + plan.brush_count + plan.snapshot_count);
                    phase = Surface; index = 0;
                }
                break;
            case Surface:
                buffer.reset(static_cast<uint8_t*>(std::malloc(r[2] * r[3] * 4)));
                if (!buffer) return fail_allocation();
                info = SkImageInfo::Make(r[2], r[3], kRGBA_8888_SkColorType,
                                         kPremul_SkAlphaType, SkColorSpace::MakeSRGB());
                phase = Clear;
                break;
            case Clear: {
                // Uniform Src clearing has no path/AA state. Use the original
                // Skia color conversion on bounded rows, then bind the complete
                // surface once. Never subdivide or re-clip actual path draws.
                const uint32_t rows = std::min(r[3] - index, mo_pixels_per_work_unit / r[2]);
                auto part = SkSurfaces::WrapPixels(info.makeWH(r[2], rows),
                    buffer.get() + size_t(index) * r[2] * 4, size_t(r[2]) * 4);
                if (!part) return fail_allocation();
                part->getCanvas()->clear(color(r[4]));
                index += rows;
                if (index < r[3]) break;
                surface = SkSurfaces::WrapPixels(info, buffer.get(), size_t(r[2]) * 4);
                if (!surface) return fail_allocation();
                phase = Paths; index = 0;
                break;
            }
            case Paths:
                if (index < r[5]) {
                    const uint32_t offset = plan.offsets[index];
                    if (!builder) builder.reset(new (std::nothrow) SkPathBuilder(
                        r[offset] ? SkPathFillType::kEvenOdd : SkPathFillType::kWinding));
                    if (!builder) return fail_allocation();
                    const uint32_t end = std::min(plan.counts[index], command + 4096);
                    for (; command < end; ++command) {
                        const uint32_t pos = offset + 2 + command * 7;
                        const float a = scalar(r[pos + 1]), b = scalar(r[pos + 2]);
                        const float c = scalar(r[pos + 3]), d = scalar(r[pos + 4]);
                        const float e = scalar(r[pos + 5]), f = scalar(r[pos + 6]);
                        switch (r[pos]) {
                            case 1: builder->moveTo(a, b); break;
                            case 2: builder->lineTo(a, b); break;
                            case 3: builder->quadTo(a, b, c, d); break;
                            case 4: builder->cubicTo(a, b, c, d, e, f); break;
                            case 5: builder->close(); break;
                        }
                    }
                    if (command == plan.counts[index]) {
                        paths.push_back(builder->detach()); builder.reset(); ++index; command = 0;
                    }
                } else { phase = Draws; index = 0; }
                break;
            case Draws: return draw();
            case Complete: return 0;
            case Failed: return status;
            case Taken: return 1;
        }
        return 0;
    }
};

extern "C" uint32_t mo_skia_execution_abi(void) { return 1; }
extern "C" int32_t mo_skia_raster_begin(const uint32_t* r, uint32_t words,
    const uint8_t* data, uint32_t bytes, uint32_t with_images, MoSkiaRasterTask** out) {
    if (!out) return 1;
    *out = nullptr;
    if (invalid.load(std::memory_order_acquire)) return 4;
    if (with_images > 1 || (!with_images && (data || bytes))) return 1;
    Plan plan{};
    if (const int status = validate(r, words, data, bytes, with_images != 0, plan)) return status;
    auto task = std::unique_ptr<MoSkiaRasterTask>(new (std::nothrow) MoSkiaRasterTask(r, data, plan));
    if (!task) return fail_allocation();
    task->shaders.reserve(r[9] + plan.brush_count + plan.snapshot_count);
    task->images.reserve(plan.image_count);
    task->paths.reserve(r[5]);
    *out = task.release();
    return 0;
}
extern "C" int32_t mo_skia_raster_step(MoSkiaRasterTask* task, uint32_t budget, uint32_t* complete) {
    if (!complete) return 1;
    *complete = 0;
    if (invalid.load(std::memory_order_acquire)) return 4;
    if (!task || !budget || budget > 4096) return 1;
    int result = 0;
    for (uint32_t i = 0; i < budget; ++i) {
        result = task->advance();
        if (result && task->phase != MoSkiaRasterTask::Taken) {
            task->status = result; task->phase = MoSkiaRasterTask::Failed;
        }
        if (result || task->phase == MoSkiaRasterTask::Complete) break;
    }
    if (!result && task->phase == MoSkiaRasterTask::Complete) *complete = 1;
    return result;
}
extern "C" int32_t mo_skia_raster_take(MoSkiaRasterTask* task, uint8_t** pixels, uint32_t* bytes) {
    if (!pixels || !bytes) return 1;
    *pixels = nullptr; *bytes = 0;
    if (invalid.load(std::memory_order_acquire)) return 4;
    if (!task) return 1;
    if (task->phase == MoSkiaRasterTask::Failed) return task->status;
    if (task->phase != MoSkiaRasterTask::Complete) return 1;
    *pixels = task->buffer.release(); *bytes = task->r[2] * task->r[3] * 4;
    task->phase = MoSkiaRasterTask::Taken;
    return 0;
}
extern "C" void mo_skia_raster_drop(MoSkiaRasterTask* task) { delete task; }
static int32_t raster(const uint32_t* r, uint32_t words, const uint8_t* data,
                      uint32_t bytes, bool with_images, uint8_t** pixels, uint32_t* length) {
    if (!pixels || !length) return 1;
    *pixels = nullptr; *length = 0;
    MoSkiaRasterTask* raw = nullptr;
    if (const auto status = mo_skia_raster_begin(r, words, data, bytes, with_images, &raw)) return status;
    const std::unique_ptr<MoSkiaRasterTask> task(raw);
    uint32_t complete = 0;
    while (!complete) if (const auto status = mo_skia_raster_step(raw, 4096, &complete)) return status;
    return mo_skia_raster_take(raw, pixels, length);
}

extern "C" int32_t mo_skia_raster(const uint32_t* r, uint32_t words,
                                  uint8_t** pixels, uint32_t* length) {
    return raster(r, words, nullptr, 0, false, pixels, length);
}
extern "C" int32_t mo_skia_raster_images(const uint32_t* r, uint32_t words,
    const uint8_t* data, uint32_t bytes, uint8_t** pixels, uint32_t* length) {
    return raster(r, words, data, bytes, true, pixels, length);
}
