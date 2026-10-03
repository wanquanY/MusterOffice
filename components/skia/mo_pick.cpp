#include "mo_path.h"
#include "mo_clip.h"
#include "mo_miter_clip.h"
#include "mo_skia_state.h"
#include <algorithm>
#include <cstdlib>
#include <map>
#include <memory>
#include <vector>

namespace {
constexpr uint32_t magic=0x4d4f504b;
constexpr uint32_t max_words=10+4096*2+262144*7+4096*4+8192*4+65536*5+64*3;
constexpr uint64_t max_query_work=8388608, max_generated=1048576;
struct Plan {
    uint32_t offsets[4096]{},counts[4096]{};
    float bounds[4096][4]{};
    uint32_t strokes=0,clips=0,draws=0,queries=0;
};
bool coordinate(uint32_t word) {
    const auto v=mo_scalar(word);return std::isfinite(v) && std::abs(v)<=32768;
}
int validate(const uint32_t* r,uint32_t words,Plan& p) {
    if(!r || words<10 || words>max_words || r[0]!=magic || r[1]!=1) return 1;
    if(!r[2] || !r[3] || r[2]>8192 || r[3]>8192 || uint64_t(r[2])*r[3]>16777216 ||
        r[4]>4096 || r[5]>262144 || r[6]>4096 || r[7]>8192 || r[8]>65536 || r[9]>64) return 3;
    uint32_t pos=10,total=0;
    for(uint32_t i=0;i<r[4];i++) {
        if(words-pos<2 || r[pos]>1) return 1;
        p.offsets[i]=pos;p.counts[i]=r[pos+1];pos+=2;
        if(p.counts[i]>262144-total || p.counts[i]>(words-pos)/7) return 3;
        total+=p.counts[i];bool contour=false;
        for(uint32_t j=0;j<p.counts[i];j++,pos+=7) {
            const uint32_t op=r[pos];
            if(op<1 || op>5 || (op!=1 && !contour)) return 1;
            const uint32_t n=op<=2?2:op==3?4:op==4?6:0;
            for(uint32_t k=0;k<6;k++) if(k<n?!coordinate(r[pos+1+k]):r[pos+1+k]!=0) return 1;
            for(uint32_t k=0;k<n;k+=2) {
                p.bounds[i][0]=std::min(p.bounds[i][0],mo_scalar(r[pos+1+k]));
                p.bounds[i][1]=std::min(p.bounds[i][1],mo_scalar(r[pos+2+k]));
                p.bounds[i][2]=std::max(p.bounds[i][2],mo_scalar(r[pos+1+k]));
                p.bounds[i][3]=std::max(p.bounds[i][3],mo_scalar(r[pos+2+k]));
            }
            if(op==1) contour=true;if(op==5) contour=false;
        }
    }
    if(total!=r[5] || uint64_t(pos)+r[6]*4+r[7]*4+r[8]*5+r[9]*3!=words) return 1;
    p.strokes=pos;
    for(uint32_t i=0;i<r[6];i++,pos+=4) if(!mo_valid_stroke(r+pos)) return 1;
    p.clips=pos;
    if(auto status=mo_validate_clips(r+pos,r[7],r[4],p.counts,p.bounds)) return status;
    uint64_t work=r[7]+r[8];
    for(uint32_t i=0;i<r[7];i++,pos+=4) work+=p.counts[r[pos+1]];
    p.draws=pos;
    for(uint32_t i=0;i<r[8];i++,pos+=5) {
        if(r[pos]>=r[4] || !coordinate(r[pos+1]) || !coordinate(r[pos+2]) || r[pos+3]>r[6] || r[pos+4]>r[7]) return 1;
        const auto& b=p.bounds[r[pos]];
        for(uint32_t k=0;k<4;k++) if(std::abs(double(b[k])+mo_scalar(r[pos+1+k%2]))>32768) return 3;
        work+=p.counts[r[pos]];
    }
    if(work*r[9]>max_query_work) return 3;
    p.queries=pos;
    for(uint32_t i=0;i<r[9];i++,pos+=3) {
        if(!coordinate(r[pos]) || !coordinate(r[pos+1]) || !coordinate(r[pos+2]) ||
            mo_scalar(r[pos+2])<0 || mo_scalar(r[pos+2])>32) return 1;
    }
    return 0;
}
}
extern "C" uint32_t mo_skia_pick_abi() { return 1; }
extern "C" int mo_skia_pick(const uint32_t* r,uint32_t words,uint32_t** output,uint32_t* output_words) {
    if(!output || !output_words) return 1;
    *output=nullptr;*output_words=0;
    if(mo_skia_unavailable()) return 4;
    auto p=std::unique_ptr<Plan>(new(std::nothrow) Plan);
    if(!p) return mo_skia_allocation_failure();
    if(auto status=validate(r,words,*p)) return status;
    std::vector<SkPath> paths,fills;paths.reserve(r[4]);fills.reserve(r[4]);
    for(uint32_t i=0;i<r[4];i++) {
        auto offset=p->offsets[i];const auto rule=r[offset]?SkPathFillType::kEvenOdd:SkPathFillType::kWinding;
        SkPathBuilder b(rule),fill(rule);bool contour=false;
        for(uint32_t j=0;j<p->counts[i];j++) {
            const auto* command=r+offset+2+j*7;
            if(command[0]==1 && contour) fill.close();
            mo_append_path_command(b,command);mo_append_path_command(fill,command);
            if(command[0]==1) contour=true;if(command[0]==5) contour=false;
        }
        if(contour) fill.close();
        paths.push_back(b.detach());fills.push_back(fill.detach());
    }
    // Intern by actual path and stroke. Paint, images and alpha never enter a
    // geometry query; application object IDs and stacking policy stay in Rust.
    std::map<std::pair<uint32_t,uint32_t>,uint32_t> indices;
    std::vector<SkPath> geometry;std::vector<uint32_t> draws;draws.reserve(r[8]);
    uint64_t generated=0;
    for(uint32_t i=0;i<r[8];i++) {
        const auto* d=r+p->draws+i*5;auto key=std::make_pair(d[0],d[3]);
        auto it=indices.find(key);
        if(it!=indices.end()) {draws.push_back(it->second);continue;}
        if(geometry.size()==8192) return 3;
        SkPath g=d[3]?paths[d[0]]:fills[d[0]];
        if(d[3]) {
            const auto* s=r+p->strokes+(d[3]-1)*4;SkPaint paint;mo_configure_stroke(paint,s);
            g=mo_stroke_outline(g,paint,s[2]==3);
            generated+=g.countVerbs();if(generated>max_generated) return 3;
            if(!g.isFinite()) return 3;
        }
        auto id=uint32_t(geometry.size());indices.emplace(key,id);draws.push_back(id);geometry.push_back(std::move(g));
    }
    const uint32_t stride=(r[8]+31)/32,clip_stride=(r[7]+32)/32,query_stride=stride*2+clip_stride,length=7+r[9]*query_stride;
    std::unique_ptr<uint32_t,decltype(&std::free)> result(static_cast<uint32_t*>(std::calloc(length,4)),&std::free);
    if(!result) return mo_skia_allocation_failure();
    const uint32_t header[]{magic,1,r[9],r[8],stride,r[7],clip_stride};std::copy(header,header+7,result.get());
    uint64_t work=0;
    for(uint32_t q=0;q<r[9];q++) {
        const auto* query=r+p->queries+q*3;const float x=mo_scalar(query[0]),y=mo_scalar(query[1]),radius=mo_scalar(query[2]);
        if(x<0 || y<0 || x>=r[2] || y>=r[3]) continue;
        auto* clip_bits=result.get()+7+q*query_stride+stride*2;clip_bits[0]=1;
        std::vector<bool> clips(r[7]+1,true);
        for(uint32_t c=0;c<r[7];c++) {
            const auto* clip=r+p->clips+c*4;work+=1+paths[clip[1]].countVerbs();
            if(work>max_query_work) return 3;
            clips[c+1]=clips[clip[0]] && paths[clip[1]].contains(x-mo_scalar(clip[2]),y-mo_scalar(clip[3]));
            if(clips[c+1]) clip_bits[(c+1)/32]|=1u<<((c+1)%32);
        }
        std::map<uint32_t,SkPath> halo;
        for(uint32_t i=0;i<r[8];i++) {
            const auto* d=r+p->draws+i*5;if(++work>max_query_work) return 3;
            if(!clips[d[4]]) continue;
            const auto& g=geometry[draws[i]];work+=g.countVerbs();if(work>max_query_work) return 3;
            const float local_x=x-mo_scalar(d[1]),local_y=y-mo_scalar(d[2]);
            bool exact=g.contains(local_x,local_y),near=false;
            if(!exact && radius>0) {
                auto found=halo.find(draws[i]);
                if(found==halo.end()) {
                    SkPaint paint;paint.setStyle(SkPaint::kStroke_Style);paint.setStrokeWidth(2*radius);
                    paint.setStrokeJoin(SkPaint::kRound_Join);paint.setStrokeCap(SkPaint::kRound_Cap);
                    auto outline=mo_stroke_outline(g,paint,false);
                    generated+=outline.countVerbs();if(generated>max_generated || !outline.isFinite()) return 3;
                    found=halo.emplace(draws[i],std::move(outline)).first;
                }
                work+=found->second.countVerbs();if(work>max_query_work) return 3;
                near=found->second.contains(local_x,local_y);
            }
            if(exact || near) result.get()[7+q*query_stride+(exact?0:stride)+i/32] |= 1u<<(i%32);
        }
    }
    *output_words=length;*output=result.release();return 0;
}
