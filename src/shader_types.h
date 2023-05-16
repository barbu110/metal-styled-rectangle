#include <simd/simd.h>

#ifndef shader_types_h
#define shader_types_h

struct PerRectUniforms {
  vector_float2 size;
  vector_float2 origin;
  simd_float4 background_color;

  /* Borders */

  vector_float4 border_size; // (top, right, bottom, left)
  vector_float4 border_color;

  float corner_radius;
};

struct Uniforms {
  vector_float2 viewport_size;
};

#endif /* shader_types_h */
