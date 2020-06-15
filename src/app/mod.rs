mod imgui_wgpu;

use futures::executor::block_on;
use glob::glob;
use imgui::*;
use imgui_wgpu::Renderer;
use imgui_winit_support;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

mod chip8;

fn find_roms() -> glob::Paths {
    let exe_path = std::env::current_exe();
    let rom_path = exe_path.unwrap().parent().unwrap().join("../../roms");

    glob(rom_path.join("**/*.ch8").to_str().unwrap()).unwrap()
}

pub struct Chip8App {
    _roms: Vec<PathBuf>,
    _emulator: chip8::Emulator,
}

impl Chip8App {
    pub fn new() -> Self {
        let roms = find_roms().map(|res| res.unwrap()).collect();

        Chip8App {
            _roms: roms,
            _emulator: chip8::Emulator::new(),
        }
    }

    fn draw_ui(&mut self, ui: &imgui::Ui) {
        let window = imgui::Window::new(im_str!("ROMs"));
        window
            .size([400.0, 600.0], Condition::FirstUseEver)
            .build(&ui, || {
                for rom_file in &self._roms {
                    if ui.button(
                        &ImString::new(rom_file.file_name().unwrap().to_str().unwrap()),
                        [0 as f32, 0 as f32],
                    ) {
                        self._emulator.load_rom(rom_file);
                    }
                }
            });

        let window = imgui::Window::new(im_str!("CPU"));
        window
            .size([400.0, 600.0], Condition::FirstUseEver)
            .build(&ui, || {
                ui.text(format!("PC: {:#X}", self._emulator.pc));
                ui.text(format!("I: {:#X}", self._emulator.ri));
                for i in 0..self._emulator.rs.len() {
                    ui.text(format!("V{:X}: {:#X} ", i, self._emulator.rs[i]));
                    if (i + 1) % 4 != 0 {
                        ui.same_line(0.0);
                    }
                }
            });

        let window = imgui::Window::new(im_str!("Code"));
        window
            .size([400.0, 600.0], Condition::FirstUseEver)
            .build(&ui, || {
                let code = self._emulator.get_code();
                for i in 0..code.len() {
                    ui.text(format!("{}: {:#X}", i, code[i]));
                }
            });

        self._emulator.update(ui.io().delta_time);
    }

    fn set_key_state(&mut self, code: VirtualKeyCode, state: bool) {
        self._emulator.keypad.set(
            match code {
                VirtualKeyCode::Key1 => 0,
                VirtualKeyCode::Key2 => 1,
                VirtualKeyCode::Key3 => 2,
                VirtualKeyCode::Key4 => 3,
                VirtualKeyCode::Q => 4,
                VirtualKeyCode::W => 5,
                VirtualKeyCode::E => 6,
                VirtualKeyCode::R => 7,
                VirtualKeyCode::A => 8,
                VirtualKeyCode::S => 9,
                VirtualKeyCode::D => 10,
                VirtualKeyCode::F => 11,
                VirtualKeyCode::Z => 12,
                VirtualKeyCode::X => 13,
                VirtualKeyCode::C => 14,
                VirtualKeyCode::V => 15,
                _ => return,
            },
            state,
        )
    }

    pub fn run(mut self: Rc<Self>) {
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

        let screen_w = chip8::SCREEN_SIZE.0;
        let screen_h = chip8::SCREEN_SIZE.1;
        let mut screen_raw_data: Vec<u8> = vec![0; screen_w * screen_h * 4];
        let screen_texture_id = renderer.create_texture(&device, screen_w as u32, screen_h as u32);
        let mut screen_scale = 9.0_f32;
        let mut screen_color = [1.0_f32, 1.0_f32, 1.0_f32, 1.0_f32];

        let mut last_cursor = None;

        // Event loop
        event_loop.run(move |event, _, control_flow| {
            let self_mut = Rc::get_mut(&mut self).unwrap();

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
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    virtual_keycode: Some(virtual_keycode),
                                    state,
                                    ..
                                },
                            ..
                        },
                    ..
                } => {
                    self_mut.set_key_state(virtual_keycode, state == ElementState::Pressed);
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
                    self_mut.draw_ui(&ui);

                    if self_mut._emulator.screen.is_dirty() {
                        self_mut._emulator.screen.reset_dirty();
                        let screen_w = chip8::SCREEN_SIZE.0;
                        let screen_h = chip8::SCREEN_SIZE.1;
                        for x in 0..screen_w {
                            for y in 0..screen_h {
                                let x0 = x * 4;
                                let y0 = y * 4;

                                let v = if self_mut._emulator.screen.get_pixel(x, y) {
                                    0xFF
                                } else {
                                    0
                                };

                                screen_raw_data[y0 * screen_w + x0] = v;
                                screen_raw_data[y0 * screen_w + x0 + 1] = v;
                                screen_raw_data[y0 * screen_w + x0 + 2] = v;
                                screen_raw_data[y0 * screen_w + x0 + 3] = 0xFF;
                            }
                        }
                    }

                    // Uploaded update screen
                    renderer.update_texture(
                        screen_texture_id,
                        &device,
                        &mut queue,
                        &screen_raw_data,
                        screen_w as u32,
                        screen_h as u32,
                    );

                    // Screen window
                    {
                        let window = imgui::Window::new(im_str!("Screen")).always_auto_resize(true);
                        window.build(&ui, || {
                            let size = [
                                (screen_w as f32) * screen_scale,
                                (screen_h as f32) * screen_scale,
                            ];
                            Image::new(screen_texture_id, size)
                                .tint_col(screen_color)
                                .build(&ui);
                            ui.drag_float(im_str!("Scale"), &mut screen_scale).build();
                            ui.same_line(0.0);
                            imgui::ColorEdit::new(im_str!("Color"), &mut screen_color).build(&ui);
                        });
                    }

                    let mut encoder: wgpu::CommandEncoder = device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

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
}

pub fn run() {
    let app = Rc::new(Chip8App::new());
    app.run()
}
