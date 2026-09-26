#include "mo_elliptic_prepared.h"
#include "mo_elliptic_interval.h"
namespace mo::elliptic {
using namespace detail;
PreparedField::PreparedField(Field f) : geometry_(f), nested_(false),
    concentric_(f.cx==0 && f.cy==0 && f.sx==f.sy && f.sx>=0 && f.sx<=32768),
    radius_(f.sx), radius2_(radius_*radius_) {
    const auto d=sub(exact(1),exact(radius_));
    denominator_lo_=d.lo; denominator_hi_=d.hi;
    if (std::isfinite(f.cx) && std::isfinite(f.cy) && f.sx>=0 && f.sy>=0 && f.sx<1 && f.sy<1) {
        const auto travel=sum(double(f.cx)*f.cx,double(f.cy)*f.cy);
        if (f.cx==0 && f.cy==0) nested_=true;
        else if (f.sx==0 && f.sy==0) nested_=travel.hi<1;
        else if (f.sx>0 && f.sy>0) {
            // h(t)=||((1-t)*r+t)*n|| is convex. At t=0 its
            // derivative is >= min(r_i*(1-r_i))/max(r_i) for unit n.
            // If this certified lower bound exceeds center speed, every
            // support direction grows strictly over the entire family.
            const double rate=std::min(mul(exact(f.sx),d).lo,
                mul(exact(f.sy),sub(exact(1),exact(f.sy))).lo);
            const double maximum=std::max(double(f.sx),double(f.sy));
            nested_=rate>0 && product(rate,rate).lo>mul(travel,exact(maximum*maximum)).hi;
        }
    }
}
Result PreparedField::evaluate(float x, float y, uint32_t budget) const {
    if ((!concentric_ && !nested_) || !std::isfinite(x) || !std::isfinite(y) ||
        std::abs(x)>1 || std::abs(y)>1 || budget==0)
        return detail::evaluate_prepared(geometry_,x,y,budget);
    if (nested_ && !concentric_) {
        Result result;
        if (evaluate_nested(x,y,result)) return result;
        return detail::evaluate_prepared(geometry_,x,y,budget);
    }
    // A binary32 square is exact in binary64, including input subnormals.
    const auto norm=sum(double(x)*x,double(y)*y);
    Result out; out.status=Status::Ok; out.certified_fast_path=true;
    if (norm.hi<=radius2_) { out.location=Location::Inner; return out; }
    if (norm.lo<=radius2_) return detail::evaluate_prepared(geometry_,x,y,budget);
    if (radius_>=1 || norm.lo>1) {
        out.location=Location::Outside; out.value=1; out.lower=out.upper=1; return out;
    }
    if (norm.hi>1) return detail::evaluate_prepared(geometry_,x,y,budget);
    constexpr double inf=std::numeric_limits<double>::infinity();
    const Interval distance{std::nextafter(std::sqrt(norm.lo),-inf),
                            std::nextafter(std::sqrt(norm.hi),inf)};
    const auto numerator=sub(distance,exact(radius_));
    out.lower=std::max(0.0,std::nextafter(numerator.lo/denominator_hi_,-inf));
    out.upper=std::min(1.0,std::nextafter(numerator.hi/denominator_lo_,inf));
    out.value=static_cast<float>((out.lower+out.upper)*0.5);
    out.lower=std::min(out.lower,double(out.value)); out.upper=std::max(out.upper,double(out.value));
    if (sum(out.upper,-out.lower).hi>max_scalar_error)
        return detail::evaluate_prepared(geometry_,x,y,budget);
    out.location=Location::Boundary; out.nodes=1; out.degree=2;
    return out;
}
}

namespace mo::elliptic {
namespace {
using namespace detail;
Interval scaled(Interval v, double a) {
    return a>=0 ? Interval{product(v.lo,a).lo,product(v.hi,a).hi}
                : Interval{product(v.hi,a).lo,product(v.lo,a).hi};
}
Interval squared(Interval v) {
    const auto a=product(v.lo,v.lo),b=product(v.hi,v.hi);
    return {v.lo<=0 && v.hi>=0 ? 0 : std::min(a.lo,b.lo),std::max(a.hi,b.hi)};
}
Interval positive_product(Interval a, Interval b) {
    return {product(a.lo,b.lo).lo,product(a.hi,b.hi).hi};
}
// Sign of ellipse membership. For t>0 both radii are positive. The t=0
// degenerate axes are classified geometrically before clearing denominators.
Interval membership(const detail::PreparedGeometry& g, float x, float y, double t) {
    const auto& f=g.field;
    if (t==0) {
        if ((f.sx==0 && x!=f.cx) || (f.sy==0 && y!=f.cy)) return exact(1);
        const auto ax=squared(sub(exact(x),exact(f.cx)));
        const auto ay=squared(sub(exact(y),exact(f.cy)));
        if (f.sx==0 && f.sy==0) return exact(-1);
        if (f.sx==0) return sub(ay,exact(g.by[0]));
        if (f.sy==0) return sub(ax,exact(g.bx[0]));
        return sub(add(scaled(ax,g.by[0]),scaled(ay,g.bx[0])),product(g.bx[0],g.by[0]));
    }
    const auto s=sum(1,-t);
    const auto ax=squared(sub(exact(x),scaled(s,f.cx)));
    const auto ay=squared(sub(exact(y),scaled(s,f.cy)));
    const auto bx=squared(add(scaled(s,f.sx),exact(t)));
    const auto by=squared(add(scaled(s,f.sy),exact(t)));
    return sub(add(positive_product(ax,by),positive_product(ay,bx)),positive_product(bx,by));
}
bool propose(const Field& f, float x, float y, double& result) {
    const double dx=1-double(f.sx),dy=1-double(f.sy);
    const double nx=double(f.cx)-double(x)*dx,ny=double(f.cy)-double(y)*dy;
    double a=0,b=1,t=0.5;
    for (unsigned i=0;i<16;++i) {
        const double s=1-t,rx=1/(s*f.sx+t),ry=1/(s*f.sy+t);
        const double u=(double(x)-s*f.cx)*rx,v=(double(y)-s*f.cy)*ry;
        const double value=u*u+v*v-1;
        const double derivative=2*(u*nx*rx*rx+v*ny*ry*ry);
        if (value>0) a=t; else b=t;
        const double next=derivative!=0 ? t-value/derivative : -1;
        if (next>=0 && next<=1 && std::abs(next-t)<0x1p-50) {result=next;return true;}
        t=next>a && next<b ? next : (a+b)*0.5;
    }
    result=t;return std::isfinite(t);
}
}
bool PreparedField::evaluate_nested(float x, float y, Result& out) const {
    const auto inner=membership(geometry_,x,y,0);
    out.certified_fast_path=true;
    if (inner.hi<=0) {out.status=Status::Ok;out.location=Location::Inner;return true;}
    if (inner.lo<=0) return false;
    const auto outer=sum(double(x)*x,double(y)*y);
    if (outer.lo>1 || (outer.lo==1 && outer.hi==1)) {
        out.status=Status::Ok;out.location=outer.lo>1 ? Location::Outside : Location::Boundary;
        out.value=1;out.lower=out.upper=1;
        if (out.location==Location::Boundary) {out.nodes=1;out.degree=geometry_.field.sx==geometry_.field.sy?2:4;}
        return true;
    }
    if (outer.hi>=1) return false;
    double t;
    if (!propose(geometry_.field,x,y,t) || t<0 || t>1) return false;
    const float value=static_cast<float>(t);
    // Verify the rounding cell, not an arbitrary epsilon around the proposal.
    // Strict opposite signs and nested sets prove the exact first root rounds
    // to this binary32 value. Uncertain midpoint ties use the general solver.
    const double lo=value==0 ? 0 : (double(std::nextafter(value,-INFINITY))+value)*0.5;
    const double hi=value==1 ? 1 : (double(std::nextafter(value,INFINITY))+value)*0.5;
    if ((lo!=0 && membership(geometry_,x,y,lo).lo<=0) ||
        (hi!=1 && membership(geometry_,x,y,hi).hi>=0)) return false;
    if (sum(hi,-lo).hi>max_scalar_error) return false;
    out.status=Status::Ok;out.location=Location::Boundary;out.value=value;
    out.lower=lo;out.upper=hi;out.nodes=1;
    out.degree=geometry_.field.sx==geometry_.field.sy?2:4;
    return true;
}
}
