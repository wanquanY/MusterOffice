#ifndef MO_ENCODE_H
#define MO_ENCODE_H
#include "mo_media.h"
#ifdef __cplusplus
extern "C" {
#endif
/* Explicit profiles, never inferred from a filename or silently substituted. */
enum { MO_ENCODE_H264_VT_AAC_MP4=1, MO_ENCODE_FFV1_PCM_MP4=2 };
/* Separate leaf profile: externally encoded, ordered AVC + local AAC/mux.
   Not advertised as a native video encoder by mo_encode_profiles(). */
enum { MO_ENCODE_EXTERNAL_AVC_AAC_MP4=4 };
typedef struct mo_encode mo_encode;
typedef struct {
    void *opaque;
    /* May short-write; zero, negative or oversized results fail the session.
       The host owns the candidate and must discard it on any error. */
    int32_t (*write_at)(void*, uint64_t, const uint8_t*, int32_t);
    int32_t (*cancelled)(void*);
} mo_encode_io;
typedef struct {
    int32_t profile, width, height, video_time_scale, frame_rate_num, frame_rate_den;
    int64_t video_duration, video_frames, video_bit_rate;
    int32_t audio_sample_rate, audio_channels, audio_bit_rate, gop_frames;
    int64_t audio_samples; /* zero means no audio stream */
} mo_encode_config;
typedef struct {
    uint64_t max_file_bytes, max_write_bytes, max_pending_bytes, max_frame_bytes;
    uint32_t max_pixels, max_packets, max_packet_bytes, max_pending_packets, max_steps, max_write_calls;
} mo_encode_limits;
typedef struct {
    int64_t pts, duration;
    const uint8_t *planes[3]; /* explicitly BT.709 limited-range, left-cosited I420 */
    uint64_t plane_bytes[3], strides[3];
} mo_encode_video_frame;
typedef struct {
    uint64_t byte_length, write_bytes, video_frames, audio_samples;
    uint64_t video_packets, audio_packets, peak_pending_bytes;
} mo_encode_stats;
MO_MEDIA_API uint32_t mo_encode_abi_version(void);
/* A compiled capability is not proof that the device can open a HW session. */
MO_MEDIA_API uint32_t mo_encode_profiles(void);
MO_MEDIA_API int32_t mo_encode_open(const mo_encode_io*, const mo_encode_config*, const mo_encode_limits*, mo_encode**, mo_media_error*);
/* AVC decoder configuration record with four-byte NAL lengths. Caller must
   establish BT.709 limited-range left-cosited 8-bit I420 codec configuration;
   the parser verifies dimensions/picture structure, not SPS color metadata.
   Input access
   units must be contiguous CFR in presentation/decode order, no B pictures.
   This explicit profile refuses reordering instead of guessing decode clocks.
   Final output remains a candidate requiring host verification/publication. */
MO_MEDIA_API int32_t mo_encode_open_avc(const mo_encode_io*, const mo_encode_config*, const mo_encode_limits*, const uint8_t*, uint32_t, mo_encode**, mo_media_error*);
MO_MEDIA_API int32_t mo_encode_avc(mo_encode*, int64_t pts, int64_t duration, const uint8_t*, uint32_t, int32_t key, mo_media_error*);
MO_MEDIA_API int32_t mo_encode_video(mo_encode*, const mo_encode_video_frame*, mo_media_error*);
MO_MEDIA_API int32_t mo_encode_audio(mo_encode*, int64_t first_sample, const float*, uint32_t frames, mo_media_error*);
/* Drains both encoders, interleaves remaining packets and writes the trailer.
   This is a byte candidate, not a persistence/publication receipt. */
MO_MEDIA_API int32_t mo_encode_finish(mo_encode*, mo_encode_stats*, mo_media_error*);
MO_MEDIA_API void mo_encode_close(mo_encode*);
#ifdef __cplusplus
}
#endif
#endif
