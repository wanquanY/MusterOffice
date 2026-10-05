#ifndef MO_MEDIA_H
#define MO_MEDIA_H
#include <stddef.h>
#include <stdint.h>
#if defined(_WIN32)
#define MO_MEDIA_API __declspec(dllexport)
#else
#define MO_MEDIA_API __attribute__((visibility("default")))
#endif
#ifdef __cplusplus
extern "C" {
#endif

enum { MO_MEDIA_OK=0, MO_MEDIA_FRAME=1, MO_MEDIA_EOF=2,
       MO_MEDIA_INVALID=-1, MO_MEDIA_IO=-2, MO_MEDIA_CANCELLED=-3,
       MO_MEDIA_LIMIT=-4, MO_MEDIA_UNSUPPORTED=-5, MO_MEDIA_DECODE=-6, MO_MEDIA_STATE=-7 };
enum { MO_MEDIA_VIDEO=1, MO_MEDIA_AUDIO=2, MO_MEDIA_OTHER=3 };
typedef struct mo_media mo_media;
typedef struct { int32_t status, upstream_code; char message[192]; } mo_media_error;
typedef struct {
    void *opaque;
    uint64_t byte_length;
    /* Return bytes read, zero at EOF, -1 on I/O failure. Never unwind through this ABI. */
    int32_t (*read_at)(void *opaque, uint64_t offset, uint8_t *buffer, int32_t capacity);
    int32_t (*cancelled)(void *opaque);
} mo_media_io;
typedef struct {
    uint64_t max_input_bytes, max_read_bytes, max_frame_bytes;
    uint32_t max_streams, max_pixels, max_audio_samples, max_channels, max_decode_steps;
} mo_media_limits;
typedef struct {
    int32_t kind, index, time_base_num, time_base_den;
    int64_t start, duration;
    int32_t has_start, has_duration, width, height, sample_rate, channels;
    int32_t frame_rate_num, frame_rate_den, sar_num, sar_den;
    int32_t color_primaries, color_transfer, color_matrix, color_range, chroma_location;
    int32_t has_display_matrix, display_matrix[9];
    char codec[64], channel_layout[128];
} mo_media_stream_info;
typedef struct {
    uint64_t generation;
    int32_t kind, time_base_num, time_base_den, has_pts, has_duration;
    int64_t pts, duration;
    int32_t width, height, sample_rate, channels, samples, planes;
    int32_t color_primaries, color_transfer, color_matrix, color_range, chroma_location;
    int32_t sar_num, sar_den, alpha_mode, key_frame, interlaced, top_field_first;
    char format[64], channel_layout[128];
} mo_media_frame_info;
typedef struct { uint64_t row_bytes, rows; } mo_media_plane_info;

MO_MEDIA_API uint32_t mo_media_abi_version(void);
MO_MEDIA_API const char *mo_media_upstream_version(void);
MO_MEDIA_API const char *mo_media_upstream_license(void);
/* Host keeps callbacks/context alive until close. A session is single-thread-affine. */
MO_MEDIA_API int32_t mo_media_open(const mo_media_io*, const mo_media_limits*, mo_media**, mo_media_error*);
MO_MEDIA_API void mo_media_close(mo_media*);
MO_MEDIA_API uint32_t mo_media_stream_count(const mo_media*);
MO_MEDIA_API int32_t mo_media_stream(const mo_media*, uint32_t, mo_media_stream_info*, mo_media_error*);
/* One selected stream per session; use independent sessions for audio/video access. */
MO_MEDIA_API int32_t mo_media_select(mo_media*, uint32_t, mo_media_error*);
/* Returns the next presentation frame, EOF, or an error. Frames stay valid until next/seek/close. */
MO_MEDIA_API int32_t mo_media_next(mo_media*, mo_media_frame_info*, mo_media_error*);
/* Timestamp is in the selected stream's timebase. Seek is to a preceding random access point;
   decode/discard until the desired presentation interval. A successful seek increments generation. */
MO_MEDIA_API int32_t mo_media_seek(mo_media*, int64_t timestamp, mo_media_error*);
MO_MEDIA_API int32_t mo_media_plane(const mo_media*, uint32_t, mo_media_plane_info*, mo_media_error*);
/* Copies active rows only, never decoder padding. No implicit color conversion or downmix. */
MO_MEDIA_API int32_t mo_media_copy_plane(const mo_media*, uint32_t, uint8_t*, uint64_t capacity, uint64_t stride, mo_media_error*);
#ifdef __cplusplus
}
#endif
#endif
