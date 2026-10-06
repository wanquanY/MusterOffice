#include "mo_encode_internal.h"

uint32_t mo_encode_abi_version(void) {return 1;}
uint32_t mo_encode_profiles(void) {
    uint32_t result=0;
    if (avcodec_find_encoder_by_name("h264_videotoolbox") && avcodec_find_encoder_by_name("aac") && av_guess_format("mp4",NULL,NULL)) result|=1;
    if (avcodec_find_encoder_by_name("ffv1") && avcodec_find_encoder_by_name("pcm_f32le") && av_guess_format("mp4",NULL,NULL)) result|=2;
    return result;
}
void mo_encode_close(mo_encode *s) {
    if (!s) return;
    mo_encode_track_close(&s->video);mo_encode_track_close(&s->audio);
    if (s->format) s->format->pb=NULL;
    avformat_free_context(s->format);
    if (s->avio) {av_freep(&s->avio->buffer);avio_context_free(&s->avio);}
    av_free(s);
}
/* MP4 edit-list durations use the movie clock, independently of each track.
   Both clocks must fit exactly; never hide sample-tail rounding in the muxer. */
static int64_t movie_scale(const mo_encode_config *c) {
    if (!c->audio_samples) return c->video_time_scale;
    return c->video_time_scale / av_gcd(c->video_time_scale,c->audio_sample_rate) * (int64_t)c->audio_sample_rate;
}
static int validate(const mo_encode_io *io,const mo_encode_config *c,const mo_encode_limits *l) {
    if (!io || !c || !l || !io->write_at || !io->cancelled ||
        (c->profile!=MO_ENCODE_H264_VT_AAC_MP4 && c->profile!=MO_ENCODE_FFV1_PCM_MP4) ||
        c->width<2 || c->height<2 || c->width>8192 || c->height>8192 || (c->width%2) || (c->height%2) ||
        (uint64_t)c->width*c->height>l->max_pixels || (uint64_t)c->width*c->height*2>l->max_frame_bytes ||
        c->video_time_scale<1 || c->video_time_scale>1000000000 || c->video_frames<1 || c->video_frames>l->max_packets ||
        c->video_duration<1 || c->frame_rate_num<1 || c->frame_rate_num>1000000000 ||
        c->frame_rate_den<1 || c->frame_rate_den>1000000000 ||
        (int64_t)c->frame_rate_num>240LL*c->frame_rate_den || c->frame_rate_num<c->frame_rate_den ||
        c->video_bit_rate<10000 || c->video_bit_rate>1000000000 || c->gop_frames<1 || c->gop_frames>10000 ||
        c->audio_samples<0 || (c->audio_samples && (c->audio_channels<1 || c->audio_channels>2 ||
        c->audio_sample_rate<8000 || c->audio_sample_rate>192000 || c->audio_bit_rate<8000 || c->audio_bit_rate>512000)) ||
        !l->max_file_bytes || l->max_file_bytes>INT64_MAX || l->max_write_bytes<l->max_file_bytes ||
        !l->max_pending_bytes || l->max_pending_bytes>(1ULL<<30) || !l->max_packet_bytes ||
        l->max_packet_bytes>l->max_pending_bytes || !l->max_pending_packets || l->max_pending_packets>4096 ||
        !l->max_steps || l->max_steps>1000000 || !l->max_write_calls || l->max_write_calls>1000000) return 0;
    int64_t scale=movie_scale(c);
    if (scale>INT32_MAX || c->video_duration>INT64_MAX/(scale/c->video_time_scale) ||
        (c->audio_samples && c->audio_samples>INT64_MAX/(scale/c->audio_sample_rate))) return 0;
    int64_t period=(int64_t)c->video_time_scale*c->frame_rate_den;
    if (period%c->frame_rate_num) return 0;
    period/=c->frame_rate_num;
    if (c->video_duration<=(c->video_frames-1)*period || c->video_duration>c->video_frames*period) return 0;
    return 1;
}
static int open_track(mo_encode *s,mo_encode_track *t,const char *name,int audio) {
    const AVCodec *codec=avcodec_find_encoder_by_name(name);
    if (!codec) return AVERROR_ENCODER_NOT_FOUND;
    t->codec=avcodec_alloc_context3(codec);if (!t->codec) return AVERROR(ENOMEM);
    AVCodecContext *c=t->codec;
    c->thread_count=1;c->flags|=AV_CODEC_FLAG_GLOBAL_HEADER|AV_CODEC_FLAG_FRAME_DURATION;
    AVDictionary *options=NULL;
    int ret;
    if (audio) {
        c->sample_rate=s->config.audio_sample_rate;c->time_base=(AVRational){1,c->sample_rate};
        av_channel_layout_default(&c->ch_layout,s->config.audio_channels);
        c->sample_fmt=s->config.profile==MO_ENCODE_H264_VT_AAC_MP4?AV_SAMPLE_FMT_FLTP:AV_SAMPLE_FMT_FLT;
        c->bit_rate=s->config.audio_bit_rate;
    } else {
        c->width=s->config.width;c->height=s->config.height;c->pix_fmt=AV_PIX_FMT_YUV420P;
        c->time_base=(AVRational){1,s->config.video_time_scale};
        c->framerate=(AVRational){s->config.frame_rate_num,s->config.frame_rate_den};
        c->sample_aspect_ratio=(AVRational){1,1};c->bit_rate=s->config.video_bit_rate;
        c->gop_size=s->config.gop_frames;c->max_b_frames=s->config.profile==MO_ENCODE_H264_VT_AAC_MP4?2:0;
        c->color_primaries=AVCOL_PRI_BT709;c->color_trc=AVCOL_TRC_BT709;c->colorspace=AVCOL_SPC_BT709;
        c->color_range=AVCOL_RANGE_MPEG;c->chroma_sample_location=AVCHROMA_LOC_LEFT;
        if (s->config.profile==MO_ENCODE_H264_VT_AAC_MP4) {
            av_dict_set(&options,"allow_sw","0",0);av_dict_set(&options,"profile","high",0);
        } else { c->level=3;av_dict_set(&options,"slicecrc","1",0); }
    }
    ret=avcodec_open2(c,codec,&options);
    if (ret>=0 && av_dict_count(options)) ret=AVERROR_OPTION_NOT_FOUND;
    av_dict_free(&options);if (ret<0) return ret;
    t->stream=avformat_new_stream(s->format,NULL);if (!t->stream) return AVERROR(ENOMEM);
    t->stream->time_base=c->time_base;
    t->stream->avg_frame_rate=audio?(AVRational){0,1}:c->framerate;
    t->stream->sample_aspect_ratio=c->sample_aspect_ratio;
    ret=avcodec_parameters_from_context(t->stream->codecpar,c);if (ret<0) return ret;
    t->frame=av_frame_alloc();if (!t->frame) return AVERROR(ENOMEM);
    AVFrame *f=t->frame;
    if (audio) {
        f->format=c->sample_fmt;f->sample_rate=c->sample_rate;
        ret=av_channel_layout_copy(&f->ch_layout,&c->ch_layout);if (ret<0) return ret;
        f->nb_samples=c->frame_size?c->frame_size:1024;
        if (f->nb_samples>65536 || (uint64_t)f->nb_samples*c->ch_layout.nb_channels*4>s->limits.max_frame_bytes) return AVERROR(ENOSPC);
    } else {
        f->format=c->pix_fmt;f->width=c->width;f->height=c->height;f->sample_aspect_ratio=c->sample_aspect_ratio;
        f->color_primaries=c->color_primaries;f->color_trc=c->color_trc;f->colorspace=c->colorspace;
        f->color_range=c->color_range;f->chroma_location=c->chroma_sample_location;
    }
    return av_frame_get_buffer(f,32);
}
int32_t mo_encode_open(const mo_encode_io *io,const mo_encode_config *config,const mo_encode_limits *limits,mo_encode **out,mo_media_error *e) {
    if (out) *out=NULL;
    if (!out || !validate(io,config,limits)) return mo_media_fail(e,MO_MEDIA_INVALID,0,"invalid encode configuration");
    if (!(mo_encode_profiles() & (uint32_t)config->profile)) return mo_media_fail(e,MO_MEDIA_UNSUPPORTED,0,"encoding profile unavailable");
    mo_encode *s=av_mallocz(sizeof(*s));if (!s) return mo_media_fail(e,MO_MEDIA_LIMIT,AVERROR(ENOMEM),"encoder allocation");
    s->io=*io;s->config=*config;s->limits=*limits;
    const int h264=config->profile==MO_ENCODE_H264_VT_AAC_MP4;
    int ret=avformat_alloc_output_context2(&s->format,NULL,"mp4",NULL);
    if (ret>=0) ret=mo_encode_io_open(s);
    if (ret>=0 && mo_encode_poll(s)) ret=AVERROR_EXIT;
    if (ret>=0) ret=open_track(s,&s->video,h264?"h264_videotoolbox":"ffv1",0);
    if (ret>=0 && config->audio_samples) ret=open_track(s,&s->audio,h264?"aac":"pcm_f32le",1);
    if (!config->audio_samples) s->audio.done=1;
    AVDictionary *options=NULL;
    {av_dict_set_int(&options,"video_track_timescale",config->video_time_scale,0);av_dict_set_int(&options,"movie_timescale",movie_scale(config),0);}
    if (ret>=0) ret=avformat_write_header(s->format,&options);
    if (ret>=0 && av_dict_count(options)) ret=AVERROR_OPTION_NOT_FOUND;
    av_dict_free(&options);
    if (ret>=0 && (av_cmp_q(s->video.stream->time_base,s->video.codec->time_base) ||
        (s->audio.codec && av_cmp_q(s->audio.stream->time_base,s->audio.codec->time_base)))) ret=AVERROR_INVALIDDATA;
    if (ret>=0 && s->avio->error<0) ret=s->avio->error;
    if (ret>=0 && mo_encode_poll(s)) ret=AVERROR_EXIT;
    if (ret<0) {int status=mo_encode_error(s,e,ret,"open encoding profile");mo_encode_close(s);return status;}
    *out=s;return MO_MEDIA_OK;
}
static int receive(mo_encode *s,mo_encode_track *t,int draining) {
    AVPacket *packet=av_packet_alloc();if (!packet) return AVERROR(ENOMEM);
    int ret;
    for (;;) {
        ret=mo_encode_step(s);if (ret<0) break;
        ret=avcodec_receive_packet(t->codec,packet);
        if (ret==AVERROR(EAGAIN)) {ret=draining?AVERROR_BUG:0;break;}
        if (ret==AVERROR_EOF) {t->done=1;ret=mo_encode_flush_ready(s);break;}
        if (ret<0) break;
        ret=mo_encode_queue(s,t,packet);av_packet_unref(packet);if (ret<0) break;
    }
    av_packet_free(&packet);return ret;
}
static int send_frame(mo_encode *s,mo_encode_track *t,AVFrame *frame) {
    int ret=mo_encode_step(s);if (ret<0) return ret;
    /* Every previous send was completely received. A simultaneous EAGAIN is
       invalid, not a sleep/retry policy or permission to duplicate input. */
    ret=avcodec_send_frame(t->codec,frame);if (ret<0) return ret;
    return receive(s,t,frame==NULL);
}
static int start_call(mo_encode *s,mo_media_error *e) {
    if (!s || s->failed || s->finished) return mo_media_fail(e,MO_MEDIA_STATE,0,"closed or failed encoder");
    s->steps=0;s->write_calls=0;
    if (mo_encode_poll(s)) return mo_encode_error(s,e,AVERROR_EXIT,"encoding cancelled");
    return 0;
}
static int invalid(mo_encode *s,mo_media_error *e,const char *message) {s->io_error=MO_MEDIA_INVALID;return mo_encode_error(s,e,AVERROR(EINVAL),message);}
int32_t mo_encode_video(mo_encode *s,const mo_encode_video_frame *v,mo_media_error *e) {
    int ret=start_call(s,e);if (ret<0) return ret;
    if (!v || v->pts!=s->video_end || v->duration<=0 || v->duration>s->config.video_duration-v->pts ||
        s->stats.video_frames>=(uint64_t)s->config.video_frames) return invalid(s,e,"non-contiguous video frame");
    int64_t expected=(int64_t)s->config.video_time_scale*s->config.frame_rate_den/s->config.frame_rate_num;
    if (expected>s->config.video_duration-v->pts) expected=s->config.video_duration-v->pts;
    if (v->duration!=expected) return invalid(s,e,"video frame clock mismatch");
    for (int p=0;p<3;p++) {
        uint64_t w=(uint64_t)s->config.width>>(p!=0),h=(uint64_t)s->config.height>>(p!=0);
        if (!v->planes[p] || v->strides[p]<w || v->strides[p]>s->limits.max_frame_bytes ||
            v->strides[p]>UINT64_MAX/h || v->plane_bytes[p]<(h-1)*v->strides[p]+w) return invalid(s,e,"video plane layout");
    }
    int slot=-1;for (int i=0;i<MO_ENCODE_PENDING;i++) if (!s->video_pending[i].used) {slot=i;break;}
    if (slot<0) {s->io_error=MO_MEDIA_LIMIT;return mo_encode_error(s,e,AVERROR(ENOSPC),"video encoder delay budget");}
    AVFrame *f=s->video.frame;
    ret=av_frame_make_writable(f);if (ret<0) return mo_encode_error(s,e,ret,"video frame allocation");
    for (int p=0;p<3;p++) {
        int w=s->config.width>>(p!=0),h=s->config.height>>(p!=0);
        for (int y=0;y<h;y++) {
            if (mo_encode_poll(s)) return mo_encode_error(s,e,AVERROR_EXIT,"video copy cancelled");
            memcpy(f->data[p]+(size_t)y*f->linesize[p],v->planes[p]+(uint64_t)y*v->strides[p],(size_t)w);
        }
    }
    f->pts=v->pts;f->duration=v->duration;
    s->video_pending[slot].used=1;s->video_pending[slot].pts=v->pts;s->video_pending[slot].duration=v->duration;
    ret=send_frame(s,&s->video,f);if (ret<0) return mo_encode_error(s,e,ret,"encode video frame");
    s->stats.video_frames++;s->video_end+=v->duration;return MO_MEDIA_OK;
}
static int send_audio(mo_encode *s) {
    AVFrame *f=s->audio.frame;
    f->nb_samples=s->audio_fill;f->pts=s->audio_sent;f->duration=s->audio_fill;
    int ret=send_frame(s,&s->audio,f);if (ret<0) return ret;
    s->audio_sent+=s->audio_fill;s->audio_fill=0;
    f->nb_samples=s->audio.codec->frame_size?s->audio.codec->frame_size:1024;
    return av_frame_make_writable(f);
}
int32_t mo_encode_audio(mo_encode *s,int64_t first,const float *input,uint32_t frames,mo_media_error *e) {
    int ret=start_call(s,e);if (ret<0) return ret;
    if (!s->audio.codec || !input || !frames || frames>65536 || first<0 || (uint64_t)first!=s->stats.audio_samples ||
        frames>(uint64_t)s->config.audio_samples-s->stats.audio_samples) return invalid(s,e,"non-contiguous audio frame");
    int channels=s->config.audio_channels;
    for (uint64_t i=0;i<(uint64_t)frames*channels;i++) if (!isfinite(input[i])) return invalid(s,e,"non-finite audio input");
    uint32_t copied=0;
    while (copied<frames) {
        if (mo_encode_step(s)<0) return mo_encode_error(s,e,AVERROR_EXIT,"audio work budget");
        AVFrame *f=s->audio.frame;int capacity=s->audio.codec->frame_size?s->audio.codec->frame_size:1024;
        int count=FFMIN((int)(frames-copied),capacity-s->audio_fill);
        for (int i=0;i<count;i++) for (int ch=0;ch<channels;ch++) {
            if (f->format==AV_SAMPLE_FMT_FLTP) ((float*)f->data[ch])[s->audio_fill+i]=input[((uint64_t)copied+i)*channels+ch];
            else ((float*)f->data[0])[(s->audio_fill+i)*channels+ch]=input[((uint64_t)copied+i)*channels+ch];
        }
        s->audio_fill+=count;copied+=(uint32_t)count;s->stats.audio_samples+=(uint64_t)count;
        if (s->audio_fill==capacity || s->stats.audio_samples==(uint64_t)s->config.audio_samples) {
            ret=send_audio(s);if (ret<0) return mo_encode_error(s,e,ret,"encode audio frame");
        }
    }
    return MO_MEDIA_OK;
}
int32_t mo_encode_finish(mo_encode *s,mo_encode_stats *stats,mo_media_error *e) {
    int ret=start_call(s,e);if (ret<0) return ret;
    if (!stats || s->stats.video_frames!=(uint64_t)s->config.video_frames || s->video_end!=s->config.video_duration ||
        s->stats.audio_samples!=(uint64_t)s->config.audio_samples || s->audio_fill) return invalid(s,e,"incomplete export input");
    ret=send_frame(s,&s->video,NULL);
    if (ret>=0 && s->audio.codec) ret=send_frame(s,&s->audio,NULL);
    if (ret>=0) ret=mo_encode_flush_ready(s);
    if (ret>=0 && (s->pending_packets || s->stats.video_packets!=s->stats.video_frames)) ret=AVERROR_INVALIDDATA;
    if (ret>=0) ret=av_write_trailer(s->format);
    if (ret>=0) {avio_flush(s->avio);if (s->avio->error<0) ret=s->avio->error;}
    if (ret>=0 && mo_encode_poll(s)) ret=AVERROR_EXIT;
    if (ret<0) return mo_encode_error(s,e,ret,"finalize encoded candidate");
    s->finished=1;*stats=s->stats;return MO_MEDIA_OK;
}
