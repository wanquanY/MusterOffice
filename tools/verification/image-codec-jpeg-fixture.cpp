// Owned synthetic fixture encoder. Each 8x8 block is constant, so its expected
// decoded gray levels are independently known without using another decoder.
#include <cstdio>
#include <cstdlib>
#include <jpeglib.h>
int main(int argc, char **) {
  jpeg_compress_struct c{};
  jpeg_error_mgr errors;
  c.err = jpeg_std_error(&errors);
  jpeg_create_compress(&c);
  unsigned char *bytes = nullptr;
  unsigned long length = 0;
  jpeg_mem_dest(&c, &bytes, &length);
  c.image_width = 8; c.image_height = 16;
  c.input_components = 1; c.in_color_space = JCS_GRAYSCALE;
  jpeg_set_defaults(&c);
  jpeg_set_quality(&c, 100, TRUE);
  if (argc > 1) jpeg_simple_progression(&c);
  jpeg_start_compress(&c, TRUE);
  unsigned char row[8];
  while (c.next_scanline < 16) {
    for (auto &v : row) v = c.next_scanline < 8 ? 40 : 180;
    JSAMPROW rows[] = {row};
    jpeg_write_scanlines(&c, rows, 1);
  }
  jpeg_finish_compress(&c);
  const bool ok = std::fwrite(bytes, 1, length, stdout) == length;
  std::free(bytes);
  jpeg_destroy_compress(&c);
  return ok ? 0 : 1;
}
