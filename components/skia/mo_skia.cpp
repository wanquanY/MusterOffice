#include "mo_skia.h"
#include "mo_clip.h"
#include "mo_composite.h"
#include "mo_miter_clip.h"
#include "mo_gradient.h"
#include "mo_image.h"
#include "include/core/SkCanvas.h"
#include "include/core/SkColorSpace.h"
#include "include/core/SkImageInfo.h"
#include "include/core/SkPaint.h"
#include "include/core/SkPath.h"
#include "include/core/SkPathBuilder.h"
#include "include/core/SkSurface.h"
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
    uint32_t snapshots = 0, snapshot_count = 0;
};
int validate(const uint32_t* r, uint32_t words, const uint8_t* data,
             uint32_t bytes, bool with_images, Plan& plan) {
    if (!r || words < 10 || words > max_words_v12 || r[0] != magic) return 1;
    const bool elliptic = r[1] == 12;
    const bool rectangular = r[1] == 11 || elliptic;
    const bool office = r[1] == 10 || rectangular;
    const bool planes = r[1] == 9 || office;
    const bool composite = r[1] == 8 || planes;
    const bool clipping = r[1] == 7 || composite;
    const uint32_t header = composite ? 14 : clipping ? 13 : with_images ? 12 : 10;
    if (words < header || words > (elliptic ? max_words_v12 : rectangular ? max_words_v11 : planes ? max_words_v9 : composite ? max_words_v8 : clipping ? max_words_v7 : with_images ? max_words_v6 : max_words_v4) ||
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
        uint64_t(plan.clip_count)*4 + plan.snapshot_count + uint64_t(r[6])*plan.draw_words != words) return 1;
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
    for (uint32_t i = 0; i < plan.snapshot_count; ++i, ++pos) {
        if (r[pos] >= r[6] || (i && r[pos - 1] >= r[pos])) return 1;
    }
    plan.draws = pos;
    uint64_t work = 0, clip_work = 0, applications = 0;
    MoClipStack clip_stack;
    for (uint32_t i = 0; i < r[6]; ++i, pos += plan.draw_words) {
        if (clipping) {
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
        if (r[pos + 5] > r[9] + plan.brush_count &&
            r[plan.snapshots + r[pos + 5] - r[9] - plan.brush_count - 1] > i) return 1;
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

extern "C" uint32_t mo_skia_abi(void) { return 4; }
extern "C" uint32_t mo_skia_clips_abi(void) { return 1; }
extern "C" uint32_t mo_skia_gradient_planes_abi(void) { return 1; }
extern "C" uint32_t mo_skia_office_gradients_abi(void) { return 1; }
extern "C" uint32_t mo_skia_rect_gradients_abi(void) { return 1; }
extern "C" uint32_t mo_skia_elliptic_gradients_abi(void) { return 1; }
extern "C" uint32_t mo_skia_compositing_abi(void) { return 1; }
extern "C" uint32_t mo_skia_images_abi(void) { return 2; }
extern "C" void mo_skia_free(void* pixels) { std::free(pixels); }
static int32_t raster(const uint32_t* r, uint32_t words, const uint8_t* data,
                      uint32_t bytes_in, bool with_images, uint8_t** pixels, uint32_t* length) {
    if (!pixels || !length) return 1;
    *pixels = nullptr;
    *length = 0;
    if (invalid.load(std::memory_order_acquire)) return 4;
    Plan plan{};
    if (const int status = validate(r, words, data, bytes_in, with_images, plan)) return status;
    MoGradientState gradient_state;
    std::vector<sk_sp<SkShader>> shaders;
    shaders.reserve(r[9] + plan.brush_count + plan.snapshot_count);
    for (uint32_t i = 0; i < r[9]; ++i) {
        auto shader = mo_make_gradient(r + plan.gradients[i], &gradient_state);
        if (!shader) return fail_allocation();
        shaders.push_back(std::move(shader));
    }
    std::vector<sk_sp<SkImage>> images;
    images.reserve(plan.image_count);
    for (uint32_t i = 0; i < plan.image_count; ++i) {
        auto image = mo_make_image(r + plan.images + 4*i, data);
        if (!image) return fail_allocation();
        images.push_back(std::move(image));
    }
    for (uint32_t i = 0; i < plan.brush_count; ++i) {
        const auto* brush = r + plan.brushes + plan.brush_words*i;
        auto shader = plan.brush_words == 14 ? mo_make_image_domain(brush, images[brush[0]])
                                            : mo_make_image_brush(brush, images[brush[0]]);
        if (!shader) return fail_allocation();
        shaders.push_back(std::move(shader));
    }
    shaders.resize(r[9] + plan.brush_count + plan.snapshot_count);
    const uint32_t bytes = r[2] * r[3] * 4;
    std::unique_ptr<uint8_t, decltype(&std::free)> buffer(
        static_cast<uint8_t*>(std::malloc(bytes)), &std::free);
    if (!buffer) return fail_allocation();
    const auto info = SkImageInfo::Make(r[2], r[3], kRGBA_8888_SkColorType,
                                      kPremul_SkAlphaType, SkColorSpace::MakeSRGB());
    auto surface = SkSurfaces::WrapPixels(info, buffer.get(), size_t(r[2]) * 4);
    if (!surface) return fail_allocation();
    auto* canvas = surface->getCanvas();
    canvas->clear(color(r[4]));
    std::vector<SkPath> paths;
    paths.reserve(r[5]);
    for (uint32_t i = 0; i < r[5]; ++i) {
        uint32_t pos = plan.offsets[i];
        SkPathBuilder builder(r[pos] ? SkPathFillType::kEvenOdd : SkPathFillType::kWinding);
        pos += 2;
        for (uint32_t j = 0; j < plan.counts[i]; ++j, pos += 7) {
            const float a = scalar(r[pos + 1]), b = scalar(r[pos + 2]);
            const float c = scalar(r[pos + 3]), d = scalar(r[pos + 4]);
            const float e = scalar(r[pos + 5]), f = scalar(r[pos + 6]);
            switch (r[pos]) {
                case 1: builder.moveTo(a, b); break;
                case 2: builder.lineTo(a, b); break;
                case 3: builder.quadTo(a, b, c, d); break;
                case 4: builder.cubicTo(a, b, c, d, e, f); break;
                case 5: builder.close(); break;
            }
        }
        paths.push_back(builder.detach());
    }
    SkPaint paint;
    paint.setAntiAlias(true);
    paint.setStyle(SkPaint::kFill_Style);
    paint.setBlendMode(SkBlendMode::kSrcOver);
    MoClipStack clip_stack;
    uint32_t next_snapshot = 0;
    for (uint32_t i = 0, pos = plan.draws; i < r[6]; ++i, pos += plan.draw_words) {
        if (next_snapshot < plan.snapshot_count && r[plan.snapshots + next_snapshot] == i) {
            auto shader = mo_snapshot_shader(info, buffer.get(), bytes);
            if (!shader) return fail_allocation();
            shaders[r[9] + plan.brush_count + next_snapshot++] = std::move(shader);
        }
        if (plan.draw_words >= 7) {
            const uint32_t old_depth = clip_stack.length;
            const uint32_t common = clip_stack.transition(r + plan.clips, r[pos + 6]);
            for (uint32_t k = common; k < old_depth; ++k) canvas->restore();
            for (uint32_t k = common; k < clip_stack.length; ++k) {
                const auto* clip = r + plan.clips + (clip_stack.nodes[k] - 1) * 4;
                canvas->save();
                canvas->translate(scalar(clip[2]), scalar(clip[3]));
                canvas->clipPath(paths[clip[1]], SkClipOp::kIntersect, true);
                canvas->resetMatrix();
            }
        }
        canvas->save();
        canvas->translate(scalar(r[pos + 1]), scalar(r[pos + 2]));
        // World-space brushes do not move when a reused local path is placed.
        // The inverse translation cancels the canvas placement only for paint.
        auto shader = r[pos + 5] ? shaders[r[pos + 5] - 1]->makeWithLocalMatrix(
            SkMatrix::Translate(-scalar(r[pos + 1]), -scalar(r[pos + 2]))) : nullptr;
        if (r[pos + 5] && !shader) return fail_allocation();
        paint.setShader(std::move(shader));
        paint.setColor(r[pos + 5] ? SK_ColorWHITE : color(r[pos + 3]));
        paint.setBlendMode(plan.draw_words == 8 && r[pos + 7] ? SkBlendMode::kSrc : SkBlendMode::kSrcOver);
        if (r[pos + 4]) {
            const uint32_t s = plan.strokes + (r[pos + 4] - 1) * 4;
            paint.setStyle(SkPaint::kStroke_Style);
            paint.setStrokeWidth(scalar(r[s]));
            paint.setStrokeCap(r[s + 1] == 0 ? SkPaint::kButt_Cap : r[s + 1] == 1 ? SkPaint::kRound_Cap : SkPaint::kSquare_Cap);
            paint.setStrokeJoin(r[s + 2] == 1 ? SkPaint::kRound_Join : r[s + 2] == 2 ? SkPaint::kBevel_Join : SkPaint::kMiter_Join);
            paint.setStrokeMiter(scalar(r[s + 3]));
            if (r[s + 2] == 3) {
                mo_draw_miter_clip(*canvas, paths[r[pos]], paint);
                canvas->restore();
                if (const auto status=gradient_state.failure.load(std::memory_order_relaxed)) return status;
                continue;
            }
        } else {
            paint.setStyle(SkPaint::kFill_Style);
        }
        canvas->drawPath(paths[r[pos]], paint);
        canvas->restore();
        if (const auto status=gradient_state.failure.load(std::memory_order_relaxed)) return status;
    }
    for (uint32_t k = 0; k < clip_stack.length; ++k) canvas->restore();
    // Destroy Skia's references before transferring the caller-owned buffer.
    surface.reset();
    *pixels = buffer.release();
    *length = bytes;
    return 0;
}

extern "C" int32_t mo_skia_raster(const uint32_t* r, uint32_t words,
                                  uint8_t** pixels, uint32_t* length) {
    return raster(r, words, nullptr, 0, false, pixels, length);
}
extern "C" int32_t mo_skia_raster_images(const uint32_t* r, uint32_t words,
    const uint8_t* data, uint32_t bytes, uint8_t** pixels, uint32_t* length) {
    return raster(r, words, data, bytes, true, pixels, length);
}
