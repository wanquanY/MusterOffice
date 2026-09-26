// The exponent 1.875 is exactly 15/8. Three square roots and integer powers
// avoid platform libm pow implementations and runtime shader compilation.
SI F mo_office_power(F x) {
    F q = sqrt_(sqrt_(sqrt_(min(max(x, F0), F1))));
    F q2 = q*q, q4 = q2*q2, q8 = q4*q4;
    return ((q8*q4)*q2)*q;
}
SI F mo_office_channel(F rising, F falling, float first, float second) {
    F ratio = second > first ? rising : falling;
    return min(max(F_(first) + ratio*(second-first), F0), F1);
}
HIGHP_STAGE(mo_office_gradient, const SkRasterPipelineContexts::MoOfficeGradientCtx* c) {
    F t = min(max(r, F0), F1);
    if (c->midpoint < 1.0f) {
        t = if_then_else(t <= c->midpoint, t/c->midpoint, (F1-t)/(1.0f-c->midpoint));
    }
    // The scalar shader's alpha is the decal mask, not an interpolated alpha.
    a = a * (F_(c->first[3]) + t*(c->second[3]-c->first[3]));
    F rising = F1 - mo_office_power(F1-t), falling = mo_office_power(t);
    r = mo_office_channel(rising, falling, c->first[0], c->second[0]) * a;
    g = mo_office_channel(rising, falling, c->first[1], c->second[1]) * a;
    b = mo_office_channel(rising, falling, c->first[2], c->second[2]) * a;
}
