#pragma once
#include <stddef.h>
#ifdef __cplusplus
extern "C" {
#endif
void *mo_hb_alloc(size_t length);
void *mo_hb_calloc(size_t count, size_t length);
void *mo_hb_realloc(void *pointer, size_t length);
void mo_hb_dealloc(void *pointer);
void mo_hb_alloc_begin(void);
int mo_hb_alloc_failed(void);
int mo_hb_instance_invalid(void);
void mo_hb_invalidate(void);
size_t mo_hb_alloc_live(void);
#ifdef MO_HB_FAULT_TEST
void mo_hb_fail_after(size_t successful_allocations);
#endif
#ifdef __cplusplus
}
#endif
