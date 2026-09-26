#include "mo_elliptic_field.h"
#include "mo_elliptic_geometry.h"
#include <array>

namespace mo::elliptic {
namespace {
using namespace detail;
using Quadratic = std::array<Interval,3>;
struct Node { Polynomial p{}; double lo=0, hi=1; uint32_t depth=0; };
Quadratic square(Interval a, Interval b) { return {mul(a,a),mul(a,b),mul(b,b)}; }
// Six times the product in Bernstein degree four: positive scale avoids the
// non-dyadic division by six and cannot change roots or coefficient signs.
Polynomial multiply6(const Quadratic& a, const Quadratic& b) {
    auto m = [&](unsigned i, unsigned j) { return mul(a[i],b[j]); };
    return {mul(exact(6),m(0,0)),mul(exact(3),add(m(0,1),m(1,0))),
        add(add(m(0,2),mul(exact(4),m(1,1))),m(2,0)),
        mul(exact(3),add(m(1,2),m(2,1))),mul(exact(6),m(2,2))};
}
// Radius Bernstein coefficients are exact nonnegative binary64 values.
// Multiplication by a singleton interval needs only its two monotone endpoints.
Interval scaled(Interval a, double b) {
    return {product(a.lo,b).lo,product(a.hi,b).hi};
}
Polynomial multiply6_radius(const Quadratic& a, const std::array<double,3>& b) {
    auto m = [&](unsigned i, unsigned j) { return scaled(a[i],b[j]); };
    return {scaled(m(0,0),6),scaled(add(m(0,1),m(1,0)),3),
        add(add(m(0,2),scaled(m(1,1),4)),m(2,0)),
        scaled(add(m(1,2),m(2,1)),3),scaled(m(2,2),6)};
}
int sign(Interval v) {
    return v.lo > 0 ? 1 : v.hi < 0 ? -1 : v.lo == 0 && v.hi == 0 ? 0 : 2;
}
int variations(const Polynomial& p, unsigned degree) {
    int last=0, count=0;
    for (unsigned i=0; i<=degree; ++i) {
        const int s=sign(p[i]);
        if (s == 2) return -1;
        if (s != 0) { if (last != 0 && last != s) ++count; last=s; }
    }
    return count;
}
Interval value(const Polynomial& p, unsigned degree, double t) {
    Polynomial v=p;
    for (unsigned n=degree; n>0; --n)
        for (unsigned i=0; i<n; ++i) v[i]=mix(v[i],v[i+1],t);
    return v[0];
}
void split(const Node& in, unsigned degree, Node& left, Node& right) {
    Polynomial v=in.p;
    const double mid=(in.lo+in.hi)*0.5;
    left.lo=in.lo; left.hi=mid; right.lo=mid; right.hi=in.hi;
    left.depth=right.depth=in.depth+1;
    left.p[0]=v[0]; right.p[degree]=v[degree];
    for (unsigned n=degree; n>0; --n) {
        for (unsigned i=0; i<n; ++i) v[i]=average(v[i],v[i+1]);
        left.p[degree-n+1]=v[0]; right.p[n-1]=v[n-1];
    }
}
Result root(Result out, double lo, double hi) {
    out.value=static_cast<float>((lo+hi)*0.5);
    out.lower=std::min(lo,double(out.value)); out.upper=std::max(hi,double(out.value));
    if (sum(out.upper,-out.lower).hi > max_scalar_error) {
        out.status=Status::Precision;
        return out;
    }
    out.status=Status::Ok; out.location=Location::Boundary;
    return out;
}
// Safeguarded floating Newton steps only propose a candidate. Descartes proves
// one root; interval signs must then certify both candidate endpoints. Failure
// continues through bounded root isolation with no acceptance of the guess.
bool candidate(const Polynomial& p, unsigned degree, double& lo, double& hi) {
    double a=0, b=1, t=0.5;
    for (unsigned step=0; step<16; ++step) {
        double v[5];
        for (unsigned i=0; i<=degree; ++i) v[i]=(p[i].lo+p[i].hi)*0.5;
        for (unsigned n=degree; n>1; --n)
            for (unsigned i=0; i<n; ++i) v[i]=(1-t)*v[i]+t*v[i+1];
        const double derivative=degree*(v[1]-v[0]);
        const double y=(1-t)*v[0]+t*v[1];
        if (y==0) break;
        if (y>0) a=t; else b=t;
        const double newton=derivative!=0 ? t-y/derivative : -1;
        // A rounded Newton step can equal the newly updated bracket endpoint.
        // Preserve this converged proposal; interval verification still decides
        // whether it is accepted, including a step just outside that bracket.
        if (newton>=0 && newton<=1 && std::abs(newton-t)<0x1p-30) { t=newton; break; }
        const double next=newton>a && newton<b ? newton : (a+b)*0.5;
        const bool close=std::abs(next-t)<0x1p-30;
        t=next;
        if (close) break;
    }
    lo=std::max(0.0,t-0x1p-25); hi=std::min(1.0,t+0x1p-25);
    return value(p,degree,lo).lo > 0 && value(p,degree,hi).hi < 0;
}
bool repeated_quadratic(const Polynomial& p, double& lo, double& hi) {
    // Bernstein quadratic discriminant / 4 is p1^2-p0*p2. Certify exact
    // repeated roots rather than classifying a close pair as a tangency.
    const auto d=sub(mul(p[1],p[1]),mul(p[0],p[2]));
    const auto denominator=sub(p[0],p[1]);
    if (sign(d)!=0 || p[0].lo<=0 || denominator.lo<=0) return false;
    constexpr double inf=std::numeric_limits<double>::infinity();
    lo=std::nextafter(p[0].lo/denominator.hi,-inf);
    hi=std::nextafter(p[0].hi/denominator.lo,inf);
    return lo>=0 && hi<=1;
}
}
detail::PreparedGeometry::PreparedGeometry(Field f) : field(f) {
    if (!std::isfinite(f.sx) || !std::isfinite(f.sy) || f.sx<0 || f.sy<0 || f.sx>32768 || f.sy>32768) return;
    bx={double(f.sx)*f.sx,double(f.sx),1};
    by={double(f.sy)*f.sy,double(f.sy),1};
    if (f.sx!=f.sy) radius_product=multiply6(
        {exact(bx[0]),exact(bx[1]),exact(1)},
        {exact(by[0]),exact(by[1]),exact(1)});
}
Result evaluate(const Field& f, float x, float y, uint32_t node_budget) {
    return detail::evaluate_prepared(detail::PreparedGeometry(f),x,y,node_budget);
}
Result detail::evaluate_prepared(const PreparedGeometry& geometry, float x, float y, uint32_t node_budget) {
    const auto& f=geometry.field;
    Result out;
    for (float v : {x,y,f.cx,f.cy,f.sx,f.sy})
        if (!std::isfinite(v) || std::abs(v)>32768) return out;
    if (std::abs(x)>1 || std::abs(y)>1 || f.sx<0 || f.sy<0) return out;
    if (node_budget == 0) { out.status=Status::Limit; return out; }
    node_budget=std::min(node_budget,max_nodes);
    const auto ax=square(sub(exact(x),exact(f.cx)),exact(x));
    const auto ay=square(sub(exact(y),exact(f.cy)),exact(y));
    const Quadratic bx{exact(geometry.bx[0]),exact(geometry.bx[1]),exact(1)};
    const Quadratic by{exact(geometry.by[0]),exact(geometry.by[1]),exact(1)};
    Interval inner=exact(1);
    if ((f.sx!=0 || x==f.cx) && (f.sy!=0 || y==f.cy)) {
        if (f.sx==0 && f.sy==0) inner=exact(0);
        else if (f.sx==0) inner=sub(ay[0],by[0]);
        else if (f.sy==0) inner=sub(ax[0],bx[0]);
        else if (f.sx==f.sy) inner=sub(add(ax[0],ay[0]),bx[0]);
        else inner=sub(add(mul(ax[0],by[0]),mul(ay[0],bx[0])),mul(bx[0],by[0]));
    }
    if (inner.hi<=0) {
        out.status=Status::Ok; out.location=Location::Inner;
        return out;
    }
    if (inner.lo<=0) { out.status=Status::Precision; return out; }
    Node first;
    unsigned degree=4;
    // Remove a common squared radius for circular foci, or an exact t^2
    // denominator factor at degenerate endpoints. These factors are positive
    // for t>0; t=0 was classified geometrically above.
    if (f.sx==f.sy) {
        degree=2;
        for (unsigned i=0; i<3; ++i) first.p[i]=sub(add(ax[i],ay[i]),bx[i]);
    } else if (f.sx==0 && x==f.cx) {
        degree=2;
        const auto k=sub(mul(exact(f.cx),exact(f.cx)),exact(1));
        for (unsigned i=0; i<3; ++i) first.p[i]=add(ay[i],mul(k,by[i]));
    } else if (f.sy==0 && y==f.cy) {
        degree=2;
        const auto k=sub(mul(exact(f.cy),exact(f.cy)),exact(1));
        for (unsigned i=0; i<3; ++i) first.p[i]=add(ax[i],mul(k,bx[i]));
    } else {
        const auto a=multiply6_radius(ax,geometry.by), b=multiply6_radius(ay,geometry.bx);
        for (unsigned i=0; i<5; ++i) first.p[i]=sub(add(a[i],b[i]),geometry.radius_product[i]);
    }
    out.degree=degree;
    if (degree==2) {
        double lo,hi;
        if (repeated_quadratic(first.p,lo,hi)) {
            out.nodes=1;
            return root(out,lo,hi);
        }
    }
    // Descartes' one-variation test is valid even when ellipse membership is
    // not monotone. Ambiguous coefficient signs never enter this fast path.
    if (variations(first.p,degree)==1 && first.p[0].lo>0 && first.p[degree].hi<0) {
        double lo,hi;
        if (candidate(first.p,degree,lo,hi)) {
            out.nodes=1; out.certified_fast_path=true;
            return root(out,lo,hi);
        }
    }
    // Left-first search proves every earlier interval empty before returning a
    // root. At most 25 pending nodes, depth 24, 512 total visits, no heap/recursion.
    Node stack[25];
    unsigned count=1; stack[0]=first;
    while (count) {
        if (out.nodes==node_budget) { out.status=Status::Limit; return out; }
        ++out.nodes;
        Node node=stack[--count];
        if (node.p[0].hi<=0) return root(out,node.lo,node.lo);
        const int v=variations(node.p,degree);
        if (v==0 && node.p[0].lo>0) {
            if (sign(node.p[degree])==0) return root(out,node.hi,node.hi);
            continue;
        }
        if (node.depth==24) {
            // Sign change or one variation proves a root exists here. A near
            // miss/tangency that cannot be distinguished remains a diagnostic.
            if ((node.p[0].lo>0 && node.p[degree].hi<=0) || v==1)
                return root(out,node.lo,node.hi);
            out.status=Status::Precision;
            return out;
        }
        Node left,right;
        split(node,degree,left,right);
        stack[count++]=right; stack[count++]=left;
    }
    out.status=Status::Ok; out.location=Location::Outside;
    out.value=1; out.lower=out.upper=1;
    return out;
}
}
