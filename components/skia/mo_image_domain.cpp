#include "mo_image.h"
#include "include/core/SkColorSpace.h"
#include "include/core/SkPixmap.h"
#include "src/core/SkArenaAlloc.h"
#include "src/core/SkColorSpaceXformSteps.h"
#include "src/core/SkEffectPriv.h"
#include "src/core/SkRasterPipeline.h"
#include "src/core/SkRasterPipelineOpList.h"
#include "src/core/SkRasterPipelineOpContexts.h"
#include "src/shaders/SkShaderBase.h"
#include <algorithm>
#include <bit>
#include <cmath>

namespace {
class DomainShader final : public SkShaderBase {
public:
    DomainShader(sk_sp<SkImage> image, const uint32_t* p) : image_(std::move(image)) {
        context_.left = std::bit_cast<float>(p[10]);
        context_.top = std::bit_cast<float>(p[11]);
        context_.right = std::bit_cast<float>(p[12]);
        context_.bottom = std::bit_cast<float>(p[13]);
        context_.tileX = p[1]; context_.tileY = p[2]; context_.linear = p[3];
        auto inset = [linear = bool(p[3])](float lo,float hi,float& min,float& max) {
            if (linear) {
                const float half = std::min((hi-lo)/2,0.5f);
                min = lo+half; max = hi-half;
            } else {
                min = std::floor(lo)+0.5f; max = std::ceil(hi)-0.5f;
            }
        };
        inset(context_.left,context_.right,context_.minX,context_.maxX);
        inset(context_.top,context_.bottom,context_.minY,context_.maxY);
    }
    Factory getFactory() const override { return nullptr; }
    const char* getTypeName() const override { return "MusterOfficeImageDomain"; }
    ShaderType type() const override { return ShaderType::kMoImageDomain; }
    bool isOpaque() const override { return false; }
    void flatten(SkWriteBuffer&) const override {} // Runtime-only borrowed resource.
    bool appendStages(const SkStageRec& rec,const SkShaders::MatrixRec& matrix) const override {
        SkPixmap pixels;
        if (!image_->peekPixels(&pixels) || !matrix.apply(rec)) return false;
        auto* ctx = rec.fAlloc->make<SkRasterPipelineContexts::MoImageDomainCtx>(context_);
        ctx->pixels = static_cast<const uint32_t*>(pixels.addr());
        ctx->width = pixels.width(); ctx->height = pixels.height(); ctx->stride = pixels.rowBytesAsPixels();
        ctx->decalX = ctx->left < 0 || ctx->right > ctx->width;
        ctx->decalY = ctx->top < 0 || ctx->bottom > ctx->height;
        rec.fPipeline->append(SkRasterPipelineOp::mo_image_domain,ctx);
        SkColorSpaceXformSteps(pixels.colorSpace(),kPremul_SkAlphaType,
                              rec.fDstCS,kPremul_SkAlphaType).apply(rec.fPipeline);
        return true;
    }
private:
    sk_sp<SkImage> image_;
    SkRasterPipelineContexts::MoImageDomainCtx context_{};
};
}
sk_sp<SkShader> mo_make_image_domain(const uint32_t* p,const sk_sp<SkImage>& image) {
    // Keep exact whole-image behavior and the existing optimized stages.
    if (std::bit_cast<float>(p[10]) == 0 && std::bit_cast<float>(p[11]) == 0 &&
        std::bit_cast<float>(p[12]) == image->width() && std::bit_cast<float>(p[13]) == image->height()) {
        return mo_make_image_brush(p,image);
    }
    auto f = [](uint32_t v){return std::bit_cast<float>(v);};
    const auto matrix = SkMatrix::MakeAll(f(p[4]),f(p[5]),f(p[6]),f(p[7]),f(p[8]),f(p[9]),0,0,1);
    return sk_make_sp<DomainShader>(image,p)->makeWithLocalMatrix(matrix);
}
