#pragma once
#include <cstddef>
#include <cstdlib>
#include <limits>
#include <new>
#include <vector>

namespace mo_stretch {
struct Budget {
  size_t live, peak, limit;
};
inline thread_local Budget *current = nullptr;
#ifdef MO_STRETCH_FAULT_TEST
inline thread_local int64_t remaining = -1;
#endif
struct Scope {
  Budget *previous;
  explicit Scope(Budget &budget) : previous(current) { current = &budget; }
  ~Scope() { current = previous; }
};
// Every upstream vector retains its instance's budget. No global new override,
// no host allocation interposition, and no cross-instance shared failure state.
template<class T> struct Allocator {
  using value_type = T;
  Budget *budget = current;
  Allocator() = default;
  template<class U> Allocator(const Allocator<U> &other) : budget(other.budget) {}
  T *allocate(size_t count) {
    if (!budget || count > std::numeric_limits<size_t>::max()/sizeof(T)) throw std::bad_alloc();
    const size_t bytes = count*sizeof(T);
    if (bytes > budget->limit || budget->live > budget->limit - bytes) throw std::bad_alloc();
#ifdef MO_STRETCH_FAULT_TEST
    if (remaining == 0) throw std::bad_alloc();
    if (remaining > 0) --remaining;
#endif
    auto result = static_cast<T *>(std::malloc(bytes ? bytes : 1));
    if (!result) throw std::bad_alloc();
    budget->live += bytes;
    if (budget->live > budget->peak) budget->peak = budget->live;
    return result;
  }
  void deallocate(T *p, size_t count) noexcept {
    budget->live -= count*sizeof(T);
    std::free(p);
  }
  template<class U> bool operator==(const Allocator<U> &other) const { return budget == other.budget; }
  template<class U> bool operator!=(const Allocator<U> &other) const { return !(*this == other); }
};
template<class T> using Vector = std::vector<T, Allocator<T>>;

// Fix the distribution mapping as well as the standard-specified minstd_rand
// generator. std::uniform_real_distribution is otherwise library-dependent.
template<class T> struct UniformReal {
  T low, high;
  UniformReal(T low_, T high_) : low(low_), high(high_) {}
  template<class Engine> T operator()(Engine &engine) {
    double fraction = double(engine() - Engine::min()) /
        (double(Engine::max()) - double(Engine::min()) + 1.0);
    return T(double(low) + fraction*(double(high) - double(low)));
  }
};
}
