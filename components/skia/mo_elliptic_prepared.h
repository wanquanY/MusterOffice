#pragma once
#include "mo_elliptic_geometry.h"
namespace mo::elliptic {
// Immutable paint-level specialization. The general first-root implementation
// remains shared; uncertain specialized classifications use that same kernel.
class PreparedField {
public:
    explicit PreparedField(Field field);
    Result evaluate(float x, float y, uint32_t budget=max_nodes) const;
private:
    detail::PreparedGeometry geometry_;
    bool nested_;
    bool evaluate_nested(float x, float y, Result&) const;
    bool concentric_;
    double radius_, radius2_, denominator_lo_, denominator_hi_;
};
}
