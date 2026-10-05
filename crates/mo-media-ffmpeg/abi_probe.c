#include "mo_media.h"
#include <stdio.h>
#define TYPE(T, R) printf("const _: () = { assert!(size_of::<" #R ">() == %zu); assert!(align_of::<" #R ">() == %zu); };\n", sizeof(T), _Alignof(T))
#define FIELD(T, R, F) printf("const _: () = assert!(offset_of!(" #R ", " #F ") == %zu);\n", offsetof(T, F))
#define ERROR(F) FIELD(mo_media_error, RawError, F)
#define IO(F) FIELD(mo_media_io, Io, F)
#define LIMIT(F) FIELD(mo_media_limits, Limits, F)
#define STREAM(F) FIELD(mo_media_stream_info, Stream, F)
#define FRAME(F) FIELD(mo_media_frame_info, Frame, F)
int main(void) {
    TYPE(mo_media_error, RawError); ERROR(status); ERROR(upstream_code); ERROR(message);
    TYPE(mo_media_io, Io); IO(opaque); IO(byte_length); IO(read_at); IO(cancelled);
    TYPE(mo_media_limits, Limits);
    LIMIT(max_input_bytes); LIMIT(max_read_bytes); LIMIT(max_frame_bytes); LIMIT(max_streams);
    LIMIT(max_pixels); LIMIT(max_audio_samples); LIMIT(max_channels); LIMIT(max_decode_steps);
    TYPE(mo_media_stream_info, Stream);
    STREAM(kind); STREAM(index); STREAM(time_base_num); STREAM(time_base_den); STREAM(start); STREAM(duration);
    STREAM(has_start); STREAM(has_duration); STREAM(width); STREAM(height); STREAM(sample_rate); STREAM(channels);
    STREAM(frame_rate_num); STREAM(frame_rate_den); STREAM(sar_num); STREAM(sar_den);
    STREAM(color_primaries); STREAM(color_transfer); STREAM(color_matrix); STREAM(color_range); STREAM(chroma_location);
    STREAM(has_display_matrix); STREAM(display_matrix); STREAM(codec); STREAM(channel_layout);
    TYPE(mo_media_frame_info, Frame);
    FRAME(generation); FRAME(kind); FRAME(time_base_num); FRAME(time_base_den); FRAME(has_pts); FRAME(has_duration);
    FRAME(pts); FRAME(duration); FRAME(width); FRAME(height); FRAME(sample_rate); FRAME(channels); FRAME(samples); FRAME(planes);
    FRAME(color_primaries); FRAME(color_transfer); FRAME(color_matrix); FRAME(color_range); FRAME(chroma_location);
    FRAME(sar_num); FRAME(sar_den); FRAME(alpha_mode); FRAME(key_frame); FRAME(interlaced); FRAME(top_field_first);
    FRAME(format); FRAME(channel_layout);
    TYPE(mo_media_plane_info, Plane); FIELD(mo_media_plane_info, Plane, row_bytes); FIELD(mo_media_plane_info, Plane, rows);
    return 0;
}
