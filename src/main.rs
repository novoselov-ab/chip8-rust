use clap::{App, Arg};
use imgui::*;

mod app;
mod chip8;

fn load_rom_from_cli() {
    let matches = App::new("chip8-rust")
        .version("0.1")
        .about("chip8 emulator")
        .author("Anton Novoselov")
        .arg(
            Arg::with_name("ROM_FILE")
                .help("Input rom file to run")
                .required(true)
                .index(1),
        )
        .get_matches();

    let romfile = matches.value_of("ROM_FILE").unwrap();

    chip8::load_rom(romfile);
}

fn main() {
    let app_desc = app::AppDesc {
        screen_width: 64,
        screen_height: 64
    };

    app::run(app_desc, |ui: &imgui::Ui| {
        let window = imgui::Window::new(im_str!("Hello world 2"));
        window
            .size([400.0, 600.0], Condition::FirstUseEver)
            .build(&ui, || {
                if ui.button(im_str!("Button"), [200 as f32, 100 as f32]) {
                    println!("Press");
                }
                ui.text(im_str!("Hello 1!"));
                ui.text(im_str!("Hello 2!"));
            });
    })
}
