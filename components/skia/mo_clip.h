#pragma once
#include <algorithm>
#include <bit>
#include <cmath>
#include <cstdint>

// The table is validated before traversal. References on wire are 1-based;
// 0 denotes the viewport, including draws which leave all document clips.
struct MoClipStack {
    uint32_t nodes[64]{};
    uint32_t length = 0;
    uint32_t transition(const uint32_t* clips, uint32_t target) {
        uint32_t next[64], count = 0;
        while (target) {
            next[count++] = target;
            target = clips[(target - 1) * 4];
        }
        std::reverse(next, next + count);
        uint32_t common = 0;
        while (common < std::min(count, length) && nodes[common] == next[common]) ++common;
        std::copy(next, next + count, nodes);
        length = count;
        return common;
    }
};

// No Skia calls or allocation. Enforce path references, acyclic parent order,
// finite placement, device bounds and depth for every node, including unused.
inline int mo_validate_clips(const uint32_t* clips, uint32_t count,
                             uint32_t paths, const uint32_t* commands, const float (*bounds)[4]) {
    if (count > 8192) return 3;
    uint8_t depths[8192]{};
    uint64_t placement_work = 0;
    for (uint32_t i = 0; i < count; ++i) {
        const auto* clip = clips + i * 4;
        if (clip[0] > i || clip[1] >= paths) return 1;
        placement_work += commands[clip[1]];
        if (placement_work > 1048576) return 3;
        const uint32_t depth = clip[0] ? uint32_t(depths[clip[0] - 1]) + 1 : 1;
        if (depth > 64) return 3;
        depths[i] = uint8_t(depth);
        for (uint32_t k = 0; k < 4; ++k) {
            const float origin = std::bit_cast<float>(clip[2 + k % 2]);
            if (!std::isfinite(origin) || std::abs(origin) > 32768.0f) return 1;
            if (std::abs(double(bounds[clip[1]][k]) + origin) > 32768.0) return 3;
        }
    }
    return 0;
}
