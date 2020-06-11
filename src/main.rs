use glob::glob;
use imgui::*;
use std::path::PathBuf;
use std::rc::Rc;

mod app;
mod chip8;

const SCREEN_SIZE: (u32, u32) = (64, 32);

fn find_roms() -> glob::Paths {
    let exe_path = std::env::current_exe();
    let rom_path = exe_path.unwrap().parent().unwrap().join("../../roms");

    glob(rom_path.join("**/*.ch8").to_str().unwrap()).unwrap()
}

struct MyApp {
    _roms: Vec<PathBuf>,
    _emulator: chip8::Emulator,
}

impl MyApp {
    fn new() -> Self {
        let _roms = find_roms().map(|res| res.unwrap()).collect();
        MyApp {
            _roms,
            _emulator: chip8::Emulator::new(),
        }
    }

    fn draw_ui(&mut self, ui: &imgui::Ui, screen_raw: &mut Vec<u8>) {
        let window = imgui::Window::new(im_str!("ROMs"));
        window
            .size([400.0, 600.0], Condition::FirstUseEver)
            .build(&ui, || {
                for rom in &self._roms {
                    if ui.button(&ImString::new(rom.file_name().unwrap().to_str().unwrap()), [0 as f32, 0 as f32]) {
                        self._emulator.run(rom);
                    }
                }
            });

            let window = imgui::Window::new(im_str!("CPU"));
            window
                .size([400.0, 600.0], Condition::FirstUseEver)
                .build(&ui, || {
                    ui.text(format!("PC: {}", self._emulator.pc));
                });            

        if !self._emulator.is_halting() {
            self._emulator.execute_instruction()
        }

        self._emulator.update_timer(ui.io().delta_time);

        for key in 0..16 {
            self._emulator.keypad.set(key, ui.is_key_down(key as u32));
        }

        let width = SCREEN_SIZE.0 as usize;
        let height = SCREEN_SIZE.1 as usize;
        for x in 0..width {
            for y in 0..height {
                let x0 = x * 4;
                let y0 = y * 4;
                screen_raw[y0 * width + x0] = if self._emulator.screen.get_pixel(x, y) { 0xFF} else {0};
                screen_raw[y0 * width + x0 + 1] = 0;
                screen_raw[y0 * width + x0 + 2] = 0;
                screen_raw[y0 * width + x0 + 3] = 0xFF;
            }
        }
    }
}

fn main() {
    let app_desc = app::AppDesc {
        screen_width: SCREEN_SIZE.0,
        screen_height: SCREEN_SIZE.1,
    };

    let mut app = Rc::new(MyApp::new());
    app::run(
        &app_desc,
        move |ui: &imgui::Ui, screen_raw: &mut Vec<u8>| {
            Rc::get_mut(&mut app).unwrap().draw_ui(ui, screen_raw);
        },
    )
}
