mod app;
mod chip8;
mod imgui_wgpu;

use app::Chip8App;
use std::rc::Rc;

fn main() {
    let app = Rc::new(Chip8App::new());
    app.run()
}
