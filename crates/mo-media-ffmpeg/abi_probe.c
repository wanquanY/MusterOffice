#include "mo_media.h"
#include "mo_encode.h"
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
    STREAM(has_container_track_id); STREAM(container_track_id);
    TYPE(mo_media_frame_info, Frame);
    FRAME(generation); FRAME(kind); FRAME(time_base_num); FRAME(time_base_den); FRAME(has_pts); FRAME(has_duration);
    FRAME(pts); FRAME(duration); FRAME(width); FRAME(height); FRAME(sample_rate); FRAME(channels); FRAME(samples); FRAME(planes);
    FRAME(color_primaries); FRAME(color_transfer); FRAME(color_matrix); FRAME(color_range); FRAME(chroma_location);
    FRAME(sar_num); FRAME(sar_den); FRAME(alpha_mode); FRAME(key_frame); FRAME(interlaced); FRAME(top_field_first);
    FRAME(format); FRAME(channel_layout);
    TYPE(mo_media_plane_info, Plane); FIELD(mo_media_plane_info, Plane, row_bytes); FIELD(mo_media_plane_info, Plane, rows);
    TYPE(mo_encode_io, EncodeIo);
    FIELD(mo_encode_io, EncodeIo, opaque);
    FIELD(mo_encode_io, EncodeIo, write_at);
    FIELD(mo_encode_io, EncodeIo, cancelled);
    TYPE(mo_encode_config, EncoderConfig);
    FIELD(mo_encode_config, EncoderConfig, profile);
    FIELD(mo_encode_config, EncoderConfig, width);
    FIELD(mo_encode_config, EncoderConfig, height);
    FIELD(mo_encode_config, EncoderConfig, video_time_scale);
    FIELD(mo_encode_config, EncoderConfig, frame_rate_num);
    FIELD(mo_encode_config, EncoderConfig, frame_rate_den);
    FIELD(mo_encode_config, EncoderConfig, video_duration);
    FIELD(mo_encode_config, EncoderConfig, video_frames);
    FIELD(mo_encode_config, EncoderConfig, video_bit_rate);
    FIELD(mo_encode_config, EncoderConfig, audio_sample_rate);
    FIELD(mo_encode_config, EncoderConfig, audio_channels);
    FIELD(mo_encode_config, EncoderConfig, audio_bit_rate);
    FIELD(mo_encode_config, EncoderConfig, gop_frames);
    FIELD(mo_encode_config, EncoderConfig, audio_samples);
    TYPE(mo_encode_limits, EncoderLimits);
    FIELD(mo_encode_limits, EncoderLimits, max_file_bytes);
    FIELD(mo_encode_limits, EncoderLimits, max_write_bytes);
    FIELD(mo_encode_limits, EncoderLimits, max_pending_bytes);
    FIELD(mo_encode_limits, EncoderLimits, max_frame_bytes);
    FIELD(mo_encode_limits, EncoderLimits, max_pixels);
    FIELD(mo_encode_limits, EncoderLimits, max_packets);
    FIELD(mo_encode_limits, EncoderLimits, max_packet_bytes);
    FIELD(mo_encode_limits, EncoderLimits, max_pending_packets);
    FIELD(mo_encode_limits, EncoderLimits, max_steps);
    FIELD(mo_encode_limits, EncoderLimits, max_write_calls);
    TYPE(mo_encode_video_frame, EncoderVideo);
    FIELD(mo_encode_video_frame, EncoderVideo, pts);
    FIELD(mo_encode_video_frame, EncoderVideo, duration);
    FIELD(mo_encode_video_frame, EncoderVideo, planes);
    FIELD(mo_encode_video_frame, EncoderVideo, plane_bytes);
    FIELD(mo_encode_video_frame, EncoderVideo, strides);
    TYPE(mo_encode_stats, EncoderStats);
    FIELD(mo_encode_stats, EncoderStats, byte_length);
    FIELD(mo_encode_stats, EncoderStats, write_bytes);
    FIELD(mo_encode_stats, EncoderStats, video_frames);
    FIELD(mo_encode_stats, EncoderStats, audio_samples);
    FIELD(mo_encode_stats, EncoderStats, video_packets);
    FIELD(mo_encode_stats, EncoderStats, audio_packets);
    FIELD(mo_encode_stats, EncoderStats, peak_pending_bytes);
    return 0;
}
