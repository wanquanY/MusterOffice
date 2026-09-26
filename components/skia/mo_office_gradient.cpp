#include "mo_gradient.h"
#include "src/core/SkArenaAlloc.h"
#include "src/core/SkEffectPriv.h"
#include "src/core/SkRasterPipeline.h"
#include "src/core/SkRasterPipelineOpList.h"
#include "src/core/SkRasterPipelineOpContexts.h"
#include "src/shaders/SkShaderBase.h"
#include <bit>

namespace {
class OfficeGradientShader final : public SkShaderBase {
public:
    OfficeGradientShader(const uint32_t* p, sk_sp<SkShader> scalar)
        : scalar_(std::move(scalar)) {
        const auto* stops = p + mo_gradient_header(p[0]);
        for (unsigned i=0; i<4; ++i) {
            context_.first[i] = std::bit_cast<float>(stops[1+i]);
            context_.second[i] = std::bit_cast<float>(stops[6+i]);
        }
        context_.midpoint = p[4] == 3 ? std::bit_cast<float>(stops[5]) : 1;
    }
    Factory getFactory() const override { return nullptr; }
    const char* getTypeName() const override { return "MusterOfficeOfficeGradient"; }
    ShaderType type() const override { return ShaderType::kMoOfficeGradient; }
    bool isOpaque() const override {
        return scalar_->isOpaque() && context_.first[3] == 1 && context_.second[3] == 1;
    }
    void flatten(SkWriteBuffer&) const override {} // Evaluated runtime paint only.
    bool appendStages(const SkStageRec& rec, const SkShaders::MatrixRec& matrix) const override {
        if (!as_SB(scalar_)->appendStages(rec, matrix)) return false;
        auto* context = rec.fAlloc->make<SkRasterPipelineContexts::MoOfficeGradientCtx>(context_);
        rec.fPipeline->append(SkRasterPipelineOp::mo_office_gradient, context);
        return true;
    }
private:
    sk_sp<SkShader> scalar_;
    SkRasterPipelineContexts::MoOfficeGradientCtx context_{};
};
}
sk_sp<SkShader> mo_make_office_gradient(const uint32_t* p, sk_sp<SkShader> scalar) {
    if (!scalar) return nullptr;
    return sk_make_sp<OfficeGradientShader>(p, std::move(scalar));
}
