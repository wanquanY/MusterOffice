#define _POSIX_C_SOURCE 200809L
#include "mo_media.h"
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <libavutil/md5.h>
#include <libavutil/mem.h>

typedef struct { FILE *file; uint64_t size,reads; int cancel,fail,short_reads; } source;
static int32_t read_at(void *opaque,uint64_t offset,uint8_t *out,int32_t length) {
    source *s=opaque;s->reads++;
    if(s->fail || fseeko(s->file,(off_t)offset,SEEK_SET)) return -1;
    if(s->short_reads && length>13) length=13;
    size_t n=fread(out,1,(size_t)length,s->file);return ferror(s->file)?-1:(int32_t)n;
}
static int32_t cancelled(void *opaque) {return ((source*)opaque)->cancel;}
static void require(int condition,const char *message) {if(!condition){fprintf(stderr,"probe failed: %s\n",message);exit(2);}}
static mo_media_limits limits(void) {return (mo_media_limits){1ULL<<30,1ULL<<31,1ULL<<26,32,4096*4096,262144,32,100000};}
static mo_media *open_media(source *s,mo_media_limits l,mo_media_error *e) {
    mo_media_io io={s,s->size,read_at,cancelled};mo_media *m=NULL;
    int result=mo_media_open(&io,&l,&m,e);
    if(result!=MO_MEDIA_OK) {fprintf(stderr,"open: %d %s (%d)\n",result,e->message,e->upstream_code);exit(2);}return m;
}
static uint32_t select_kind(mo_media *m,int kind,mo_media_stream_info *selected) {
    mo_media_error e;
    for(uint32_t i=0;i<mo_media_stream_count(m);i++) {
        mo_media_stream_info stream;require(mo_media_stream(m,i,&stream,&e)==0,"stream metadata");
        if(stream.kind==kind) {require(mo_media_select(m,i,&e)==0,"select stream");*selected=stream;return i;}
    }
    require(0,"requested stream missing");return 0;
}
static uint64_t frames(mo_media *m,const char *phase,int kind,FILE *pcm) {
    uint64_t count=0;mo_media_frame_info f;mo_media_error e;
    for(;;) {
        int result=mo_media_next(m,&f,&e);if(result==MO_MEDIA_EOF)break;
        if(result!=MO_MEDIA_FRAME) {fprintf(stderr,"next: %d %s (%d)\n",result,e.message,e.upstream_code);exit(2);}
        struct AVMD5 *md5=av_md5_alloc();require(md5!=NULL,"hash allocation");av_md5_init(md5);
        require(f.kind==kind && f.has_pts,"explicit valid timestamp");
        if(kind==MO_MEDIA_VIDEO) {
            if(!(f.width==128 && f.height==96 && f.color_primaries==1 && f.color_transfer==1 && f.color_matrix==1 && f.color_range==1))
                fprintf(stderr,"frame metadata: %dx%d primaries=%d transfer=%d matrix=%d range=%d\n",f.width,f.height,f.color_primaries,f.color_transfer,f.color_matrix,f.color_range);
            require(f.width==128 && f.height==96 && f.color_primaries==1 && f.color_transfer==1
                && f.color_matrix==1 && f.color_range==1,"SDR dimensions and color metadata");
        }
        else require(f.sample_rate==48000 && f.channels==1,"audio sample rate and channels");
        uint64_t bytes=0;
        for(int p=0;p<f.planes;p++) {
            mo_media_plane_info plane;require(mo_media_plane(m,(uint32_t)p,&plane,&e)==0,"plane metadata");
            uint64_t stride=plane.row_bytes+7,total=stride*plane.rows;
            uint8_t *data=malloc((size_t)total);require(data!=NULL,"plane allocation");memset(data,0xCD,(size_t)total);
            require(mo_media_copy_plane(m,(uint32_t)p,data,total,stride,&e)==0,"plane copy");
            for(uint64_t y=0;y<plane.rows;y++) {
                av_md5_update(md5,data+y*stride,(size_t)plane.row_bytes);
                if(pcm) require(fwrite(data+y*stride,1,(size_t)plane.row_bytes,pcm)==plane.row_bytes,"PCM output");
                for(uint64_t x=plane.row_bytes;x<stride;x++) require(data[y*stride+x]==0xCD,"copy overwrote destination padding");
            }
            require(mo_media_copy_plane(m,(uint32_t)p,data,plane.row_bytes-1,stride,&e)==MO_MEDIA_INVALID,"short output accepted");
            bytes+=plane.row_bytes*plane.rows;free(data);
        }
        uint8_t digest[16];char hex[33];av_md5_final(md5,digest);av_free(md5);
        for(int i=0;i<16;i++) snprintf(hex+i*2,3,"%02x",digest[i]);
        printf("{\"phase\":\"%s\",\"pts\":\"%"PRId64"\",\"duration\":\"%"PRId64"\",\"timeBase\":[%d,%d],\"generation\":%"PRIu64",\"samples\":%d,\"bytes\":%"PRIu64",\"format\":\"%s\",\"md5\":\"%s\"}\n",phase,f.pts,f.duration,f.time_base_num,f.time_base_den,f.generation,f.samples,bytes,f.format,hex);
        require(++count<10000,"frame count guard");
    }
    require(mo_media_next(m,&f,&e)==MO_MEDIA_EOF,"repeated EOF must remain EOF");
    mo_media_plane_info p;require(mo_media_plane(m,0,&p,&e)==MO_MEDIA_STATE,"stale frame after EOF");
    return count;
}
static void failures(source *s,int kind) {
    mo_media_error e;mo_media *m=NULL;mo_media_io io={s,s->size,read_at,cancelled};mo_media_limits l=limits();
    s->cancel=1;require(mo_media_open(&io,&l,&m,&e)==MO_MEDIA_CANCELLED && !m,"cancelled open");s->cancel=0;
    s->fail=1;require(mo_media_open(&io,&l,&m,&e)==MO_MEDIA_IO && !m,"I/O failure");s->fail=0;
    l.max_input_bytes=1;require(mo_media_open(&io,&l,&m,&e)==MO_MEDIA_LIMIT && !m,"input budget");
    l=limits();l.max_read_bytes=1;require(mo_media_open(&io,&l,&m,&e)==MO_MEDIA_LIMIT && !m,"read budget");
    m=open_media(s,limits(),&e);mo_media_stream_info stream;uint32_t index=select_kind(m,kind,&stream);
    require(mo_media_select(m,index,&e)==MO_MEDIA_STATE,"double selection");
    mo_media_frame_info frame;require(mo_media_next(m,&frame,&e)==MO_MEDIA_FRAME,"first frame");
    s->cancel=1;require(mo_media_next(m,&frame,&e)==MO_MEDIA_CANCELLED,"cancelled decode");
    require(mo_media_next(m,&frame,&e)==MO_MEDIA_STATE,"failed decoder must not resume");s->cancel=0;mo_media_close(m);
    l=limits();l.max_frame_bytes=1;m=open_media(s,l,&e);select_kind(m,kind,&stream);
    require(mo_media_next(m,&frame,&e)==MO_MEDIA_LIMIT,"frame byte budget");mo_media_close(m);
    l=limits();l.max_decode_steps=1;m=open_media(s,l,&e);select_kind(m,kind,&stream);
    require(mo_media_next(m,&frame,&e)==MO_MEDIA_LIMIT,"decode work budget");mo_media_close(m);
    if(kind==MO_MEDIA_VIDEO) {
        l=limits();l.max_pixels=1;m=open_media(s,l,&e);
        require(mo_media_select(m,index,&e)==MO_MEDIA_LIMIT,"pixel admission");mo_media_close(m);
    }
}
int main(int argc,char **argv) {
    require(argc==3 || argc==4,"usage: media-probe input video|audio [mono-pcm-output]");
    int kind=strcmp(argv[2],"video")==0?MO_MEDIA_VIDEO:MO_MEDIA_AUDIO;
    source s={0};s.file=fopen(argv[1],"rb");require(s.file!=NULL,"fixture open");
    require(fseeko(s.file,0,SEEK_END)==0,"fixture size");off_t size=ftello(s.file);require(size>0,"empty fixture");s.size=(uint64_t)size;
    require(mo_media_abi_version()==1,"ABI version");require(strcmp(mo_media_upstream_version(),"9.0.2")==0,"upstream version");
    require(strstr(mo_media_upstream_license(),"LGPL")!=NULL,"component license");
    mo_media_error e;mo_media *m=open_media(&s,limits(),&e);mo_media_stream_info stream;select_kind(m,kind,&stream);
    FILE *pcm=argc==4?fopen(argv[3],"wb"):NULL;require(argc==3 || pcm!=NULL,"PCM destination");
    uint64_t count=frames(m,"linear",kind,pcm);require(count>0,"empty decode");if(pcm)require(fclose(pcm)==0,"PCM close");
    if(kind==MO_MEDIA_VIDEO) {
        int64_t target=stream.time_base_den/stream.time_base_num;
        require(mo_media_seek(m,target,&e)==0,"forward seek");frames(m,"seek-forward",kind,NULL);
        require(mo_media_seek(m,0,&e)==0,"backward seek");require(frames(m,"seek-backward",kind,NULL)==count,"backward seek count");
    }
    mo_media_close(m);failures(&s,kind);
    s.short_reads=1;m=open_media(&s,limits(),&e);select_kind(m,kind,&stream);require(frames(m,"short-reads",kind,NULL)==count,"short-read frame count");mo_media_close(m);
    fclose(s.file);printf("{\"status\":\"passed\",\"readCallbacks\":%"PRIu64"}\n",s.reads);return 0;
}
