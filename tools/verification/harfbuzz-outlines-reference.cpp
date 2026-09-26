// Independent upstream API entry. No MusterOffice component or allocator linked.
#include "hb.h"
#include "hb-ot.h"
#include <iostream>
#include <vector>
#include <array>
#include <cmath>
using Path=std::vector<std::array<int64_t,7>>;
static void add(void *p,int op,std::initializer_list<float> values) {std::array<int64_t,7> r={op,0,0,0,0,0,0};int i=1;for(float v:values)r[i++]=std::llround(v);static_cast<Path *>(p)->push_back(r);}
static void move(hb_draw_funcs_t *,void *p,hb_draw_state_t *,float x,float y,void *){add(p,1,{x,y});}
static void line(hb_draw_funcs_t *,void *p,hb_draw_state_t *,float x,float y,void *){add(p,2,{x,y});}
static void quad(hb_draw_funcs_t *,void *p,hb_draw_state_t *,float a,float b,float x,float y,void *){add(p,3,{a,b,x,y});}
static void cubic(hb_draw_funcs_t *,void *p,hb_draw_state_t *,float a,float b,float c,float d,float x,float y,void *){add(p,4,{a,b,c,d,x,y});}
static void close(hb_draw_funcs_t *,void *p,hb_draw_state_t *,void *){add(p,5,{});}
int main(int argc,char **argv){
 if(argc<4)return 1;auto *blob=hb_blob_create_from_file_or_fail(argv[1]);if(!blob)return 2;
 auto *face=hb_face_create(blob,std::stoul(argv[2]));auto *font=hb_font_create(face);hb_ot_font_set_funcs(font);const int upem=hb_face_get_upem(face);hb_font_set_scale(font,upem*64,upem*64);
 std::vector<hb_variation_t> axes;for(int i=4;i<argc;++i){hb_variation_t v;if(!hb_variation_from_string(argv[i],-1,&v))return 3;axes.push_back(v);}hb_font_set_variations(font,axes.data(),axes.size());
 auto *draw=hb_draw_funcs_create();hb_draw_funcs_set_move_to_func(draw,move,nullptr,nullptr);hb_draw_funcs_set_line_to_func(draw,line,nullptr,nullptr);hb_draw_funcs_set_quadratic_to_func(draw,quad,nullptr,nullptr);hb_draw_funcs_set_cubic_to_func(draw,cubic,nullptr,nullptr);hb_draw_funcs_set_close_path_func(draw,close,nullptr,nullptr);
 std::string ids=argv[3];std::cout<<"[";size_t start=0;bool first=true;
 while(start<ids.size()){auto end=ids.find(',',start);auto id=std::stoul(ids.substr(start,end-start));Path path;bool ok=hb_font_draw_glyph_or_fail(font,id,draw,&path);if(!first)std::cout<<",";first=false;
 if(!ok)std::cout<<"null";else {std::cout<<"[";bool sep=false;for(auto &command:path){if(sep)std::cout<<",";sep=true;std::cout<<"[";for(int i=0;i<7;++i){if(i)std::cout<<",";std::cout<<command[i];}std::cout<<"]";}std::cout<<"]";}
 if(end==std::string::npos)break;start=end+1;}
 std::cout<<"]\n";hb_draw_funcs_destroy(draw);hb_font_destroy(font);hb_face_destroy(face);hb_blob_destroy(blob);
}
