#pragma once
#include "include/core/SkPaint.h"
#include "include/core/SkPathBuilder.h"
#include <bit>
#include <cmath>
#include <cstdint>
inline float mo_scalar(uint32_t word) { return std::bit_cast<float>(word); }
inline bool mo_valid_stroke(const uint32_t* s) {
    const float width=mo_scalar(s[0]), miter=mo_scalar(s[3]);
    return std::isfinite(width) && width>=0 && width<=32768 && s[1]<=2 && s[2]<=3 &&
        std::isfinite(miter) && miter>=0 && miter<=1024 &&
        (s[2]!=3 || miter>=1) && ((s[2]!=1 && s[2]!=2) || s[3]==0);
}
inline void mo_configure_stroke(SkPaint& paint, const uint32_t* s) {
    paint.setStyle(SkPaint::kStroke_Style);
    paint.setStrokeWidth(mo_scalar(s[0]));
    paint.setStrokeCap(s[1]==0 ? SkPaint::kButt_Cap : s[1]==1 ? SkPaint::kRound_Cap : SkPaint::kSquare_Cap);
    paint.setStrokeJoin(s[2]==1 ? SkPaint::kRound_Join : s[2]==2 ? SkPaint::kBevel_Join : SkPaint::kMiter_Join);
    paint.setStrokeMiter(mo_scalar(s[3]));
}
inline void mo_append_path_command(SkPathBuilder& builder,const uint32_t* p) {
    const float a=mo_scalar(p[1]),b=mo_scalar(p[2]),c=mo_scalar(p[3]),d=mo_scalar(p[4]),e=mo_scalar(p[5]),f=mo_scalar(p[6]);
    switch(p[0]) {
        case 1: builder.moveTo(a,b);break;
        case 2: builder.lineTo(a,b);break;
        case 3: builder.quadTo(a,b,c,d);break;
        case 4: builder.cubicTo(a,b,c,d,e,f);break;
        case 5: builder.close();break;
    }
}
