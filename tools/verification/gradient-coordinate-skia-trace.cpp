#include "include/core/SkMatrix.h"
#include "src/core/SkArenaAlloc.h"
#include "src/core/SkRasterPipeline.h"
#include "src/core/SkRasterPipelineOpContexts.h"
#include "src/core/SkRasterPipelineOpList.h"
#include <bit>
#include <cstdio>
struct Capture:SkRasterPipelineContexts::CallbackCtx { float x=0,y=0; };
int main() {
    const int points[][2]={{322,120},{86,141},{49,173},{284,173}};
    for(const auto& p:points)for(int moved=0;moved<2;++moved){
        auto matrix=SkMatrix::MakeAll(300,0,25+10*moved,0,150,50+20*moved,0,0,1);SkMatrix inverse;
        if(!matrix.invert(&inverse))return 1;
        SkArenaAlloc arena(4096);SkRasterPipeline pipeline(&arena);Capture capture;
        capture.fn=[](SkRasterPipelineContexts::CallbackCtx* self,int){auto& c=*static_cast<Capture*>(self);c.x=c.rgba[0];c.y=c.rgba[1];};
        pipeline.append(SkRasterPipelineOp::seed_shader);pipeline.appendMatrix(&arena,inverse);
        pipeline.append(SkRasterPipelineOp::callback,&capture);pipeline.run(p[0]+10*moved,p[1]+20*moved,1,1);
        std::printf("%d %d %d %08x %08x\n",p[0],p[1],moved,std::bit_cast<uint32_t>(capture.x),std::bit_cast<uint32_t>(capture.y));
    }
}
