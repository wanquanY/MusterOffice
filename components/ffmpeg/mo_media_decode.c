#include "mo_media_internal.h"

int32_t mo_media_select(mo_media *m,uint32_t index,mo_media_error *error) {
    if (!m || index>=mo_media_stream_count(m)) return mo_media_fail(error,MO_MEDIA_INVALID,0,"invalid stream selection");
    if (m->decoder || m->failed) return mo_media_fail(error,MO_MEDIA_STATE,0,"stream already selected or failed session");
    if (mo_media_cancelled(m)) return mo_media_fail(error,MO_MEDIA_CANCELLED,0,"stream selection cancelled");
    const AVCodecParameters *p=m->format->streams[index]->codecpar;
    if (p->codec_type!=AVMEDIA_TYPE_VIDEO && p->codec_type!=AVMEDIA_TYPE_AUDIO)
        return mo_media_fail(error,MO_MEDIA_UNSUPPORTED,0,"stream is not audio or video");
    if (p->width<0 || p->height<0 || (uint64_t)p->width*(uint64_t)p->height>m->limits.max_pixels
        || p->ch_layout.nb_channels<0 || (uint32_t)p->ch_layout.nb_channels>m->limits.max_channels)
        return mo_media_fail(error,MO_MEDIA_LIMIT,0,"stream dimensions or channels exceed budget");
    const AVCodec *codec=avcodec_find_decoder(p->codec_id);
    if (!codec) return mo_media_fail(error,MO_MEDIA_UNSUPPORTED,0,"codec absent from this component profile");
    AVCodecContext *decoder=avcodec_alloc_context3(codec);
    if (!decoder) return mo_media_fail(error,MO_MEDIA_LIMIT,0,"decoder allocation");
    int result=avcodec_parameters_to_context(decoder,p);
    decoder->pkt_timebase=m->format->streams[index]->time_base;
    decoder->thread_count=1;decoder->max_pixels=m->limits.max_pixels;decoder->max_samples=m->limits.max_audio_samples;
    if (result>=0) result=avcodec_open2(decoder,codec,NULL);
    if (result<0) {avcodec_free_context(&decoder);return mo_media_av_error(m,error,result,"cannot open selected decoder");}
    m->packet=av_packet_alloc();m->frame=av_frame_alloc();
    if (!m->packet || !m->frame) {
        av_packet_free(&m->packet);av_frame_free(&m->frame);avcodec_free_context(&decoder);
        return mo_media_fail(error,MO_MEDIA_LIMIT,0,"decode state allocation");
    }
    m->decoder=decoder;m->stream=(int)index;m->generation=1;return MO_MEDIA_OK;
}

static int32_t frame_plane(const mo_media *m,uint32_t plane,mo_media_plane_info *out,const uint8_t **data,ptrdiff_t *stride,mo_media_error *error) {
    if (!m || !out || !m->frame_valid) return mo_media_fail(error,MO_MEDIA_STATE,0,"no decoded frame");
    const AVFrame *f=m->frame;memset(out,0,sizeof(*out));
    if (m->decoder->codec_type==AVMEDIA_TYPE_AUDIO) {
        int bytes=av_get_bytes_per_sample(f->format),planar=av_sample_fmt_is_planar(f->format);
        if (bytes<=0 || f->ch_layout.nb_channels<=0 || f->nb_samples<=0) return mo_media_fail(error,MO_MEDIA_DECODE,0,"invalid audio frame layout");
        uint32_t planes=planar?(uint32_t)f->ch_layout.nb_channels:1;
        if (plane>=planes) return mo_media_fail(error,MO_MEDIA_INVALID,0,"audio plane index");
        out->row_bytes=(uint64_t)bytes*(uint64_t)f->nb_samples*(planar?1:(uint64_t)f->ch_layout.nb_channels);out->rows=1;
        *data=f->extended_data[plane];*stride=f->linesize[0];
    } else {
        const AVPixFmtDescriptor *desc=av_pix_fmt_desc_get(f->format);
        if (!desc || (desc->flags&(AV_PIX_FMT_FLAG_HWACCEL|AV_PIX_FMT_FLAG_PAL))) return mo_media_fail(error,MO_MEDIA_UNSUPPORTED,0,"pixel layout needs a different frame adapter");
        int count=av_pix_fmt_count_planes(f->format);
        if (count<=0 || count>4 || plane>=(uint32_t)count) return mo_media_fail(error,MO_MEDIA_INVALID,0,"video plane index");
        int lines[4];ptrdiff_t strides[4];size_t sizes[4];
        if (av_image_fill_linesizes(lines,f->format,f->width)<0) return mo_media_fail(error,MO_MEDIA_DECODE,0,"video row size");
        for (int i=0;i<4;i++) strides[i]=lines[i];
        if (av_image_fill_plane_sizes(sizes,f->format,f->height,strides)<0 || lines[plane]<=0)
            return mo_media_fail(error,MO_MEDIA_DECODE,0,"video plane size");
        out->row_bytes=(uint64_t)lines[plane];out->rows=sizes[plane]/out->row_bytes;
        *data=f->data[plane];*stride=f->linesize[plane];
    }
    if (!*data || !out->rows || out->row_bytes>m->limits.max_frame_bytes/out->rows
        || *stride==PTRDIFF_MIN || (uint64_t)(*stride<0?-*stride:*stride)<out->row_bytes)
        return mo_media_fail(error,MO_MEDIA_LIMIT,0,"frame plane exceeds byte budget or stride");
    return MO_MEDIA_OK;
}
int32_t mo_media_plane(const mo_media *m,uint32_t plane,mo_media_plane_info *out,mo_media_error *error) {
    const uint8_t *data;ptrdiff_t stride;return frame_plane(m,plane,out,&data,&stride,error);
}
int32_t mo_media_copy_plane(const mo_media *m,uint32_t plane,uint8_t *out,uint64_t capacity,uint64_t stride,mo_media_error *error) {
    const uint8_t *data;ptrdiff_t source_stride;mo_media_plane_info info;
    int status=frame_plane(m,plane,&info,&data,&source_stride,error);if(status<0) return status;
    if (!out || stride<info.row_bytes || stride>SIZE_MAX || capacity>SIZE_MAX || (info.rows-1)>SIZE_MAX/stride
        || info.row_bytes>capacity || (info.rows-1)*stride>capacity-info.row_bytes)
        return mo_media_fail(error,MO_MEDIA_INVALID,0,"output plane capacity or stride");
    for (uint64_t row=0;row<info.rows;row++) {
        if (mo_media_cancelled(m)) return mo_media_fail(error,MO_MEDIA_CANCELLED,0,"frame copy cancelled; discard partial destination");
        memcpy(out+(size_t)(row*stride),data+(ptrdiff_t)row*source_stride,(size_t)info.row_bytes);
    }
    return MO_MEDIA_OK;
}
int32_t mo_media_frame_metadata(mo_media *m,mo_media_frame_info *out,mo_media_error *error) {
    AVFrame *f=m->frame;memset(out,0,sizeof(*out));out->generation=m->generation;
    out->kind=m->decoder->codec_type==AVMEDIA_TYPE_VIDEO?MO_MEDIA_VIDEO:MO_MEDIA_AUDIO;
    AVRational tb=f->time_base;
    if (tb.num<=0 || tb.den<=0) tb=m->decoder->pkt_timebase;
    out->time_base_num=tb.num;out->time_base_den=tb.den;
    out->pts=f->pts;out->has_pts=f->pts!=AV_NOPTS_VALUE;
    out->duration=f->duration;out->has_duration=f->duration>0;
    out->width=f->width;out->height=f->height;out->sample_rate=f->sample_rate;
    out->channels=f->ch_layout.nb_channels;out->samples=f->nb_samples;
    out->color_primaries=f->color_primaries;out->color_transfer=f->color_trc;out->color_matrix=f->colorspace;
    out->color_range=f->color_range;out->chroma_location=f->chroma_location;
    out->sar_num=f->sample_aspect_ratio.num;out->sar_den=f->sample_aspect_ratio.den;
    out->alpha_mode=f->alpha_mode;out->key_frame=!!(f->flags&AV_FRAME_FLAG_KEY);
    out->interlaced=!!(f->flags&AV_FRAME_FLAG_INTERLACED);out->top_field_first=!!(f->flags&AV_FRAME_FLAG_TOP_FIELD_FIRST);
    const char *format=NULL;
    if (out->kind==MO_MEDIA_VIDEO) {
        if(f->width<=0 || f->height<=0 || (uint64_t)f->width*(uint64_t)f->height>m->limits.max_pixels)
            return mo_media_fail(error,MO_MEDIA_LIMIT,0,"decoded dimensions exceed budget");
        format=av_get_pix_fmt_name(f->format);out->planes=av_pix_fmt_count_planes(f->format);
    } else {
        if(f->ch_layout.nb_channels<=0 || (uint32_t)f->ch_layout.nb_channels>m->limits.max_channels
            || f->nb_samples<=0 || (uint32_t)f->nb_samples>m->limits.max_audio_samples)
            return mo_media_fail(error,MO_MEDIA_LIMIT,0,"decoded audio exceeds budget");
        format=av_get_sample_fmt_name(f->format);out->planes=av_sample_fmt_is_planar(f->format)?f->ch_layout.nb_channels:1;
        av_channel_layout_describe(&f->ch_layout,out->channel_layout,sizeof(out->channel_layout));
    }
    if (!format || out->planes<=0) return mo_media_fail(error,MO_MEDIA_UNSUPPORTED,0,"unknown decoded layout");
    snprintf(out->format,sizeof(out->format),"%s",format);
    uint64_t total=0;
    for (int i=0;i<out->planes;i++) {
        mo_media_plane_info plane;const uint8_t *data;ptrdiff_t stride;
        int result=frame_plane(m,(uint32_t)i,&plane,&data,&stride,error);if(result<0)return result;
        uint64_t bytes=plane.rows*plane.row_bytes;
        if(bytes>m->limits.max_frame_bytes-total) return mo_media_fail(error,MO_MEDIA_LIMIT,0,"decoded frame byte budget");
        total+=bytes;
    }
    return MO_MEDIA_OK;
}

static int32_t decode_failure(mo_media *m,mo_media_error *e,int result,const char *message) {
    m->failed=1;m->frame_valid=0;av_frame_unref(m->frame);return mo_media_av_error(m,e,result,message);
}
int32_t mo_media_next(mo_media *m,mo_media_frame_info *out,mo_media_error *error) {
    if(!m || !out || !m->decoder || m->failed) return mo_media_fail(error,MO_MEDIA_STATE,0,"decoder not ready");
    m->frame_valid=0;av_frame_unref(m->frame);memset(out,0,sizeof(*out));
    if(mo_media_cancelled(m)) return decode_failure(m,error,AVERROR_EXIT,"decode cancelled");
    if(m->decoder_eof) return MO_MEDIA_EOF;
    for(uint32_t step=0;step<m->limits.max_decode_steps;step++) {
        if(mo_media_cancelled(m)) return decode_failure(m,error,AVERROR_EXIT,"decode cancelled");
        int result=avcodec_receive_frame(m->decoder,m->frame);
        if(result==0) {
            m->frame_valid=1;result=mo_media_frame_metadata(m,out,error);
            if(result<0) {m->failed=1;m->frame_valid=0;av_frame_unref(m->frame);return result;}
            return MO_MEDIA_FRAME;
        }
        if(result==AVERROR_EOF) {m->decoder_eof=1;return MO_MEDIA_EOF;}
        if(result!=AVERROR(EAGAIN)) return decode_failure(m,error,result,"decoder receive failed");
        if(m->packet_pending) {
            result=avcodec_send_packet(m->decoder,m->packet);
            if(result<0) return decode_failure(m,error,result,"decoder rejected pending packet after receive EAGAIN");
            av_packet_unref(m->packet);m->packet_pending=0;continue;
        }
        if(m->input_eof) {
            if(m->drain_sent) return decode_failure(m,error,AVERROR_BUG,"decoder requested input after drain");
            result=avcodec_send_packet(m->decoder,NULL);
            if(result<0) return decode_failure(m,error,result,"decoder drain failed");
            m->drain_sent=1;continue;
        }
        result=av_read_frame(m->format,m->packet);
        if(m->io_error) return decode_failure(m,error,result,"media read failed");
        if(result==AVERROR_EOF) {m->input_eof=1;continue;}
        if(result<0) return decode_failure(m,error,result,"demux failed");
        if(m->packet->stream_index!=m->stream) {av_packet_unref(m->packet);continue;}
        m->packet_pending=1;
    }
    m->failed=1;return mo_media_fail(error,MO_MEDIA_LIMIT,0,"decode work budget");
}
int32_t mo_media_seek(mo_media *m,int64_t timestamp,mo_media_error *error) {
    if(!m || !m->decoder || m->failed) return mo_media_fail(error,MO_MEDIA_STATE,0,"decoder not ready for seek");
    if(mo_media_cancelled(m)) return decode_failure(m,error,AVERROR_EXIT,"seek cancelled");
    if(m->generation==UINT64_MAX) return mo_media_fail(error,MO_MEDIA_LIMIT,0,"seek generation exhausted");
    m->frame_valid=0;av_frame_unref(m->frame);av_packet_unref(m->packet);
    int result=avformat_seek_file(m->format,m->stream,INT64_MIN,timestamp,timestamp,0);
    if(result<0 || m->io_error) return decode_failure(m,error,result,"media seek failed");
    avcodec_flush_buffers(m->decoder);m->packet_pending=0;m->input_eof=0;m->drain_sent=0;m->decoder_eof=0;m->generation++;
    return MO_MEDIA_OK;
}
