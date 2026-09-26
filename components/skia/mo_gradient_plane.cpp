#include "mo_gradient.h"
#include "mo_elliptic_prepared.h"
#include "mo_gradient_coordinates.h"
#include "src/core/SkArenaAlloc.h"
#include "src/core/SkEffectPriv.h"
#include "src/core/SkRasterPipeline.h"
#include "src/core/SkRasterPipelineOpList.h"
#include "src/core/SkRasterPipelineOpContexts.h"
#include "src/shaders/SkShaderBase.h"
#include <bit>

namespace {
struct PlanePaint {
    mo::elliptic::PreparedField field;
    mo::gradient::PreparedCoordinates coordinates;
    MoGradientState* state;
};
mo::gradient::Coordinates coordinate_values(const uint32_t* p) {
    const auto f=[](uint32_t v){return std::bit_cast<float>(v);};
    return {{f(p[5]),f(p[6]),f(p[7]),f(p[8]),f(p[9]),f(p[10])},
            {p[11],p[12]},p[0]==4,{p[0]==4?f(p[13]):1,p[0]==4?f(p[14]):1}};
}
void coordinate_batch(const void* user,float* x,float* y,uint32_t count) {
    const auto& paint=*static_cast<const PlanePaint*>(user);auto& state=*paint.state;
    for(uint32_t i=0;i<count;++i){
        if(state.failure.load(std::memory_order_relaxed)){x[i]=y[i]=0;continue;}
        const auto result=paint.coordinates.map(x[i],y[i]);
        if(!result.valid||(result.exact_fallback&&state.coordinateFallbacks.fetch_add(1,std::memory_order_relaxed)>=MoGradientState::maxCoordinateFallbacks)) {
            state.failure.store(3,std::memory_order_relaxed);x[i]=y[i]=0;continue;
        }
        x[i]=result.x;y[i]=result.y;
    }
}
void elliptic_batch(const void* user, float* output, const float* x, const float* y, uint32_t count) {
    const auto& paint=*static_cast<const PlanePaint*>(user);
    auto& state=*paint.state;
    if (state.failure.load(std::memory_order_relaxed)) return;
    if (state.samples.fetch_add(count,std::memory_order_relaxed)>MoGradientState::maxSamples-count) {
        state.failure.store(3,std::memory_order_relaxed); return;
    }
    uint32_t nodes=0;
    for (uint32_t i=0; i<count; ++i) {
        const auto result=paint.field.evaluate(x[i],y[i]);
        if (result.status!=mo::elliptic::Status::Ok) {
            state.failure.store(3,std::memory_order_relaxed); return;
        }
        nodes+=result.nodes;
        output[i]=result.value;
    }
    if (state.nodes.fetch_add(nodes,std::memory_order_relaxed)>MoGradientState::maxNodes-nodes)
        state.failure.store(3,std::memory_order_relaxed);
}
class PlaneShader final : public SkShaderBase {
public:
    PlaneShader(sk_sp<SkShader> ramp, const uint32_t* p, MoGradientState* state)
        : ramp_(std::move(ramp)), elliptic_{mo::elliptic::PreparedField(p[0]==4 ?
            mo::elliptic::Field{std::bit_cast<float>(p[15]),std::bit_cast<float>(p[16]),
                std::bit_cast<float>(p[17]),std::bit_cast<float>(p[18])} : mo::elliptic::Field{}),
                mo::gradient::PreparedCoordinates(coordinate_values(p)),state} {
        context_.tileX = p[11]; context_.tileY = p[12]; context_.kind = p[0];
        for (uint32_t i=0; i<(p[0]==4 ? 6u : p[0] == 3 ? 4u : 3u); ++i) {
            context_.values[i] = std::bit_cast<float>(p[13+i]);
        }
        context_.ellipticUser=&elliptic_;context_.coordinates=coordinate_batch;
        if (p[0]==4) context_.elliptic=elliptic_batch;
    }
    Factory getFactory() const override { return nullptr; }
    const char* getTypeName() const override { return "MusterOfficeGradientPlane"; }
    ShaderType type() const override { return ShaderType::kMoGradientPlane; }
    bool isOpaque() const override { return ramp_->isOpaque(); }
    void flatten(SkWriteBuffer&) const override {} // Runtime-only evaluated field.
    bool appendStages(const SkStageRec& rec, const SkShaders::MatrixRec& matrix) const override {
        auto child = matrix.apply(rec);
        if (!child) return false;
        auto* ctx = rec.fAlloc->make<SkRasterPipelineContexts::MoGradientPlaneCtx>(context_);
        rec.fPipeline->append(SkRasterPipelineOp::mo_gradient_plane, ctx);
        child->markTotalMatrixInvalid();
        return as_SB(ramp_)->appendStages(rec, *child);
    }
private:
    sk_sp<SkShader> ramp_;
    PlanePaint elliptic_;
    SkRasterPipelineContexts::MoGradientPlaneCtx context_{};
};
}
sk_sp<SkShader> mo_make_gradient_plane(const uint32_t* p, sk_sp<SkShader> ramp, MoGradientState* state) {
    if (!ramp || !state) return nullptr;
    // Coordinates supplied by MatrixRec are device-space after the component's
    // canvas placement cancellation. Do not expand the paint inverse in float32.
    return sk_make_sp<PlaneShader>(std::move(ramp), p, state);
}
