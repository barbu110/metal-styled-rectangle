#include <metal_stdlib>

#include "shader_types.h"

using namespace metal;

struct RectFragmentData {
    float4 position [[position]];
    float2 rect_origin;
    float2 rect_size;
    float4 background_color;
    float4 border_size;
    float4 border_color;

    float corner_radius;
};

float4 to_device_position(float2 pixel_space_pos, float2 viewport_size) {
    float4 ndc_pos = float4(0.0, 0.0, 0.0, 1.0);
    ndc_pos.x = 2.0 * pixel_space_pos.x / viewport_size.x - 1.0;
    ndc_pos.y = 1.0 - 2.0 * pixel_space_pos.y / viewport_size.y;

    return ndc_pos;
}

float rect_sdf(
    float2 absolute_pixel_position,
    float2 origin,
    float2 size,
    float corner_radius
) {
    auto half_size = size / 2.;
    auto rect_center = origin + half_size;

    // Change coordinate space so that the rectangle's center is at the origin,
    // taking advantage of the problem's symmetry.
    float2 pixel_position = abs(absolute_pixel_position - rect_center);

    // Shrink rectangle by the corner radius.
    float2 shrunk_corner_position = half_size - corner_radius;

    // Determine the distance vector from the pixel to the rectangle corner,
    // disallowing negative components to simplify the three cases.
    float2 pixel_to_shrunk_corner = max(float2(0., 0.), pixel_position - shrunk_corner_position);

    float distance_to_shrunk_corner = length(pixel_to_shrunk_corner);

    // Subtract the corner radius from the calculated distance to produce a
    // rectangle having the desired size.
    float distance = distance_to_shrunk_corner - corner_radius;

    return distance;
}

vertex RectFragmentData
rect_vertex_shader(uint vertex_id [[vertex_id]],
                   constant float2 *vertices [[buffer(0)]],
                   constant PerRectUniforms *rect_uniforms [[buffer(1)]],
                   constant Uniforms *uniforms [[buffer(2)]]) {
    constant auto &v = vertices[vertex_id];
    float2 position = v * rect_uniforms->size + rect_uniforms->origin;
    float4 frag_position = to_device_position(position, uniforms->viewport_size);

    return RectFragmentData {
        .position = frag_position,
        .rect_origin = rect_uniforms->origin,
        .rect_size = rect_uniforms->size,
        .border_size = rect_uniforms->border_size,
        .border_color = rect_uniforms->border_color,
        .background_color = rect_uniforms->background_color,
        .corner_radius = rect_uniforms->corner_radius,
    };
}

fragment float4
rect_fragment_shader(RectFragmentData in [[stage_in]]) {
    float2 p = in.position.xy;
    float2 rect_center = in.rect_origin + in.rect_size / 2.;

    float shape_distance = rect_sdf(p, in.rect_origin, in.rect_size, in.corner_radius);
    if (shape_distance > 0.0) {
        return float4(0.0, 0.0, 0.0, 0.0);
    }

    // Subtracting the width of borders (right, bottom) and (top, left)
    float2 background_size = in.rect_size - in.border_size.yz - in.border_size.xw;
    // Moving the origin of the background to the right by sizes of border (top, left)
    float2 background_origin = in.rect_origin + in.border_size.xw;

    float4 border_color = in.border_color;
    if (in.corner_radius > 0) {
        border_color.a *= 1.0 - smoothstep(-.75, -.1, shape_distance);
    }

    float background_distance = rect_sdf(p, background_origin, background_size, in.corner_radius);
    float4 color = mix(in.background_color, border_color, smoothstep(-0.7, 0.5, background_distance));

    return color;
}
