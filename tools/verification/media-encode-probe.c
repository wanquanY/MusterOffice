/* Native lifetime/bounds probe. The production ABI owns no file paths. */
#include "mo_encode.h"
#include <inttypes.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    unsigned char *bytes;
    uint64_t size, capacity, writes, polls, cancel_at;
    int failure;
} sink;
static void require(int value,const char *message) {
    if (!value) { fprintf(stderr,"encode probe failed: %s\n",message);exit(2); }
}
static int32_t write_at(void *opaque,uint64_t offset,const uint8_t *bytes,int32_t count) {
    sink *s=opaque;s->writes++;
    if (s->failure==1) return 0;
    if (s->failure==2) return count+1;
    if (s->failure==3) return -1;
    if (count>17) count=17;
    if (offset+(uint64_t)count>s->capacity) return -1;
    memcpy(s->bytes+offset,bytes,(size_t)count);
    if (offset+(uint64_t)count>s->size) s->size=offset+(uint64_t)count;
    return count;
}
static int32_t cancelled(void *opaque) {
    sink *s=opaque;s->polls++;
    return s->cancel_at && s->polls>=s->cancel_at;
}
static mo_encode_limits limits(void) {
    return (mo_encode_limits){8<<20,16<<20,1<<20,1<<20,1024*1024,10000,1<<20,128,16384,100000};
}
static mo_encode_config config(int profile) {
    /* Partial last frame and audio whose sample clock differs from video. */
    return (mo_encode_config){profile,128,96,30000,30000,1001,7351,8,2000000,44100,2,128000,25,10806};
}
static int exercise(sink *s,int profile,int mode,mo_encode_stats *stats) {
    mo_encode_io io={s,write_at,cancelled};mo_encode_config c=config(profile);
    mo_encode_limits l=limits();mo_media_error e={0};mo_encode *encoder=NULL;
    if (mode==1) l.max_file_bytes=100;
    if (mode==2) l.max_steps=1;
    if (mode==3) l.max_pending_packets=1;
    if (mode==4) l.max_write_calls=1;
    if (mode==5) l.max_packet_bytes=1;
    int status=mo_encode_open(&io,&c,&l,&encoder,&e);
    uint8_t y[128*96],u[64*48],v[64*48];
    memset(y,128,sizeof(y));memset(u,80,sizeof(u));memset(v,180,sizeof(v));
    int vi=0;int64_t ai=0;
    while (!status && (vi<c.video_frames || ai<c.audio_samples)) {
        if (vi<c.video_frames && (ai>=c.audio_samples || (int64_t)vi*1001*44100<=ai*30000)) {
            mo_encode_video_frame f={(int64_t)vi*1001,vi==7?344:1001,{y,u,v},{sizeof(y),sizeof(u),sizeof(v)},{128,64,64}};
            if (mode==6) f.plane_bytes[0]=1;
            if (mode==7) f.strides[1]=UINT64_MAX;
            if (mode==8) f.pts=INT64_MAX;
            status=mo_encode_video(encoder,&f,&e);vi++;
        } else {
            float samples[997*2];uint32_t n=(uint32_t)(c.audio_samples-ai);
            if (n>997) n=997;
            for (uint32_t i=0;i<n*2;i++) samples[i]=(float)(.25*sin((double)(ai+i/2)*.057));
            if (mode==9) samples[0]=NAN;
            status=mo_encode_audio(encoder,ai,samples,n,&e);ai+=n;
        }
    }
    if (!status) status=mo_encode_finish(encoder,stats,&e);
    if (status && encoder) require(mo_encode_finish(encoder,stats,&e)==MO_MEDIA_STATE,"failed session resumed");
    if (!status) require(mo_encode_finish(encoder,stats,&e)==MO_MEDIA_STATE,"finished session resumed");
    mo_encode_close(encoder);return status;
}
int main(int argc,char **argv) {
    require(argc==2 || argc==3,"usage: encode-probe output.mp4 [hardware]");
    sink s={0};s.capacity=8<<20;s.bytes=malloc((size_t)s.capacity);require(s.bytes!=NULL,"sink allocation");
    int profile=argc==3?MO_ENCODE_H264_VT_AAC_MP4:MO_ENCODE_FFV1_PCM_MP4;
    mo_encode_stats stats;
    require(exercise(&s,profile,0,&stats)==0,"actual finite export");
    require(stats.video_frames==8 && stats.video_packets==8 && stats.audio_samples==10806 && stats.byte_length==s.size,"exact output counts");
    FILE *file=fopen(argv[1],"wb");require(file!=NULL,"candidate file");
    require(fwrite(s.bytes,1,(size_t)s.size,file)==s.size && fclose(file)==0,"candidate write");
    uint64_t polls=s.polls;
    for (int mode=1;mode<=9;mode++) {
        memset(s.bytes,0,(size_t)s.capacity);s.size=s.writes=s.polls=0;
        int status=exercise(&s,profile,mode,&stats);
        require(status==(mode<=5?MO_MEDIA_LIMIT:MO_MEDIA_INVALID),"budget/invalid input refusal");
    }
    for (int failure=1;failure<=3;failure++) {
        s.failure=failure;s.size=s.writes=s.polls=0;
        require(exercise(&s,profile,0,&stats)==MO_MEDIA_IO,"sink error refusal");
    }
    s.failure=0;
    /* Exercise C-internal cancellation during copies, codec receive and AVIO.
       A fixed sample of the observed path avoids treating Rust entry checks as
       sufficient coverage; final cancellation is separately included. */
    for (uint64_t i=1;i<=polls;i+=(polls/31?polls/31:1)) {
        s.size=s.writes=s.polls=0;s.cancel_at=i;
        require(exercise(&s,profile,0,&stats)==MO_MEDIA_CANCELLED,"internal cancellation");
    }
    s.size=s.writes=s.polls=0;s.cancel_at=polls;
    require(exercise(&s,profile,0,&stats)==MO_MEDIA_CANCELLED,"final cancellation");
    free(s.bytes);
    printf("{\"status\":\"passed\",\"polls\":%"PRIu64",\"profile\":%d}\n",polls,profile);
    return 0;
}
