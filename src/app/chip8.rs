use rand::rngs::ThreadRng;
use rand::Rng;
use std::fs;
use std::path::PathBuf;

#[derive(Default)]
pub struct Screen {
    data: Vec<u8>,
}

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

// TODO: try const
pub static FONT_DATA: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

impl Screen {
    pub fn new() -> Self {
        Screen {
            data: vec![0; SCREEN_WIDTH * SCREEN_HEIGHT],
        }
    }

    pub fn clear(&mut self) {
        self.data = vec![0; SCREEN_WIDTH * SCREEN_HEIGHT];
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, v: bool) {
        self.data[x + y * SCREEN_WIDTH] = v as u8;
    }

    pub fn get_pixel(&mut self, x: usize, y: usize) -> bool {
        self.data[x + y * SCREEN_WIDTH] == 1
    }

    pub fn draw_sprite(&mut self, x: usize, y: usize, sprite: &[u8]) -> bool {
        let rows = sprite.len();
        let mut collision = false;
        for j in 0..rows {
            let row = sprite[j];
            for i in 0..8 {
                let new_value = row >> (7 - i) & 0x01;
                if new_value == 1 {
                    let xi = (x + i) % SCREEN_WIDTH;
                    let yj = (y + j) % SCREEN_HEIGHT;
                    let old_value = self.get_pixel(xi, yj);
                    if old_value {
                        collision = true;
                    }
                    self.set_pixel(xi, yj, (new_value == 1) ^ old_value);
                }
            }
        }
        return collision;
    }
}

const KEY_COUNT: usize = 16;

#[derive(Default)]
pub struct Keypad {
    keys: [bool; KEY_COUNT],
}

impl Keypad {
    pub fn is_pressed(&self, key: u8) -> bool {
        self.keys[key as usize]
    }

    pub fn set(&mut self, index: u8, down: bool) {
        self.keys[index as usize] = down;
        println!("{0} -> {1}", index, down);
    }

    pub fn reset(&mut self) {
        self.keys = [false; KEY_COUNT];
    }

    pub fn get_pressed(&self) -> Option<u8> {
        for i in 0..self.keys.len() {
            if self.is_pressed(i as u8) {
                return Some(i as u8);
            }
        }
        None
    }
}

#[derive(Default)]
pub struct Emulator {
    _halt: bool,
    pub screen: Screen,
    pub keypad: Keypad,
    memory: Vec<u8>,
    code_len: usize,
    pub stack: Vec<u16>,
    pub rs: [u8; 16], // Data registers
    pub ri: u16,      // I register
    pub pc: u16,
    rng: ThreadRng,
    pub delay: u8,
    pub total_dt: f32,
}

impl Emulator {
    pub fn new() -> Self {
        Emulator {
            _halt: true,
            screen: Screen::new(),
            ..Default::default()
        }
    }

    pub fn is_halting(&self) -> bool {
        self._halt
    }

    pub fn get_code(&self) -> &[u8] {
        if self.memory.is_empty() {
            return &[]
        }
        &self.memory[0x200..0x200 + self.code_len]
    }

    pub fn run(&mut self, romfile: &PathBuf) {
        println!("Reading... {0}", romfile.display());

        let contents = match fs::read(romfile) {
            Err(e) => {
                println!("Can't read file: '{0}'. Error: {1}", romfile.display(), e);
                std::process::exit(0)
            }
            Ok(f) => f,
        };

        self.memory = Vec::new();
        self.memory.resize(65535, 0);

        self.screen.clear();
        self.keypad.reset();

        self.delay = 0;

        self._halt = false;

        // Copy, use splice?
        for i in 0..contents.len() {
            self.memory[i + 0x200] = contents[i];
        }
        self.code_len = contents.len();

        // Copy, use splice?
        for i in 0..FONT_DATA.len() {
            self.memory[i] = FONT_DATA[i];
        }

        // Reset rs
        for i in 0..self.rs.len() {
            self.rs[i] = 0;
        }

        self.pc = 0x200;
        self.rng = rand::thread_rng();

        // println!("Content:");
        // for b in contents {
        //     println!("{0}", b);
        // }

        // println!("Memory:");
        //println!("{0}", m.bytes);

        // for b in m.bytes.iter() {
        //     println!("{0}", b);
        // }
    }

    pub fn update_timer(&mut self, dt: f32) {
        if self.delay > 0 {
            self.total_dt += dt;
            const TIMER_PERIOD: f32 = 1.0 / 60.0;
            while self.total_dt > TIMER_PERIOD {
                self.total_dt -= TIMER_PERIOD;
                self.delay -= 1;
            }
        }
    }

    pub fn execute_instruction(&mut self) {
        let opcode = ((self.memory[self.pc as usize] as u16) << 8)
            | (self.memory[(self.pc as usize) + 1] as u16);
        let nibbles = (
            (opcode & 0xF000) >> 12 as u8,
            (opcode & 0x0F00) >> 8 as u8,
            (opcode & 0x00F0) >> 4 as u8,
            (opcode & 0x000F) >> 0 as u8,
        );
        let nnn = (opcode & 0x0FFF) as u16;
        let nn = (opcode & 0x00FF) as u8;
        let x = nibbles.1 as usize;
        let y = nibbles.2 as usize;
        let n = nibbles.3 as usize;

        self.pc += 2;

        match nibbles {
            (0, 0, 0xE, 0) => {
                // clear screen
                self.screen.clear();
            }
            (0, 0, 0xE, 0xE) => {
                // Return from a subroutine
                if let Some(adr) = self.stack.pop() {
                    self.pc = adr
                }
            }
            (0, _, _, _) => {
                // Ignore 0NNN ?
                panic!("0NNN");
            }
            (1, _, _, _) => {
                // jump to adress
                self.pc = nnn;
            }
            (2, _, _, _) => {
                // Execute subroutine starting at address NNN
                self.stack.push(self.pc);
                self.pc = nnn;
            }
            (3, _, _, _) => {
                // Skip the following instruction if the value of register VX equals NN
                if self.rs[x] == nn {
                    self.pc += 2;
                }
            }
            (4, _, _, _) => {
                // Skip the following instruction if the value of register VX is not equal to NN
                if self.rs[x] != nn {
                    self.pc += 2;
                }
            }
            (5, _, _, 0) => {
                // Skip the following instruction if the value of register VX is equal to the value of register VY
                if self.rs[x] == self.rs[y] {
                    self.pc += 2;
                }
            }
            (6, _, _, _) => {
                // Store number NN in register VX
                self.rs[x] = nn;
            }
            (7, _, _, _) => {
                // Add the value NN to register VX
                self.rs[x] = self.rs[x].wrapping_add(nn);
            }
            (8, _, _, 0) => {
                // Store the value of register VY in register VX
                self.rs[x] = self.rs[y];
            }
            (8, _, _, 1) => {
                // Set VX to VX OR VY
                self.rs[x] = self.rs[x] | self.rs[y];
            }
            (8, _, _, 2) => {
                // Set VX to VX AND VY
                self.rs[x] = self.rs[x] & self.rs[y];
            }
            (8, _, _, 3) => {
                // Set VX to VX XOR VY
                self.rs[x] = self.rs[x] ^ self.rs[y];
            }
            (8, _, _, 4) => {
                // Add the value of register VY to register VX, Set VF to carry (0/1)
                let (res, overflow) = self.rs[x].overflowing_add(self.rs[y]);
                self.rs[0xF] = overflow as u8;
                self.rs[x] = res;
            }
            (8, _, _, 5) => {
                // Subtract the value of register VY from register VX, Set VF to !borrow
                let (res, overflow) = self.rs[x].overflowing_sub(self.rs[y]);
                self.rs[0xF] = !overflow as u8;
                self.rs[x] = res;
            }
            (8, _, _, 6) => {
                // Shifts VX right by one. VF is set to the value of
                // the least significant bit of VX before the shift.
                self.rs[0xF] = self.rs[x] & 0x1;
                self.rs[x] = self.rs[x] >> 1;
            }
            (8, _, _, 7) => {
                // Set register VX to the value of VY minus VX. Set VF to 00 if a borrow occurs. Set VF to 01 if a borrow does not occur
                let (res, overflow) = self.rs[y].overflowing_sub(self.rs[x]);
                self.rs[0xF] = !overflow as u8;
                self.rs[x] = res;
            }
            (8, _, _, 0xE) => {
                // Shifts VX left by one. VF is set to the value of
                // the most significant bit of VX before the shift.
                self.rs[0xF] = self.rs[x] >> 7;
                self.rs[x] = self.rs[x] << 1;
            }
            (9, _, _, 0) => {
                // Skip the following instruction if the value of register VX is not equal to the value of register VY
                if self.rs[x] != self.rs[y] {
                    self.pc += 2;
                }
            }
            (0xA, _, _, _) => {
                // Store memory address NNN in register I
                self.ri = nnn;
            }
            (0xB, _, _, _) => {
                // Jump to address NNN + V0
                self.pc = nnn + self.rs[0] as u16;
            }
            (0xC, _, _, _) => {
                // Set VX to a random number with a mask of NN
                self.rs[x] = self.rng.gen::<u8>() & nn;
            }
            (0xD, _, _, _) => {
                // Draw a sprite at position VX, VY with N bytes of sprite data starting at the address stored in I
                // Set VF to 01 if any set pixels are changed to unset, and 00 otherwise
                let c = self.screen.draw_sprite(
                    self.rs[x] as usize,
                    self.rs[y] as usize,
                    &self.memory[self.ri as usize..(self.ri + n as u16) as usize],
                );
                self.rs[0xF] = c as u8;
            }
            (0xE, _, 0x9, 0xE) => {
                // Skip the following instruction if the key corresponding to the hex value currently stored in register VX is pressed
                if self.keypad.is_pressed(self.rs[x]) {
                    self.pc += 2;
                }
            }
            (0xE, _, 0xA, 0x1) => {
                // Skip the following instruction if the key corresponding to the hex value currently stored in register VX is not pressed
                if !self.keypad.is_pressed(self.rs[x]) {
                    self.pc += 2;
                }
            }
            (0xF, _, 0x0, 0x7) => {
                // Store the current value of the delay timer in register VX
                self.rs[x] = self.delay;
            }
            (0xF, _, 0x0, 0xA) => {
                // Wait for a keypress and store the result in register VX
                if let Some(key) = self.keypad.get_pressed() {
                    self.rs[x] = key;
                } else {
                    self.pc -= 2;
                }
            }
            (0xF, _, 0x1, 0x5) => {
                // Set the delay timer to the value of register VX
                self.delay = self.rs[x];
            }
            (0xF, _, 0x1, 0x8) => {
                // Set the sound timer to the value of register VX
                // no sound?? :(
            }
            (0xF, _, 0x1, 0xE) => {
                // Add the value stored in register VX to register I
                self.ri += self.rs[x] as u16;
            }
            (0xF, _, 0x2, 0x9) => {
                // Set I to the memory address of the sprite data corresponding to the hexadecimal digit stored in register VX
                self.ri = self.rs[x] as u16 * 5;
            }
            (0xF, _, 0x3, 0x3) => {
                // Store the binary-coded decimal equivalent of the value stored in register VX at addresses I, I + 1, and I + 2
                self.memory[self.ri as usize] = self.rs[x] / 100;
                self.memory[self.ri as usize + 1] = (self.rs[x] / 10) % 10;
                self.memory[self.ri as usize + 2] = self.rs[x] % 10;
            }
            (0xF, _, 0x5, 0x5) => {
                // Store the values of registers V0 to VX inclusive in memory starting at address I is set to I + X + 1 after operation²
                self.memory[(self.ri as usize)..(self.ri + x as u16 + 1) as usize]
                    .copy_from_slice(&self.rs[0..(x as usize + 1)]);
                self.ri += (x + 1) as u16;
            }
            (0xF, _, 0x5, 0x6) => {
                // Fill registers V0 to VX inclusive with the values stored in memory starting at address I is set to I + X + 1 after operation²
                self.rs[0..(x as usize + 1)].copy_from_slice(
                    &self.memory[(self.ri as usize)..(self.ri + x as u16 + 1) as usize],
                );
                self.ri += (x + 1) as u16;
            }
            _ => {}
        }
    }
}
