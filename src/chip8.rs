use std::fs;
use std::path::{PathBuf};

#[derive(Default)]
pub struct Screen {
    data: Vec<u8>,
}

const SCREEN_WIDTH: usize = 64;
const SCREEN_HEIGHT: usize = 32;

impl Screen {
    pub fn new() -> Self {
        Screen {
            data: vec![0; SCREEN_WIDTH * SCREEN_HEIGHT]
        }
    }

    pub fn clear(& mut self) {
        self.data = vec![0; SCREEN_WIDTH * SCREEN_HEIGHT];
    }

    pub fn set_pixel(& mut self, x: usize, y: usize, v: u8) {
        let xw = x % SCREEN_WIDTH;
        let yw = y % SCREEN_HEIGHT;

        self.data[yw * SCREEN_HEIGHT + xw] = v;
    }
}


#[derive(Default)]
pub struct Emulator {
    _halt: bool,
    screen: Screen,
    memory: Vec<u8>,
    stack: Vec<u16>,
    data_registers: [u8; 16],
    pc: u16,
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

        self._halt = false;

        // Copy, use splice?
        for i in 0..contents.len() {
            self.memory[i + 0x200] = contents[i];
        }

        // Reset data_registers
        for i in 0..self.data_registers.len() {
            self.data_registers[i] = 0;
        }

        self.pc = 0x200;

        

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

    pub fn execute_instruction(&mut self) {
        let opcode = ((self.memory[self.pc as usize] as u16) << 8) | (self.memory[(self.pc as usize) + 1] as u16);
        let nibbles = (
            (opcode & 0xF000) >> 12 as u8,
            (opcode & 0x0F00) >> 8 as u8,
            (opcode & 0x00F0) >> 4  as u8,
            (opcode & 0x000F) >> 0  as u8,
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
            },
            (0, 0, 0xE, 0xE) => {
                // Return from a subroutine
                if let Some(adr) = self.stack.pop() {
                    self.pc = adr
                }
            }
            (0, _, _, _) => {
                // Ignore 0NNN ?
                panic!("0NNN");
            },
            (1, _, _, _) => {
                // jump to adress
                self.pc = nnn;
            },
            (2, _, _, _) => {
                // Execute subroutine starting at address NNN
                self.stack.push(self.pc);
                self.pc = nnn;
            },
            (3, _, _, _) => {
                // Skip the following instruction if the value of register VX equals NN
                if self.data_registers[x] == nn {
                    self.pc += 2;
                }
            },
            (4, _, _, _) => {
                // Skip the following instruction if the value of register VX is not equal to NN
                if self.data_registers[x] != nn {
                    self.pc += 2;
                }
            },            
            (5, _, _, 0) => {
                // Skip the following instruction if the value of register VX is equal to the value of register VY
                if self.data_registers[x] != self.data_registers[y] {
                    self.pc += 2;
                }
            },            
            (6, _, _, _) => {
                // Store number NN in register VX
                self.data_registers[x] = nn;
            },            
            (7, _, _, _) => {
                // Add the value NN to register VX
                self.data_registers[x] += nn;
            },            
            (8, _, _, 0) => {
                // Store the value of register VY in register VX
                self.data_registers[x] = self.data_registers[y];
            },                   
            (8, _, _, 1) => { 
                // Set VX to VX OR VY
                self.data_registers[x] = self.data_registers[x] | self.data_registers[y];
            },                   
            (8, _, _, 2) => {
                // Set VX to VX AND VY
                self.data_registers[x] = self.data_registers[x] & self.data_registers[y];
            },                   
            (8, _, _, 3) => {
                // Set VX to VX XOR VY
                self.data_registers[x] = self.data_registers[x] ^ self.data_registers[y];
            },                   
            _ => {

            }
        }
    }
}

