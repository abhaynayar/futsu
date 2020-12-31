mod utils;
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// -------------------------  WASM  --------------------------//

#[wasm_bindgen]
extern {
    fn alert(s: &str);
}

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen(raw_module="../www/index.js")]
extern "C" {
    fn put_xy(x: u16, y: u16, set: u16);
}

// ------------------------------------------------------------//

#[wasm_bindgen]
pub struct Emu {
    // Registers
    pc: u16, ra: u16,
    rd: u16, rm: u16,

    // Memory
    rom: [u16; 0x8000],
    ram: [u16; 0x8000],
}

#[wasm_bindgen]
impl Emu {

    pub fn new() -> Emu {
        Emu {
            // Registers
            pc: 0, ra: 0,
            rd: 0, rm: 0,

            // Memory
            rom: [0; 0x8000],
            ram: [0; 0x8000],
        }
    }

    pub fn continue_execution(&mut self) {
        for i in 1..10000 {
            self.tick();
        }
    }

    pub fn get_opcode(&self) -> u16 {
        self.rom[self.pc as usize]
    }

    pub fn step_forward(&mut self) {
        self.tick();
    }

    pub fn set_pc(&mut self, x: u16) {
        self.pc = x;
    }

    pub fn reset(&mut self) {
        for i in 0..0x8000 {
            self.store_ram(i,0);
            self.rom[0] = 0;
        }

        self.pc = 0;
        self.ra = 0;
        self.rd = 0;
        self.rm = 0;
    }


    pub fn load_rom(&mut self, code: &str) {
        
        let mut line_counter = 0;
        for line in code.lines() {
            let mut opcode: u16 = 0;

            for (i,c) in line.chars().enumerate() {
                let current_bit = c as u16 - '0' as u16;
                opcode |= current_bit << (15-i);
            }

            self.rom[line_counter] = opcode;
            line_counter += 1;
        }
    }

    pub fn store_ram(&mut self, addr: u16, val: u16) {
        if (addr < 0x8000) {
            self.ram[addr as usize] = val;
            
            // TODO(abhay): Check if sending words is better than pixels.
            // UPDATE: Yes, we will need to use words for debug tools.
            // So shift this stuff to JS and only send modifying words.
            if addr>=0x4000 {
                let row = (addr-0x4000)/32;
                let col = (addr-0x4000)%32;
                for i in 0..16 {
                    let set = (val>>(15-i)) & 1;
                    put_xy(col+i, row, set);
                }
            }
        }
    }

    pub fn tick(&mut self) {
        //log(&format!("{:x}", self.rom[self.pc as usize]));

        self.rm = self.ram[self.ra as usize];
        let inst = self.rom[self.pc as usize];

        if inst >> 15 == 1 {
            // C Instruction (dest=comp;jump)
            let comp = (inst & 0x1fc0) >> 6;
            let dest = (inst & 0x0038) >> 3;
            let jump = (inst & 0x0003) >> 0;
            let comp_res: u16 = match comp {

                /**/ 0x2a => 0,
                /**/ 0x3f => 1,
                /**/ 0x3a => 0xffff, //-(1 as i16) as u16,
                /**/ 0x0c => self.rd,
                /**/ 0x30 => self.ra,
                /**/ 0x0d => !self.rd,
                /**/ 0x31 => !self.ra,
                /**/ 0x0f => -(self.rd as i16) as u16,
                /**/ 0x33 => self.ra,
                /**/ 0x1f => (self.rd as i16).wrapping_add(1) as u16,
                /**/ 0x37 => (self.ra as i16).wrapping_add(1) as u16,
                /**/ 0x0e => (self.rd as i16).wrapping_sub(1) as u16,
                /**/ 0x32 => (self.ra as i16).wrapping_sub(1) as u16,
                /**/ 0x02 => ((self.rd as i16) + (self.ra as i16)) as u16,
                /**/ 0x23 => ((self.rd as i16) - (self.ra as i16)) as u16,
                /**/ 0x07 => ((self.ra as i16) - (self.rd as i16)) as u16,
                /**/ 0x00 => self.rd & self.ra,
                /**/ 0x15 => self.rd | self.ra,
                /**/ 0x70 => self.rm,
                /**/ 0x71 => !self.rm,
                /**/ 0x73 => -(self.rm as i16) as u16,
                /**/ 0x77 => (self.rm as i16).wrapping_add(1) as u16,
                /**/ 0x72 => (self.rm as i16).wrapping_sub(1) as u16,
                /**/ 0x42 => ((self.rd as i16) + (self.rm as i16)) as u16,
                /**/ 0x53 => ((self.rd as i16) - (self.rm as i16)) as u16,
                /**/ 0x47 => ((self.rm as i16) - (self.rd as i16)) as u16,
                /**/ 0x40 => self.rd & self.rm,
                /**/ 0x55 => self.rd | self.rm,
                _ => {1337}
            };

            match dest {
                0x00 => {}, /*NOP*/
                0x01 => {
                    self.store_ram(self.ra, comp_res);
                },
                0x02 => {
                    self.rd = comp_res;
                },
                0x03 => {
                    self.store_ram(self.ra, comp_res);
                    self.rd = comp_res;
                },
                0x04 => {
                    self.ra = comp_res;
                },
                0x05 => {
                    self.ra = comp_res;
                    self.store_ram(self.ra, comp_res);
                },
                0x06 => {
                    self.ra = comp_res;
                    self.rd = comp_res;
                },
                0x07 => {
                    self.ra = comp_res;
                    self.rd = comp_res;
                    self.store_ram(self.ra, comp_res);
                    // TODO(abhay): Confirm the order of these statements.
                },
                _ => {}
            };

            let jump_res = match jump {
                /* INC */ 0x00 => false, // pc += 1
                /* JGT */ 0x01 => (comp_res as i16) > 0,
                /* JEQ */ 0x02 => (comp_res as i16) == 0,
                /* JGE */ 0x03 => (comp_res as i16) >= 0,
                /* JLT */ 0x04 => (comp_res as i16) < 0,
                /* JNE */ 0x05 => (comp_res as i16) != 0,
                /* JLE */ 0x06 => (comp_res as i16) <= 0,
                /* JMP */ 0x07 => true, // Unconditional
                _ => {false}
            };

            if jump_res == true {
                self.pc = self.ra-1;
            }
        } else {
            // A Instruction
            self.ra = inst & 0x7fff;
        }

        self.pc += 1;
    }
}