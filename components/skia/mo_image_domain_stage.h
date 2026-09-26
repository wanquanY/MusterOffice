// Included inside Skia's high-precision raster pipeline namespace. The fixed
// stage uses its SIMD lanes, bounded gathers and the shared output compositor.
// No pixel copies, runtime shader compiler, per-pixel callback or external I/O.
struct MoDomainColor { F r, g, b, a; };

SI MoDomainColor mo_domain_texel(const SkRasterPipelineContexts::MoImageDomainCtx* c, F x, F y) {
    // Clamp before conversion/gather even for inactive/transparent lanes.
    U32 ix = trunc_(min(max(x, F0), F_(c->width - 1)));
    U32 iy = trunc_(min(max(y, F0), F_(c->height - 1)));
    U32 pixel = gather_unaligned(c->pixels, iy * c->stride + ix);
    MoDomainColor color;
    from_8888(pixel, &color.r, &color.g, &color.b, &color.a);
    // In-bounds subpixel domains still reconstruct the image's edge texel;
    // otherwise an opaque crop narrower than one texel would gain transparency.
    // Transparent padding is introduced only on axes the domain actually outsets.
    if (c->decalX || c->decalY) {
        auto valid = (c->decalX ? ((x >= F0) & (x < F_(c->width))) : (F0 == F0)) &
                     (c->decalY ? ((y >= F0) & (y < F_(c->height))) : (F0 == F0));
        color.r = if_then_else(valid, color.r, F0);
        color.g = if_then_else(valid, color.g, F0);
        color.b = if_then_else(valid, color.b, F0);
        color.a = if_then_else(valid, color.a, F0);
    }
    return color;
}
SI MoDomainColor mo_domain_mix(MoDomainColor x, MoDomainColor y, F weight) {
    return {lerp(x.r,y.r,weight), lerp(x.g,y.g,weight), lerp(x.b,y.b,weight), lerp(x.a,y.a,weight)};
}
SI MoDomainColor mo_domain_linear(const SkRasterPipelineContexts::MoImageDomainCtx* c, F x, F y) {
    F ix = floor_(x - 0.5f), iy = floor_(y - 0.5f);
    F fx = (x - 0.5f) - ix, fy = (y - 0.5f) - iy;
    auto top = mo_domain_mix(mo_domain_texel(c,ix,iy), mo_domain_texel(c,ix+1.0f,iy), fx);
    auto bottom = mo_domain_mix(mo_domain_texel(c,ix,iy+1.0f), mo_domain_texel(c,ix+1.0f,iy+1.0f), fx);
    return mo_domain_mix(top,bottom,fy);
}
SI F mo_domain_wrap(F x, float lo, float hi, uint32_t mode) {
    if (mode == 1 || mode == 2) {
        const float length = hi - lo, period = mode == 1 ? length : 2 * length;
        F distance = x - lo;
        F offset = distance - floor_(distance / F_(period)) * period;
        if (mode == 2) offset = min(offset, F_(period) - offset);
        return min(max(F_(lo) + offset, F_(lo)), F_(hi));
    }
    return x;
}
HIGHP_STAGE(mo_image_domain, const SkRasterPipelineContexts::MoImageDomainCtx* c) {
    F x = mo_domain_wrap(r,c->left,c->right,c->tileX);
    F y = mo_domain_wrap(g,c->top,c->bottom,c->tileY);
    MoDomainColor color;
    F coverage = F1;
    if (c->linear) {
        F qx = min(max(x,F_(c->minX)),F_(c->maxX));
        F qy = min(max(y,F_(c->minY)),F_(c->maxY));
        F ex = x - qx, ey = y - qy;
        color = mo_domain_linear(c,qx,qy);
        // Interpolate across repeat seams without a padded/resampled bitmap.
        // The common interior only performs the initial four bounded gathers.
        const bool rx = c->tileX == 1 && any(ex != F0);
        const bool ry = c->tileY == 1 && any(ey != F0);
        F oppositeX = if_then_else(ex > F0,F_(c->minX),F_(c->maxX));
        F oppositeY = if_then_else(ey > F0,F_(c->minY),F_(c->maxY));
        if (rx) {
            color = mo_domain_mix(color,mo_domain_linear(c,oppositeX,qy),abs_(ex));
        }
        if (ry) {
            auto other = mo_domain_linear(c,qx,oppositeY);
            if (rx) other = mo_domain_mix(other,mo_domain_linear(c,oppositeX,oppositeY),abs_(ex));
            color = mo_domain_mix(color,other,abs_(ey));
        }
        if (c->tileX == 3) coverage *= max(F0,F1 - abs_(ex));
        if (c->tileY == 3) coverage *= max(F0,F1 - abs_(ey));
    } else {
        F qx = min(max(x,F_(c->minX)),F_(c->maxX));
        F qy = min(max(y,F_(c->minY)),F_(c->maxY));
        // Same left/up tie direction as the ordinary Skia CPU image path.
        color = mo_domain_texel(c,-floor_(-qx)-1.0f,-floor_(-qy)-1.0f);
        if (c->tileX == 3) coverage = if_then_else((x > F_(c->left)) & (x <= F_(c->right)),coverage,F0);
        if (c->tileY == 3) coverage = if_then_else((y > F_(c->top)) & (y <= F_(c->bottom)),coverage,F0);
    }
    r = color.r * coverage; g = color.g * coverage;
    b = color.b * coverage; a = color.a * coverage;
}
