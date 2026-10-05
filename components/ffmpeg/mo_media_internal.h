#ifndef MO_MEDIA_INTERNAL_H
#define MO_MEDIA_INTERNAL_H
#include "mo_media.h"
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <libavformat/avformat.h>
#include <libavcodec/avcodec.h>
#include <libavutil/imgutils.h>
#include <libavutil/pixdesc.h>
#include <libavutil/samplefmt.h>

struct mo_media {
    mo_media_io io;
    mo_media_limits limits;
    uint64_t position, read_bytes, generation;
    AVFormatContext *format;
    AVIOContext *avio;
    AVCodecContext *decoder;
    AVPacket *packet;
    AVFrame *frame;
    int stream, packet_pending, input_eof, drain_sent, decoder_eof, frame_valid, failed;
    int io_error;
};
int32_t mo_media_fail(mo_media_error*, int32_t, int, const char*);
int mo_media_cancelled(const mo_media*);
int32_t mo_media_av_error(mo_media*, mo_media_error*, int, const char*);
int32_t mo_media_frame_metadata(mo_media*, mo_media_frame_info*, mo_media_error*);
#endif
