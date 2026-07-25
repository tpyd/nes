struct Flags {
    carry: bool,
    zero: bool,
    interrupt_disable: bool,
    decimal: bool,
    overflow: bool,
    negative: bool
    // TODO missing b flag
}

struct Cpu {
    a: u8,
    x: u8,
    y: u8,
    pc: u16,
    s: u8,
    flags: Flags,
    memory: [u8; 0x10_000],
    interrupt_disable_called: bool
}

impl Cpu {
    fn new() -> Self {
        let rom_bytes = include_bytes!("../nes-test-roms/instr_test-v5/rom_singles/01-basics.nes");

        let prg_rom_size = rom_bytes[4] as usize;  // 16 KB unit

        println!("PRG-ROM size: {prg_rom_size}");  // TODO make hex

        let prg_rom_size = prg_rom_size * 16_384;
        if prg_rom_size == 1 {
            panic!("Rom size = 1, needs mapped ROM starting from 0xC000");
        }
        let prg_rom_bytes = &rom_bytes[16..16+prg_rom_size];

        let mut memory = [0x0u8; 0x10_000];

        // Load rom into memory
        for (i, byte) in prg_rom_bytes.iter().enumerate() {
            memory[0x8000+i] = *byte;
        }

        let pc = u16::from_le_bytes([memory[0xfffc], memory[0xfffd]]);

        let flags = Flags {
            carry: false,
            zero: false,
            interrupt_disable: true,
            decimal: false,
            overflow: false,
            negative: false
        };

        Cpu {
            a: 0x0,
            x: 0x0,
            y: 0x0,
            pc: pc,
            s: 0xfd,
            flags,
            memory: memory,
            interrupt_disable_called: false
        }
    }

    fn run(&mut self) {
        loop {
            let instruction_byte = self.memory[self.pc as usize];
            self.pc += 0x1;
            
            match instruction_byte {
                0x78 => self.sei(),
                0x4c => self.jmp(),
                _ => panic!("Invalid instruction byte: {instruction_byte:x}")
            }

            println!("Instruction byte: 0x{instruction_byte:x}");

            // Interrupt disable is delayed by one instruction
            if self.interrupt_disable_called && instruction_byte != 0x78 {
                self.flags.interrupt_disable = true;
            }
        }
    }

    fn sei(&mut self) {
        self.interrupt_disable_called = true;    
    }

    fn jmp(&mut self) {
        self.pc = u16::from_le_bytes([self.memory[self.pc as usize], self.memory[self.pc as usize + 0x1]]);
    }
}

fn main() {
    let mut cpu = Cpu::new();
    cpu.run();
}
