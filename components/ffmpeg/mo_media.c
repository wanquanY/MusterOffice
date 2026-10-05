#include "mo_media_internal.h"

uint32_t mo_media_abi_version(void) { return 1; }
const char *mo_media_upstream_version(void) { return av_version_info(); }
const char *mo_media_upstream_license(void) { return avcodec_license(); }
int32_t mo_media_fail(mo_media_error *error, int32_t status, int upstream, const char *message) {
    if (error) {
        memset(error,0,sizeof(*error)); error->status=status; error->upstream_code=upstream;
        snprintf(error->message,sizeof(error->message),"%s",message);
    }
    return status;
}
int mo_media_cancelled(const mo_media *m) { return m->io.cancelled && m->io.cancelled(m->io.opaque); }
int32_t mo_media_av_error(mo_media *m, mo_media_error *e, int code, const char *message) {
    int status=MO_MEDIA_DECODE;
    if (mo_media_cancelled(m) || m->io_error==MO_MEDIA_CANCELLED) status=MO_MEDIA_CANCELLED;
    else if (m->io_error) status=m->io_error;
    else if (code==AVERROR(ENOMEM)) status=MO_MEDIA_LIMIT;
    return mo_media_fail(e,status,code,message);
}
static int interrupted(void *opaque) { return mo_media_cancelled(opaque); }
static int deny_open(AVFormatContext *s, AVIOContext **p, const char *url, int flags, AVDictionary **options) {
    (void)s;(void)p;(void)url;(void)flags;(void)options;return AVERROR(EPERM);
}
static int read_bytes(void *opaque, uint8_t *buffer, int capacity) {
    mo_media *m=opaque;
    if (mo_media_cancelled(m)) {m->io_error=MO_MEDIA_CANCELLED;return AVERROR_EXIT;}
    if (capacity<=0) return AVERROR(EINVAL);
    if (m->position>=m->io.byte_length) return AVERROR_EOF;
    uint64_t requested=(uint64_t)capacity;
    if (requested>m->io.byte_length-m->position) requested=m->io.byte_length-m->position;
    if (requested>m->limits.max_read_bytes-m->read_bytes) {m->io_error=MO_MEDIA_LIMIT;return AVERROR(ENOMEM);}
    int result=m->io.read_at(m->io.opaque,m->position,buffer,(int)requested);
    if (result<=0 || (uint64_t)result>requested) {m->io_error=MO_MEDIA_IO;return AVERROR(EIO);}
    m->position+=(uint64_t)result;m->read_bytes+=(uint64_t)result;
    return result;
}
static int64_t seek_bytes(void *opaque, int64_t offset, int whence) {
    mo_media *m=opaque;
    if (mo_media_cancelled(m)) {m->io_error=MO_MEDIA_CANCELLED;return AVERROR_EXIT;}
    if (whence==AVSEEK_SIZE) return (int64_t)m->io.byte_length;
    whence &= ~AVSEEK_FORCE;
    int64_t base;
    switch(whence) {case SEEK_SET:base=0;break;case SEEK_CUR:base=(int64_t)m->position;break;
        case SEEK_END:base=(int64_t)m->io.byte_length;break;default:return AVERROR(EINVAL);}
    if (offset< -base || offset>INT64_MAX-base) return AVERROR(EINVAL);
    int64_t position=base+offset;
    if ((uint64_t)position>m->io.byte_length) return AVERROR(EINVAL);
    m->position=(uint64_t)position;return position;
}
int32_t mo_media_open(const mo_media_io *io,const mo_media_limits *limits,mo_media **out,mo_media_error *error) {
    if (out) *out=NULL;
    if (!io || !limits || !out || !io->read_at || io->byte_length==0 || io->byte_length>INT64_MAX
        || !limits->max_read_bytes || !limits->max_frame_bytes || !limits->max_streams || limits->max_streams>1024
        || !limits->max_pixels || !limits->max_audio_samples || !limits->max_channels || limits->max_channels>64
        || !limits->max_decode_steps) return mo_media_fail(error,MO_MEDIA_INVALID,0,"invalid media input or limits");
    if (io->byte_length>limits->max_input_bytes) return mo_media_fail(error,MO_MEDIA_LIMIT,0,"input byte budget");
    if (io->cancelled && io->cancelled(io->opaque)) return mo_media_fail(error,MO_MEDIA_CANCELLED,0,"media open cancelled");
    mo_media *m=calloc(1,sizeof(*m));
    if (!m) return mo_media_fail(error,MO_MEDIA_LIMIT,0,"media context allocation");
    m->io=*io;m->limits=*limits;m->stream=-1;
    uint8_t *buffer=av_malloc(32768);
    if (!buffer) {mo_media_close(m);return mo_media_fail(error,MO_MEDIA_LIMIT,0,"media I/O allocation");}
    m->avio=avio_alloc_context(buffer,32768,0,m,read_bytes,NULL,seek_bytes);
    if (!m->avio) {av_free(buffer);mo_media_close(m);return mo_media_fail(error,MO_MEDIA_LIMIT,0,"media I/O context");}
    m->format=avformat_alloc_context();
    if (!m->format) {mo_media_close(m);return mo_media_fail(error,MO_MEDIA_LIMIT,0,"demux context");}
    m->format->pb=m->avio;m->format->flags|=AVFMT_FLAG_CUSTOM_IO;
    m->format->interrupt_callback=(AVIOInterruptCB){interrupted,m};m->format->io_open=deny_open;
    m->format->max_streams=(int)limits->max_streams;
    m->format->probesize=1048576;m->format->max_analyze_duration=5*AV_TIME_BASE;
    int result=avformat_open_input(&m->format,NULL,NULL,NULL);
    if (result<0) {int status=mo_media_av_error(m,error,result,"cannot open media container");mo_media_close(m);return status;}
    /* Probe container parameters only. Stream analysis may open arbitrary decoders before our
       per-decoder limits are installed. Decode fills unknown metadata after explicit select. */
    if (m->format->nb_streams>limits->max_streams) {mo_media_close(m);return mo_media_fail(error,MO_MEDIA_LIMIT,0,"stream count budget");}
    if (mo_media_cancelled(m) || m->io_error) {int status=mo_media_av_error(m,error,AVERROR_EXIT,"media open interrupted");mo_media_close(m);return status;}
    *out=m;return MO_MEDIA_OK;
}
void mo_media_close(mo_media *m) {
    if (!m) return;
    av_frame_free(&m->frame);av_packet_free(&m->packet);avcodec_free_context(&m->decoder);
    avformat_close_input(&m->format);
    if (m->avio) {av_freep(&m->avio->buffer);avio_context_free(&m->avio);}
    free(m);
}
uint32_t mo_media_stream_count(const mo_media *m) {return m && m->format ? m->format->nb_streams:0;}
int32_t mo_media_stream(const mo_media *m,uint32_t index,mo_media_stream_info *out,mo_media_error *error) {
    if (!m || !out || index>=mo_media_stream_count(m)) return mo_media_fail(error,MO_MEDIA_INVALID,0,"invalid stream index");
    const AVStream *s=m->format->streams[index];const AVCodecParameters *p=s->codecpar;
    memset(out,0,sizeof(*out));out->index=(int32_t)index;
    out->kind=p->codec_type==AVMEDIA_TYPE_VIDEO?MO_MEDIA_VIDEO:p->codec_type==AVMEDIA_TYPE_AUDIO?MO_MEDIA_AUDIO:MO_MEDIA_OTHER;
    out->time_base_num=s->time_base.num;out->time_base_den=s->time_base.den;
    out->has_start=s->start_time!=AV_NOPTS_VALUE;out->start=s->start_time;
    out->has_duration=s->duration!=AV_NOPTS_VALUE;out->duration=s->duration;
    out->width=p->width;out->height=p->height;out->sample_rate=p->sample_rate;out->channels=p->ch_layout.nb_channels;
    out->frame_rate_num=s->avg_frame_rate.num;out->frame_rate_den=s->avg_frame_rate.den;
    out->sar_num=p->sample_aspect_ratio.num;out->sar_den=p->sample_aspect_ratio.den;
    out->color_primaries=p->color_primaries;out->color_transfer=p->color_trc;out->color_matrix=p->color_space;
    out->color_range=p->color_range;out->chroma_location=p->chroma_location;
    snprintf(out->codec,sizeof(out->codec),"%s",avcodec_get_name(p->codec_id));
    av_channel_layout_describe(&p->ch_layout,out->channel_layout,sizeof(out->channel_layout));
    const AVPacketSideData *matrix=av_packet_side_data_get(p->coded_side_data,p->nb_coded_side_data,AV_PKT_DATA_DISPLAYMATRIX);
    if (matrix && matrix->size==sizeof(out->display_matrix)) {out->has_display_matrix=1;memcpy(out->display_matrix,matrix->data,matrix->size);}
    return MO_MEDIA_OK;
}
