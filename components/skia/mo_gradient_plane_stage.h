// Unit tile coordinates are independent of the scalar color-ramp coordinate.
// A fixed CPU vector stage: no pixel buffer, runtime compiler or per-tile draw.
HIGHP_STAGE(mo_gradient_plane, const SkRasterPipelineContexts::MoGradientPlaneCtx* c) {
    float xs[N],ys[N];store(xs,r);store(ys,g);
    const auto count=std::min(size_t(*c->tail),N);
    c->coordinates(c->ellipticUser,xs,ys,count);
    if (c->kind == 4) {
        float result[N]{};
        c->elliptic(c->ellipticUser,result,xs,ys,count);
        r=sk_unaligned_load<F>(result);
    } else if (c->kind == 3) {
        const F u=sk_unaligned_load<F>(xs),v=sk_unaligned_load<F>(ys);
        r = F0;
        if (c->values[0] > 0) r = max(r, F1 - u*c->values[0]);
        if (c->values[1] > 0) r = max(r, F1 - v*c->values[1]);
        if (c->values[2] > 0) r = max(r, F1 - (F1-u)*c->values[2]);
        if (c->values[3] > 0) r = max(r, F1 - (F1-v)*c->values[3]);
    } else {
        const F u=sk_unaligned_load<F>(xs),v=sk_unaligned_load<F>(ys);
        r = (u * c->values[0] + v * c->values[1]) + c->values[2];
    }
    g = F0;
}
