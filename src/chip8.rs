use rand::rngs::ThreadRng;
use rand::Rng;
use std::fs;
use std::path::PathBuf;

/// chip8 original screen size
pub const SCREEN_SIZE: (usize, usize) = (64, 32);

/// predefined font sprites
const FONT_DATA: [u8; 80] = [
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

/// Total RAM size
const MEMORY_SIZE: usize = 65535;

/// Screen buffer.
pub struct Screen {
    buffer: [u8; SCREEN_SIZE.0 * SCREEN_SIZE.1],
    dirty: bool,
}

impl Default for Screen {
    fn default() -> Self {
        Screen {
            buffer: [0u8; SCREEN_SIZE.0 * SCREEN_SIZE.1],
            dirty: true,
        }
    }
}

impl Screen {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn reset_dirty(&mut self) {
        self.dirty = false;
    }
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, v: bool) {
        self.buffer[x + y * SCREEN_SIZE.0] = v as u8;
        self.dirty = true;
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> bool {
        self.buffer[x + y * SCREEN_SIZE.0] == 1
    }

    pub fn draw_sprite(&mut self, x: usize, y: usize, sprite: &[u8]) -> bool {
        let rows = sprite.len();
        let mut collision = false;
        for j in 0..rows {
            let row = sprite[j];
            for i in 0..8 {
                let new_value = row >> (7 - i) & 0x01;
                if new_value == 1 {
                    let xi = (x + i) % SCREEN_SIZE.0;
                    let yj = (y + j) % SCREEN_SIZE.1;
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

/// chip8 keypad state
#[derive(Default)]
pub struct Keypad {
    keys: [bool; Self::KEY_COUNT],
}

impl Keypad {
    /// chip8 has 16 keys keypad
    const KEY_COUNT: usize = 16;

    pub fn is_pressed(&self, key: u8) -> bool {
        self.keys[key as usize]
    }

    pub fn set(&mut self, index: u8, down: bool) {
        self.keys[index as usize] = down;
    }

    fn get_pressed_key(&self) -> Option<u8> {
        for i in 0..self.keys.len() {
            if self.is_pressed(i as u8) {
                return Some(i as u8);
            }
        }
        None
    }
}

/// chip8 main emulator class. It is basically CPU + keypad, memory, screen etc.
#[derive(Default)]
pub struct Emulator {
    halt: bool,
    pub screen: Screen,
    pub keypad: Keypad,
    pub memory: Vec<u8>,
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
        let mut e = Emulator {
            halt: true,
            pc: 0x200,
            ..Default::default()
        };
        // 0 init all ROM
        e.memory.resize(MEMORY_SIZE, 0);

        // Copy Font data into memory
        e.memory[..FONT_DATA.len()].copy_from_slice(&FONT_DATA[..]);

        e
    }

    pub fn get_code_range(&self) -> (usize, usize) {
        (0x200, 0x200 + self.code_len)
    }

    pub fn load_rom(&mut self, romfile: &PathBuf) {
        // Reset emulator to initial state
        *self = Self::new();

        // Load ROM from file
        let contents = match fs::read(romfile) {
            Err(e) => {
                println!("Can't read file: '{0}'. Error: {1}", romfile.display(), e);
                std::process::exit(0)
            }
            Ok(f) => f,
        };

        // Copy rom in memory
        self.memory[0x200..0x200 + contents.len()].copy_from_slice(&contents[..]);
        self.code_len = contents.len();

        self.rng = rand::thread_rng();
        self.halt = false;
    }

    pub fn update(&mut self, dt: f32) {
        if !self.halt {
            self.update_timer(dt);
            self.execute_instruction();
        }
    }

    fn update_timer(&mut self, dt: f32) {
        if self.delay > 0 {
            self.total_dt += dt;
            const TIMER_PERIOD: f32 = 1.0 / 60.0;
            while self.total_dt > TIMER_PERIOD {
                self.total_dt -= TIMER_PERIOD;
                self.delay -= 1;
            }
        }
    }

    fn execute_instruction(&mut self) {
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
                if let Some(key) = self.keypad.get_pressed_key() {
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
