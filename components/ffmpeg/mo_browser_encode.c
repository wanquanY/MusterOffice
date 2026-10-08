/* Thin, synchronous Wasm boundary. The host owns seekable output and cancellation.
   No filesystem, task authority, clock conversion or media algorithm lives here. */
#include "mo_encode.h"
#include <emscripten.h>
#include <stdlib.h>

typedef struct {
    mo_encode_config config;
    mo_encode_limits limits;
    mo_media_error error;
    mo_encode_stats stats;
    mo_encode *encoder;
} mo_browser_encode;

EM_JS(int, browser_write, (void *id, uint64_t offset, const uint8_t *data, int size), {
    try { return Module.moWriteAt(id, offset, HEAPU8.subarray(data, data + size)); }
    catch (error) { Module.moCallbackError(id, error); return -1; }
});
EM_JS(int, browser_cancelled, (void *id), {
    try { return Module.moCancelled(id) ? 1 : 0; }
    catch (error) { Module.moCallbackError(id, error); return 1; }
});

/* Layout comes from the compiler. JS never assumes pointer size/alignment. */
#define C(field) offsetof(mo_encode_config,field)
#define L(field) offsetof(mo_encode_limits,field)
#define S(field) offsetof(mo_encode_stats,field)
MO_MEDIA_API const uint32_t *mo_browser_layout(void) {
    static const uint32_t layout[]={1,
        sizeof(mo_encode_config),C(profile),C(width),C(height),C(video_time_scale),C(frame_rate_num),C(frame_rate_den),
        C(video_duration),C(video_frames),C(video_bit_rate),C(audio_sample_rate),C(audio_channels),C(audio_bit_rate),C(gop_frames),C(audio_samples),
        sizeof(mo_encode_limits),L(max_file_bytes),L(max_write_bytes),L(max_pending_bytes),L(max_frame_bytes),
        L(max_pixels),L(max_packets),L(max_packet_bytes),L(max_pending_packets),L(max_steps),L(max_write_calls),
        sizeof(mo_encode_stats),S(byte_length),S(write_bytes),S(video_frames),S(audio_samples),S(video_packets),S(audio_packets),S(peak_pending_bytes),
        sizeof(mo_media_error),offsetof(mo_media_error,status),offsetof(mo_media_error,upstream_code),offsetof(mo_media_error,message)};
    return layout;
}
MO_MEDIA_API mo_browser_encode *mo_browser_create(void) {return calloc(1,sizeof(mo_browser_encode));}
MO_MEDIA_API mo_encode_config *mo_browser_config(mo_browser_encode *s) {return &s->config;}
MO_MEDIA_API mo_encode_limits *mo_browser_limits(mo_browser_encode *s) {return &s->limits;}
MO_MEDIA_API mo_media_error *mo_browser_error(mo_browser_encode *s) {return &s->error;}
MO_MEDIA_API mo_encode_stats *mo_browser_stats(mo_browser_encode *s) {return &s->stats;}
MO_MEDIA_API int mo_browser_open(mo_browser_encode *s,const uint8_t *description,uint32_t bytes) {
    if (s->encoder) return MO_MEDIA_STATE;
    mo_encode_io io={s,browser_write,browser_cancelled};
    return mo_encode_open_avc(&io,&s->config,&s->limits,description,bytes,&s->encoder,&s->error);
}
MO_MEDIA_API int mo_browser_video(mo_browser_encode *s,int64_t pts,int64_t duration,const uint8_t *data,uint32_t bytes,int key) {
    return mo_encode_avc(s->encoder,pts,duration,data,bytes,key,&s->error);
}
MO_MEDIA_API int mo_browser_audio(mo_browser_encode *s,int64_t first,const float *data,uint32_t frames) {
    return mo_encode_audio(s->encoder,first,data,frames,&s->error);
}
MO_MEDIA_API int mo_browser_finish(mo_browser_encode *s) {return mo_encode_finish(s->encoder,&s->stats,&s->error);}
MO_MEDIA_API void mo_browser_close(mo_browser_encode *s) {if (s) {mo_encode_close(s->encoder);free(s);}}
