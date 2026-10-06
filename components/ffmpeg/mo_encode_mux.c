#include "mo_encode_internal.h"

int mo_encode_poll(mo_encode *s) {
    if (s->io_error) return s->io_error;
    if (s->io.cancelled(s->io.opaque)) return s->io_error=MO_MEDIA_CANCELLED;
    return 0;
}
int mo_encode_step(mo_encode *s) {
    if (mo_encode_poll(s)) return AVERROR_EXIT;
    if (++s->steps > s->limits.max_steps) { s->io_error=MO_MEDIA_LIMIT; return AVERROR(ENOSPC); }
    return 0;
}
int32_t mo_encode_error(mo_encode *s, mo_media_error *e, int code, const char *where) {
    int status=s->io_error ? s->io_error : MO_MEDIA_ENCODE;
    s->failed=1;
    return mo_media_fail(e,status,code,where);
}
static int write_packet(void *opaque,const uint8_t *data,int size) {
    mo_encode *s=opaque;
    if (size<=0 || mo_encode_poll(s)) return AVERROR_EXIT;
    uint64_t n=(uint64_t)size;
    if (n>s->limits.max_file_bytes-s->position || n>s->limits.max_write_bytes-s->stats.write_bytes) {
        s->io_error=MO_MEDIA_LIMIT; return AVERROR(ENOSPC);
    }
    int written=0;
    while (written<size) {
        if (mo_encode_poll(s)) return AVERROR_EXIT;
        if (++s->write_calls>s->limits.max_write_calls) { s->io_error=MO_MEDIA_LIMIT; return AVERROR(ENOSPC); }
        int count=s->io.write_at(s->io.opaque,s->position,data+written,size-written);
        if (count<=0 || count>size-written) { s->io_error=MO_MEDIA_IO; return AVERROR(EIO); }
        s->position+=(uint64_t)count;s->stats.write_bytes+=(uint64_t)count;written+=count;
        if (s->position>s->stats.byte_length) s->stats.byte_length=s->position;
    }
    return written;
}
static int64_t seek_output(void *opaque,int64_t offset,int whence) {
    mo_encode *s=opaque;
    if (mo_encode_poll(s)) return AVERROR_EXIT;
    if (whence==AVSEEK_SIZE) return (int64_t)s->stats.byte_length;
    whence &= ~AVSEEK_FORCE;
    int64_t base=whence==SEEK_SET?0:whence==SEEK_CUR?(int64_t)s->position:whence==SEEK_END?(int64_t)s->stats.byte_length:-1;
    if (base<0 || (offset>0 && base>INT64_MAX-offset) || (offset<0 && offset < -base)) return AVERROR(EINVAL);
    int64_t position=base+offset;
    if ((uint64_t)position>s->limits.max_file_bytes) {s->io_error=MO_MEDIA_LIMIT;return AVERROR(ENOSPC);}
    s->position=(uint64_t)position;return position;
}
static int reject_open(AVFormatContext *context,AVIOContext **io,const char *url,int flags,AVDictionary **options) {
    (void)context;(void)io;(void)url;(void)flags;(void)options;return AVERROR(EPERM);
}
static int interrupted(void *opaque) { return mo_encode_poll(opaque)!=0; }
int mo_encode_io_open(mo_encode *s) {
    unsigned char *buffer=av_malloc(32768);
    if (!buffer) return AVERROR(ENOMEM);
    s->avio=avio_alloc_context(buffer,32768,1,s,NULL,write_packet,seek_output);
    if (!s->avio) {av_free(buffer);return AVERROR(ENOMEM);}
    s->avio->seekable=AVIO_SEEKABLE_NORMAL;
    s->format->pb=s->avio;s->format->flags|=AVFMT_FLAG_CUSTOM_IO;
    s->format->io_open=reject_open;
    s->format->interrupt_callback=(AVIOInterruptCB){interrupted,s};
    return 0;
}
static int write_one(mo_encode *s,mo_encode_track *t) {
    int ret=mo_encode_step(s);if (ret<0) return ret;
    mo_encode_packet *node=t->head;AVPacket *p=node->packet;
    uint64_t bytes=(uint64_t)p->size;
    t->head=node->next;if (!t->head) t->tail=NULL;
    /* Already rescaled to the post-header stream timebase. No opaque library
       interleaver can accumulate packets outside our queue admission. */
    ret=av_write_frame(s->format,p);
    av_packet_free(&node->packet);av_free(node);
    s->pending_bytes-=bytes;s->pending_packets--;
    if (ret>=0 && s->avio->error<0) ret=s->avio->error;
    return ret;
}
int mo_encode_flush_ready(mo_encode *s) {
    for (;;) {
        mo_encode_track *v=&s->video,*a=&s->audio,*selected=NULL;
        if (v->head && a->head) {
            selected=av_compare_ts(v->head->packet->dts,v->stream->time_base,a->head->packet->dts,a->stream->time_base)<=0?v:a;
        } else if (v->head && a->done) selected=v;
        else if (a->head && v->done) selected=a;
        else return 0;
        int ret=write_one(s,selected);if (ret<0) return ret;
    }
}
int mo_encode_queue(mo_encode *s,mo_encode_track *t,AVPacket *p) {
    if (p->size<0 || p->pts==AV_NOPTS_VALUE || p->dts==AV_NOPTS_VALUE || p->duration<0) return AVERROR_INVALIDDATA;
    uint64_t n=(uint64_t)p->size;
    if (n>s->limits.max_packet_bytes || n>s->limits.max_pending_bytes-s->pending_bytes ||
        s->pending_packets>=s->limits.max_pending_packets ||
        s->stats.video_packets+s->stats.audio_packets>=s->limits.max_packets) {
        s->io_error=MO_MEDIA_LIMIT;return AVERROR(ENOSPC);
    }
    if (t==&s->video) {
        int found=0;
        for (int i=0;i<MO_ENCODE_PENDING;i++) if (s->video_pending[i].used && s->video_pending[i].pts==p->pts) {
            p->duration=s->video_pending[i].duration;s->video_pending[i].used=0;found=1;break;
        }
        if (!found) return AVERROR_INVALIDDATA;
    }
    av_packet_rescale_ts(p,t->codec->time_base,t->stream->time_base);
    if (p->duration<=0 || (t->has_dts && p->dts<=t->last_dts)) return AVERROR_INVALIDDATA;
    t->has_dts=1;t->last_dts=p->dts;p->stream_index=t->stream->index;
    mo_encode_packet *node=av_mallocz(sizeof(*node));if (!node) return AVERROR(ENOMEM);
    node->packet=av_packet_alloc();if (!node->packet) {av_free(node);return AVERROR(ENOMEM);}
    av_packet_move_ref(node->packet,p);
    if (t->tail) t->tail->next=node;else t->head=node;t->tail=node;
    s->pending_bytes+=n;s->pending_packets++;
    if (s->pending_bytes>s->stats.peak_pending_bytes) s->stats.peak_pending_bytes=s->pending_bytes;
    if (t==&s->video) s->stats.video_packets++;else s->stats.audio_packets++;
    return mo_encode_flush_ready(s);
}
void mo_encode_track_close(mo_encode_track *t) {
    avcodec_free_context(&t->codec);av_frame_free(&t->frame);
    while (t->head) {mo_encode_packet *n=t->head;t->head=n->next;av_packet_free(&n->packet);av_free(n);}
}
