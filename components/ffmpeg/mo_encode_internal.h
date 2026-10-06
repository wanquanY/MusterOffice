#ifndef MO_ENCODE_INTERNAL_H
#define MO_ENCODE_INTERNAL_H
#include "mo_encode.h"
#include "mo_media_internal.h"
#include <math.h>
#include <libavutil/opt.h>
#define MO_ENCODE_PENDING 64

typedef struct mo_encode_packet {
    AVPacket *packet;
    struct mo_encode_packet *next;
} mo_encode_packet;
typedef struct {
    AVCodecContext *codec;
    AVStream *stream;
    AVFrame *frame;
    mo_encode_packet *head, *tail;
    int done, has_dts;
    int64_t last_dts;
} mo_encode_track;
struct mo_encode {
    mo_encode_io io;
    mo_encode_config config;
    mo_encode_limits limits;
    mo_encode_stats stats;
    AVFormatContext *format;
    AVIOContext *avio;
    mo_encode_track video, audio;
    int failed, finished, io_error, audio_fill;
    uint32_t steps, write_calls, pending_packets;
    uint64_t position, pending_bytes;
    int64_t video_end, audio_sent;
    struct { int used; int64_t pts, duration; } video_pending[MO_ENCODE_PENDING];
};
int mo_encode_poll(mo_encode*);
int mo_encode_step(mo_encode*);
int32_t mo_encode_error(mo_encode*, mo_media_error*, int, const char*);
int mo_encode_io_open(mo_encode*);
int mo_encode_queue(mo_encode*, mo_encode_track*, AVPacket*);
int mo_encode_flush_ready(mo_encode*);
void mo_encode_track_close(mo_encode_track*);
#endif
