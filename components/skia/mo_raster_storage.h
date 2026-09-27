#pragma once
#include <cstddef>
#include <cstdlib>
#include <limits>

namespace mo {
// Private coroutine storage, owned by one raster operation. Freed frames may be
// reused by later draws in that operation; nothing survives the owner's drop.
// Addresses stay stable while suspended. No global or thread-local allocator.
class RasterStorage {
    struct alignas(std::max_align_t) Block {
        RasterStorage* owner;
        Block* all_next;
        Block* free_next;
        size_t capacity;
    };
    Block* all_ = nullptr;
    Block* free_ = nullptr;
    size_t blocks_ = 0, active_ = 0;
public:
    RasterStorage() = default;
    RasterStorage(const RasterStorage&) = delete;
    RasterStorage& operator=(const RasterStorage&) = delete;
    ~RasterStorage() {
        // The owner must destroy its suspended computation before its storage.
        if (active_) std::abort();
        while (all_) {
            auto* block = all_;
            all_ = block->all_next;
            std::free(block);
        }
    }
    size_t block_count() const { return blocks_; }
    size_t active_count() const { return active_; }
    static void* allocate(RasterStorage* owner, size_t bytes) noexcept {
        if (owner) {
            // Best fit keeps large scan frames available to their callers. The
            // live frame types/depth come from the fixed internal call graph,
            // rather than object count, path size or user-provided size classes.
            Block** best = nullptr;
            for (auto** p = &owner->free_; *p; p = &(*p)->free_next) {
                if ((*p)->capacity >= bytes && (!best || (*p)->capacity < (*best)->capacity)) {
                    best = p;
                    if ((*p)->capacity == bytes) break;
                }
            }
            if (best) {
                auto* block = *best;
                *best = block->free_next;
                ++owner->active_;
                return block + 1;
            }
        }
        if (bytes > std::numeric_limits<size_t>::max() - sizeof(Block)) return nullptr;
        auto* block = static_cast<Block*>(std::malloc(sizeof(Block) + bytes));
        if (!block) return nullptr;
        *block = {owner, owner ? owner->all_ : nullptr, nullptr, bytes};
        if (owner) {
            owner->all_ = block;
            ++owner->blocks_;
            ++owner->active_;
        }
        return block + 1;
    }
    static void release(void* memory) noexcept {
        auto* block = static_cast<Block*>(memory) - 1;
        if (auto* owner = block->owner) {
            block->free_next = owner->free_;
            owner->free_ = block;
            --owner->active_;
        } else {
            std::free(block);
        }
    }
};
} // namespace mo
