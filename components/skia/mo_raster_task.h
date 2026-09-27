#pragma once
#include "mo_raster_storage.h"
#include <coroutine>
#include <cstdlib>
#include <type_traits>
#include <utility>

namespace mo {
// Private, thread-confined scan-conversion continuation. A task owns its entire
// nested computation; suspension keeps arenas, edges, blitters and their borrows
// alive. No scheduler, clock, OS thread or serializable continuation is involved.
class RasterTask {
public:
    struct promise_type;
    using Handle = std::coroutine_handle<promise_type>;
    enum class Status { Yielded, Complete, AllocationFailure };
    RasterTask() = default;
    RasterTask(const RasterTask&) = delete;
    RasterTask& operator=(const RasterTask&) = delete;
    RasterTask(RasterTask&& other) noexcept
        : handle_(std::exchange(other.handle_, {})), failed_(std::exchange(other.failed_, false)) {}
    RasterTask& operator=(RasterTask&& other) noexcept {
        if (this != &other) {
            if (handle_) handle_.destroy();
            handle_ = std::exchange(other.handle_, {});
            failed_ = std::exchange(other.failed_, false);
        }
        return *this;
    }
    ~RasterTask() { if (handle_) handle_.destroy(); }
    Status step();
    // Upstream synchronous entry points drain exactly the same computation.
    // Their void API cannot report OOM; preserve Skia's abort/isolation rule.
    void drain() {
        Status status;
        do { status = step(); } while (status == Status::Yielded);
        if (status == Status::AllocationFailure) std::abort();
    }
    struct Awaiter {
        RasterTask& task;
        bool await_ready() const { return !task.handle_ && !task.failed_; }
        std::coroutine_handle<> await_suspend(Handle parent) const;
        void await_resume() const {}
    };
    Awaiter operator co_await() && { return {*this}; }
private:
    explicit RasterTask(Handle handle) : handle_(handle) {}
    Handle handle_{};
    bool failed_ = false;
};

struct RasterTask::promise_type {
    Handle child{}, parent{};
    bool failed = false;
    static void* operator new(size_t bytes) noexcept { return RasterStorage::allocate(nullptr, bytes); }
    // Coroutine allocation receives the function's arguments (including the
    // implicit object for member functions). Only the explicit storage pointer
    // selects an owner; unrelated arguments cannot affect allocation policy.
    template<class... Args>
    static void* operator new(size_t bytes, Args&&... args) noexcept {
        RasterStorage* storage = nullptr;
        ([&] {
            if constexpr (std::is_same_v<std::remove_cvref_t<Args>, RasterStorage*>) storage = args;
        }(), ...);
        return RasterStorage::allocate(storage, bytes);
    }
    static void operator delete(void* p, size_t) noexcept { RasterStorage::release(p); }
    static RasterTask get_return_object_on_allocation_failure() noexcept {
        RasterTask task; task.failed_ = true; return task;
    }
    RasterTask get_return_object() { return RasterTask(Handle::from_promise(*this)); }
    std::suspend_always initial_suspend() const noexcept { return {}; }
    std::suspend_always yield_value(int) const noexcept { return {}; }
    struct Final {
        bool await_ready() const noexcept { return false; }
        std::coroutine_handle<> await_suspend(Handle handle) const noexcept {
            auto parent = handle.promise().parent;
            if (parent) { parent.promise().child = {}; return parent; }
            return std::noop_coroutine();
        }
        void await_resume() const noexcept {}
    };
    Final final_suspend() const noexcept { return {}; }
    void return_void() const noexcept {}
    void unhandled_exception() const noexcept { std::abort(); }
};

inline std::coroutine_handle<> RasterTask::Awaiter::await_suspend(Handle parent) const {
    if (!task.handle_) {
        parent.promise().failed = true;
        return std::noop_coroutine();
    }
    parent.promise().child = task.handle_;
    task.handle_.promise().parent = parent;
    return task.handle_;
}

inline RasterTask::Status RasterTask::step() {
    if (failed_) return Status::AllocationFailure;
    if (!handle_ || handle_.done()) return Status::Complete;
    auto leaf = handle_;
    while (leaf.promise().child) leaf = leaf.promise().child;
    if (leaf.promise().failed) return Status::AllocationFailure;
    leaf.resume();
    leaf = handle_;
    while (leaf.promise().child) leaf = leaf.promise().child;
    if (leaf.promise().failed) return Status::AllocationFailure;
    return handle_.done() ? Status::Complete : Status::Yielded;
}
} // namespace mo
