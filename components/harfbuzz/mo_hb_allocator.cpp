#include "mo_hb_allocator.h"
#include <atomic>
#include <cstddef>
#include <cstdlib>
#include <cstring>
#include <limits>

namespace {
// This hard component ceiling includes retained HarfBuzz global allocations.
// Input font/text buffers remain caller-owned and are budgeted by the Rust host.
constexpr size_t ceiling=256*1024*1024;
std::atomic<size_t> live{0};
std::atomic<bool> invalidated{false};
thread_local bool failed=false;
#ifdef MO_HB_FAULT_TEST
thread_local size_t remaining=std::numeric_limits<size_t>::max();
#endif
union Header { size_t bytes; std::max_align_t alignment; };
bool reserve(size_t amount) {
  size_t current=live.load(std::memory_order_relaxed);
  do {
    if (amount>ceiling || current>ceiling-amount) { failed=true; invalidated.store(true,std::memory_order_release); return false; }
  } while (!live.compare_exchange_weak(current,current+amount,std::memory_order_relaxed));
  return true;
}
bool injected_failure() {
#ifdef MO_HB_FAULT_TEST
  if (!remaining) { failed=true; invalidated.store(true,std::memory_order_release); return true; }
  if (remaining!=std::numeric_limits<size_t>::max()) --remaining;
#endif
  return false;
}
}
extern "C" void mo_hb_alloc_begin(void) { failed=false; }
extern "C" int mo_hb_alloc_failed(void) { return failed; }
extern "C" void mo_hb_invalidate(void) { invalidated.store(true,std::memory_order_release); }
extern "C" int mo_hb_instance_invalid(void) { return invalidated.load(std::memory_order_acquire); }
extern "C" size_t mo_hb_alloc_live(void) { return live.load(std::memory_order_relaxed); }
#ifdef MO_HB_FAULT_TEST
extern "C" void mo_hb_fail_after(size_t n) { remaining=n; }
#endif
extern "C" void *mo_hb_alloc(size_t length) {
  if (injected_failure()) return nullptr;
  if (length>ceiling-sizeof(Header)) { failed=true; invalidated.store(true,std::memory_order_release); return nullptr; }
  const size_t total=sizeof(Header)+(length ? length : 1);
  if (!reserve(total)) return nullptr;
  auto *header=static_cast<Header *>(std::malloc(total));
  if (!header) { live.fetch_sub(total,std::memory_order_relaxed);failed=true; invalidated.store(true,std::memory_order_release);return nullptr; }
  header->bytes=total;
  return header+1;
}
extern "C" void mo_hb_dealloc(void *pointer) {
  if (!pointer) return;
  auto *header=static_cast<Header *>(pointer)-1;
  live.fetch_sub(header->bytes,std::memory_order_relaxed);
  std::free(header);
}
extern "C" void *mo_hb_calloc(size_t count,size_t length) {
  if (length && count>std::numeric_limits<size_t>::max()/length) { failed=true; invalidated.store(true,std::memory_order_release);return nullptr; }
  const size_t total=count*length;
  void *pointer=mo_hb_alloc(total);
  if (pointer) std::memset(pointer,0,total);
  return pointer;
}
extern "C" void *mo_hb_realloc(void *pointer,size_t length) {
  if (!pointer) return mo_hb_alloc(length);
  if (!length) { mo_hb_dealloc(pointer);return nullptr; }
  if (injected_failure()) return nullptr;
  if (length>ceiling-sizeof(Header)) { failed=true; invalidated.store(true,std::memory_order_release);return nullptr; }
  auto *old=static_cast<Header *>(pointer)-1;
  const size_t before=old->bytes, after=sizeof(Header)+length;
  if (after>before && !reserve(after-before)) return nullptr;
  auto *replacement=static_cast<Header *>(std::realloc(old,after));
  if (!replacement) {
    if (after>before) live.fetch_sub(after-before,std::memory_order_relaxed);
    failed=true; invalidated.store(true,std::memory_order_release);return nullptr;
  }
  replacement->bytes=after;
  if (before>after) live.fetch_sub(before-after,std::memory_order_relaxed);
  return replacement+1;
}
