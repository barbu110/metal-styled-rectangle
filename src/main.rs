#![feature(portable_simd)]

embed_plist::embed_info_plist!("info.plist");

mod bindings;

use std::{
    mem, println,
    simd::{f32x2, f32x4, Simd},
};

use bindings::{PerRectUniforms, Uniforms};
use cocoa::{appkit::NSView, base::id as cocoa_id};
use core_graphics_types::geometry::CGSize;
use metal::{
    Device, MTLResourceOptions, MTLStoreAction, MetalLayer, RenderPassDescriptor,
    RenderPipelineDescriptor,
};
use objc::runtime::YES;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::macos::WindowExtMacOS,
    window::WindowBuilder,
};

const LIBRARY_DATA: &[u8] = include_bytes!("shaders.metallib");
#[rustfmt::skip]
const VERTEX_ATTRIBS: &[f32] = &[
    0.0, 0.0,
    1.0, 0.0,
    1.0, 1.0,
    1.0, 1.0,
    0.0, 1.0,
    0.0, 0.0,
];

fn main() {
    let event_loop = EventLoop::new();

    let size = LogicalSize::new(600, 400);
    let window = WindowBuilder::new()
        .with_inner_size(size)
        .with_title("Window with Metal Primitive")
        .build(&event_loop)
        .unwrap();

    let device = Device::system_default().expect("no device found");

    let layer = MetalLayer::new();
    layer.set_device(&device);
    layer.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
    layer.set_presents_with_transaction(false);

    unsafe {
        let view = window.ns_view() as cocoa_id;
        view.setWantsLayer(YES);
        view.setLayer(mem::transmute(layer.as_ref()));
    }

    let draw_size = window.inner_size();
    layer.set_drawable_size(CGSize::new(draw_size.width as f64, draw_size.height as f64));

    let library = device.new_library_with_data(LIBRARY_DATA).unwrap();
    let vertex_fn = library.get_function("rect_vertex_shader", None).unwrap();
    let fragment_fn = library.get_function("rect_fragment_shader", None).unwrap();

    let pipeline_state_descriptor = RenderPipelineDescriptor::new();
    pipeline_state_descriptor.set_vertex_function(Some(&vertex_fn));
    pipeline_state_descriptor.set_fragment_function(Some(&fragment_fn));

    let attachment = pipeline_state_descriptor
        .color_attachments()
        .object_at(0)
        .unwrap();

    attachment.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
    attachment.set_blending_enabled(true);
    attachment.set_rgb_blend_operation(metal::MTLBlendOperation::Add);
    attachment.set_alpha_blend_operation(metal::MTLBlendOperation::Add);
    attachment.set_source_rgb_blend_factor(metal::MTLBlendFactor::SourceAlpha);
    attachment.set_source_alpha_blend_factor(metal::MTLBlendFactor::SourceAlpha);
    attachment.set_destination_rgb_blend_factor(metal::MTLBlendFactor::OneMinusSourceAlpha);
    attachment.set_destination_alpha_blend_factor(metal::MTLBlendFactor::OneMinusSourceAlpha);

    let pipeline_state = device
        .new_render_pipeline_state(&pipeline_state_descriptor)
        .unwrap();

    let command_queue = device.new_command_queue();

    let rect_uniforms = PerRectUniforms {
        background_color: unsafe { mem::transmute(f32x4::from_array([0.1, 0.1, 0.1, 1.0])) },
        border_color: unsafe { mem::transmute(f32x4::from_array([1.0, 1.0, 1.0, 0.1])) },
        border_size: unsafe { mem::transmute(f32x4::from_array([1.0, 1.0, 1.0, 1.0])) },
        corner_radius: 12.0,
        origin: unsafe { mem::transmute(f32x2::from_array([20.0, 20.0])) },
        size: unsafe { mem::transmute(f32x2::from_array([200.0, 100.0])) },
    };
    let rect_uniforms_buffer = device.new_buffer_with_data(
        &rect_uniforms as *const PerRectUniforms as *const _,
        mem::size_of::<PerRectUniforms>() as u64,
        MTLResourceOptions::CPUCacheModeDefaultCache | MTLResourceOptions::StorageModeManaged,
    );

    let mut uniforms = Uniforms {
        viewport_size: unsafe {
            mem::transmute(f32x2::from_array([
                draw_size.width as _,
                draw_size.height as _,
            ]))
        },
    };
    let uniforms_buffer = device.new_buffer_with_data(
        &uniforms as *const Uniforms as *const _,
        mem::size_of::<Uniforms>() as u64,
        MTLResourceOptions::CPUCacheModeDefaultCache | MTLResourceOptions::StorageModeManaged,
    );

    let vertex_buffer = device.new_buffer_with_data(
        VERTEX_ATTRIBS.as_ptr() as *const _,
        (VERTEX_ATTRIBS.len() * mem::size_of::<f32>()) as u64,
        MTLResourceOptions::CPUCacheModeDefaultCache | MTLResourceOptions::StorageModeManaged,
    );

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(draw_size) => {
                    layer.set_drawable_size(CGSize::new(
                        draw_size.width as f64,
                        draw_size.height as f64,
                    ));
                }
                _ => (),
            },
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let drawable = match layer.next_drawable() {
                    Some(drawable) => drawable,
                    None => return,
                };

                let render_pass_descriptor = RenderPassDescriptor::new();

                let color_attachment = render_pass_descriptor
                    .color_attachments()
                    .object_at(0)
                    .unwrap();
                color_attachment.set_texture(Some(drawable.texture()));
                color_attachment.set_load_action(metal::MTLLoadAction::Clear);
                color_attachment.set_clear_color(metal::MTLClearColor {
                    red: 0.2,
                    green: 0.2,
                    blue: 0.2,
                    alpha: 1.0,
                });
                color_attachment.set_store_action(MTLStoreAction::Store);

                let command_buffer = command_queue.new_command_buffer();
                let rc_encoder = command_buffer.new_render_command_encoder(&render_pass_descriptor);
                let physical_size = window.inner_size();
                uniforms.viewport_size = unsafe {
                    mem::transmute(f32x2::from_array([
                        draw_size.width as _,
                        draw_size.height as _,
                    ]))
                };
                uniforms_buffer.did_modify_range(metal::NSRange {
                    location: 0,
                    length: mem::size_of::<Uniforms>() as u64,
                });
                rc_encoder.set_scissor_rect(metal::MTLScissorRect {
                    x: 0,
                    y: 0,
                    width: physical_size.width as _,
                    height: physical_size.height as _,
                });
                rc_encoder.set_render_pipeline_state(&pipeline_state);
                rc_encoder.set_vertex_buffer(0, Some(&vertex_buffer), 0);
                rc_encoder.set_vertex_buffer(1, Some(&rect_uniforms_buffer), 0);
                rc_encoder.set_vertex_buffer(2, Some(&uniforms_buffer), 0);
                rc_encoder.draw_primitives(metal::MTLPrimitiveType::Triangle, 0, 6);
                rc_encoder.end_encoding();

                command_buffer.present_drawable(&drawable);
                command_buffer.commit();
            }
            _ => (),
        }
    });
}
