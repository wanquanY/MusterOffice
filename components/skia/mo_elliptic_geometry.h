#pragma once
#include "mo_elliptic_field.h"
#include "mo_elliptic_interval.h"
#include <array>
namespace mo::elliptic::detail {
using Polynomial = std::array<Interval,5>;
// Immutable coefficients depend on the field, not on a sample position.
struct PreparedGeometry {
    Field field;
    std::array<double,3> bx{},by{};
    Polynomial radius_product{};
    explicit PreparedGeometry(Field);
};
Result evaluate_prepared(const PreparedGeometry&, float x, float y, uint32_t budget);
}
