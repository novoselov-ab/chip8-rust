mod imgui_wgpu;

use futures::executor::block_on;
use imgui::*;
use imgui_wgpu::Renderer;
use imgui_winit_support;
use std::time::Instant;
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

pub struct AppDesc {
    pub screen_width: u32,
    pub screen_height: u32
}

pub fn run(desc: &AppDesc, mut draw_imgui: impl FnMut(&imgui::Ui,  &mut Vec<u8>) + 'static) {
    // Set up window and GPU
    let event_loop = EventLoop::new();
    let mut hidpi_factor = 1.0;
    let (window, mut size, surface) = {
        let window = Window::new(&event_loop).unwrap();
        window.set_inner_size(LogicalSize {
            width: 1280.0,
            height: 720.0,
        });
        window.set_title("chip8");
        let size = window.inner_size();

        let surface = wgpu::Surface::create(&window);

        (window, size, surface)
    };

    let adapter = block_on(wgpu::Adapter::request(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
        },
        wgpu::BackendBit::PRIMARY,
    ))
    .unwrap();

    let (mut device, mut queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
            anisotropic_filtering: false,
        },
        limits: wgpu::Limits::default(),
    }));

    // Set up swap chain
    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width: size.width as u32,
        height: size.height as u32,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);

    // Set up dear imgui
    let mut imgui = imgui::Context::create();
    let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    platform.attach_window(
        imgui.io_mut(),
        &window,
        imgui_winit_support::HiDpiMode::Default,
    );
    imgui.set_ini_filename(None);

    let font_size = (13.0 * hidpi_factor) as f32;
    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

    imgui.fonts().add_font(&[FontSource::DefaultFontData {
        config: Some(imgui::FontConfig {
            oversample_h: 1,
            pixel_snap_h: true,
            size_pixels: font_size,
            ..Default::default()
        }),
    }]);

    //
    // Set up dear imgui wgpu renderer
    //
    let clear_color = wgpu::Color {
        r: 0.1,
        g: 0.2,
        b: 0.3,
        a: 1.0,
    };
    let mut renderer = Renderer::new(
        &mut imgui,
        &device,
        &mut queue,
        sc_desc.format,
        Some(clear_color),
    );

    let mut last_frame = Instant::now();

    let screen_w = desc.screen_width;
    let screen_h = desc.screen_height;
    let mut screen_raw_data: Vec<u8> = vec![0; (screen_w as usize) * (screen_h as usize) * 4];
    let screen_texture_id =
        renderer.create_texture(&device, screen_w, screen_h);
    let mut screen_scale = 1.0_f32;


    let mut last_cursor = None;

    // Event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = if cfg!(feature = "metal-auto-capture") {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        };
        match event {
            Event::WindowEvent {
                event: WindowEvent::ScaleFactorChanged { scale_factor, .. },
                ..
            } => {
                hidpi_factor = scale_factor;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                size = window.inner_size();

                sc_desc = wgpu::SwapChainDescriptor {
                    usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    width: size.width as u32,
                    height: size.height as u32,
                    present_mode: wgpu::PresentMode::Mailbox,
                };

                swap_chain = device.create_swap_chain(&surface, &sc_desc);
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    },
                ..
            }
            | Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawEventsCleared => {
                last_frame = imgui.io_mut().update_delta_time(last_frame);

                let frame = match swap_chain.get_next_texture() {
                    Ok(frame) => frame,
                    Err(e) => {
                        eprintln!("dropped frame: {:?}", e);
                        return;
                    }
                };
                platform
                    .prepare_frame(imgui.io_mut(), &window)
                    .expect("Failed to prepare frame");
                let ui = imgui.frame();

                // Callback to the user
                draw_imgui(&ui, &mut screen_raw_data);

                // Uploaded update screen
                renderer.update_texture(screen_texture_id, &device, &mut queue, &screen_raw_data, screen_w, screen_h);

                {
                    let window = imgui::Window::new(im_str!("Screen"));
                    window
                        .size([400.0, 600.0], Condition::FirstUseEver)
                        .build(&ui, || {
                            ui.drag_float(im_str!("Scale"), &mut screen_scale).build();
                            let size = [(screen_w as f32) * screen_scale, (screen_h as f32) * screen_scale];
                            Image::new(screen_texture_id, size).build(&ui);
                        });
                }

                let mut encoder: wgpu::CommandEncoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                if last_cursor != Some(ui.mouse_cursor()) {
                    last_cursor = Some(ui.mouse_cursor());
                    platform.prepare_render(&ui, &window);
                }
                renderer
                    .render(ui.render(), &mut device, &mut encoder, &frame.view)
                    .expect("Rendering failed");

                queue.submit(&[encoder.finish()]);
            }
            _ => (),
        }

        platform.handle_event(imgui.io_mut(), &window, &event);
    });
}
