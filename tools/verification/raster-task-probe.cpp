#include "mo_raster_task.h"
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <limits>
#include <vector>

struct Guard {
    int& active;
    explicit Guard(int& count) : active(count) { ++active; }
    ~Guard() { --active; }
};
static mo::RasterTask child(int& active, std::vector<int>& values, int base) {
    Guard guard(active);
    for (int i = 0; i < 3; ++i) { values.push_back(base+i); co_yield 0; }
}
static mo::RasterTask parent(int& active, std::vector<int>& values, int base) {
    Guard guard(active);
    co_await child(active, values, base);
    co_await child(active, values, base+3);
}
static mo::RasterTask injected_child_failure(int& active, bool& resumed) {
    Guard guard(active);
    // Exercise the exact value returned by a null coroutine allocation, without
    // claiming to have forced the platform allocator to exhaust physical memory.
    co_await mo::RasterTask::promise_type::get_return_object_on_allocation_failure();
    resumed = true;
}
static mo::RasterTask parent_failure(int& active, bool& resumed) {
    Guard guard(active);
    co_await injected_child_failure(active, resumed);
    resumed = true;
}
struct StoredTasks {
    mo::RasterTask child(mo::RasterStorage* storage, int& active, std::vector<int>& values, int base) {
        Guard guard(active);
        for (int i = 0; i < 3; ++i) { values.push_back(base+i); co_yield 0; }
    }
};
static mo::RasterTask stored_parent(mo::RasterStorage* storage, StoredTasks& tasks,
                                   int& active, std::vector<int>& values, int base) {
    Guard guard(active);
    co_await tasks.child(storage, active, values, base);
    co_await tasks.child(storage, active, values, base+3);
}
static int storage_checks() {
    using Status = mo::RasterTask::Status;
    mo::RasterStorage first, second;
    // Allocation is max_align_t aligned; best-fit reuse never overwrites another
    // suspended frame and the two operation owners never exchange storage.
    void* small = mo::RasterStorage::allocate(&first, 32);
    void* large = mo::RasterStorage::allocate(&first, 4096);
    if (!small || !large || reinterpret_cast<uintptr_t>(large) % alignof(std::max_align_t)) return 20;
    std::memset(large, 0x5a, 4096);
    mo::RasterStorage::release(small);
    if (mo::RasterStorage::allocate(&first, 24) != small) return 21;
    for (int i = 0; i < 4096; ++i) if (static_cast<unsigned char*>(large)[i] != 0x5a) return 22;
    mo::RasterStorage::release(small);
    mo::RasterStorage::release(large);
    if (first.active_count() || first.block_count() != 2) return 23;
    void* other = mo::RasterStorage::allocate(&second, 24);
    if (!other || other == small || other == large) return 24;
    mo::RasterStorage::release(other);
    if (mo::RasterStorage::allocate(&first, std::numeric_limits<size_t>::max())) return 25;
    if (first.active_count() || first.block_count() != 2) return 26;

    StoredTasks tasks;
    int active = 0;
    size_t first_blocks = 0, second_blocks = 0;
    // Repeated independent owners exercise both member/free coroutine argument
    // allocation, normal completion, moves, and every suspension drop position.
    for (int iteration = 0; iteration < 1000; ++iteration) {
        std::vector<int> one_values, two_values;
        {
            auto one = stored_parent(&first, tasks, active, one_values, 0);
            auto two = stored_parent(&second, tasks, active, two_values, 20);
            for (int step = 0; step < iteration % 7; ++step) {
                if (one.step() != Status::Yielded || two.step() != Status::Yielded) return 27;
                if (one_values.back() != step || two_values.back() != 20+step || active != 4) return 28;
                if (!first.active_count() || !second.active_count()) return 29;
            }
            auto moved = std::move(one);
            if (one.step() != Status::Complete) return 30;
            two.drain();
            if (two_values != std::vector<int>({20,21,22,23,24,25})) return 31;
        }
        if (active || first.active_count() || second.active_count()) return 32;
        if (iteration == 7) { first_blocks = first.block_count(); second_blocks = second.block_count(); }
        if (iteration > 7 && (first.block_count() != first_blocks || second.block_count() != second_blocks)) return 33;
    }
    return 0;
}
int main() {
    using Status = mo::RasterTask::Status;
    int active = 0;
    std::vector<int> values;
    for (int cancel = 0; cancel <= 6; ++cancel) {
        {
            auto task = parent(active, values, 10);
            for (int i = 0; i < cancel; ++i) if (task.step() != Status::Yielded) return 1;
            if (cancel && active != 2) return 2;
            // Move must transfer suspended child ownership, not resume/recreate it.
            auto moved = std::move(task);
            if (task.step() != Status::Complete) return 3;
        }
        if (active) return 4;
        values.clear();
    }
    {
        auto one = parent(active, values, 0);
        std::vector<int> other_values;
        auto two = parent(active, other_values, 20);
        for (int i = 0; i < 6; ++i) {
            if (one.step() != Status::Yielded || two.step() != Status::Yielded) return 5;
            if (values.back() != i || other_values.back() != 20+i || active != 4) return 6;
        }
        if (one.step() != Status::Complete || two.step() != Status::Complete || active) return 7;
        if (one.step() != Status::Complete) return 8;
    }
    bool resumed = false;
    {
        auto failure = parent_failure(active, resumed);
        if (failure.step() != Status::AllocationFailure || active != 2 || resumed) return 9;
        if (failure.step() != Status::AllocationFailure || resumed) return 10;
        failure = {};
        if (active) return 11;
    }
    values.clear();
    parent(active, values, 0).drain();
    if (active || values != std::vector<int>({0,1,2,3,4,5})) return 12;
    if (int failure = storage_checks()) return failure;
    std::puts("{\"status\":\"passed\",\"cancellationPoints\":7,\"interleavedOwners\":2,\"injectedChildFailure\":true,\"storageReuseIterations\":1000,\"storageOwners\":2}");
}
