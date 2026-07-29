struct Flags {
    carry: bool,
    zero: bool,
    interrupt_disable: bool,
    decimal: bool,
    overflow: bool,
    negative: bool
    // TODO missing b flag
}

#[derive(Debug)]
enum AddressingMode {
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    IndirectX,
    IndirectY
}

struct Cpu {
    a: u8,
    x: u8,
    y: u8,
    pc: u16,
    sp: u8,
    flags: Flags,
    memory: [u8; 0x10_000],
    interrupt_disable_called: bool
}

impl Cpu {
    fn new() -> Self {
        let rom_bytes = include_bytes!("../nes-test-roms/instr_test-v5/rom_singles/01-basics.nes");

        let prg_rom_size = rom_bytes[4] as usize;  // 16 KB unit

        println!("PRG-ROM size: {prg_rom_size:x}");

        let prg_rom_num = prg_rom_size * 16_384;
        if prg_rom_num == 1 {
            panic!("Rom size = 1, needs mapped ROM starting from 0xC000");
        }
        let prg_rom_bytes = &rom_bytes[16..16+prg_rom_num];

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
            sp: 0xfd,
            flags,
            memory: memory,
            interrupt_disable_called: false
        }
    }

    fn run(&mut self) {
        loop {
            let instruction_byte = self.read_next_byte();
            
            match instruction_byte {
                // STA
                0x85 => self.store_a(AddressingMode::ZeroPage),
                0x95 => self.store_a(AddressingMode::ZeroPageX),
                0x8d => self.store_a(AddressingMode::Absolute),
                0x9d => self.store_a(AddressingMode::AbsoluteX),
                0x99 => self.store_a(AddressingMode::AbsoluteY),
                0x81 => self.store_a(AddressingMode::IndirectX),
                0x91 => self.store_a(AddressingMode::IndirectY),

                // STX
                0x86 => self.store_x(AddressingMode::ZeroPage),
                0x96 => self.store_x(AddressingMode::ZeroPageY),
                0x8e => self.store_x(AddressingMode::Absolute),

                0x20 => self.jump_to_subroutine(),  // JSR
                0x4c => self.jump(),  // JMP
                0x78 => self.set_interrupt_disable(),  // SEI
                0x84 => self.store_y(),  // STY
                0x9a => self.transfer_x_to_stack_pointer(),  // TXS
                0xa0 => self.load_y(),  // LDY
                0xa2 => self.load_x(),  // LDX
                0xa9 => self.load_a(),  // LDA
                0xd8 => self.clear_decimal(),  // CLD
                0xe8 => self.increment_x(),  // INX
                _ => panic!("Invalid instruction byte: {instruction_byte:x}")
            }

            println!("Instruction byte: 0x{instruction_byte:x}");

            // Interrupt disable is delayed by one instruction
            if self.interrupt_disable_called && instruction_byte != 0x78 {
                self.flags.interrupt_disable = true;
            }
        }
    }

    fn read_next_byte(&mut self) -> u8 {
        let byte = self.memory[self.pc as usize];
        self.pc += 1;
        byte
    }
    
    fn read_next_word(&mut self) -> u16 {
        let first = self.memory[self.pc as usize];
        let second = self.memory[self.pc as usize + 1];
        self.pc += 2;
        u16::from_le_bytes([first, second])
    }

    fn peek_next_word(&self) -> u16 {
        let first = self.memory[self.pc as usize];
        let second = self.memory[self.pc as usize + 1];
        u16::from_le_bytes([first, second])
    }

    fn push_to_stack(&mut self, value: u8) {
        self.memory[0x100 + self.sp as usize] = value;
        self.sp = self.sp.wrapping_sub(1);
    }
}

// Instructions
impl Cpu {
    fn set_interrupt_disable(&mut self) {
        self.interrupt_disable_called = true;    
    }

    fn jump(&mut self) {
        self.pc = self.peek_next_word();
    }

    fn jump_to_subroutine(&mut self) {
        let new_address = self.peek_next_word();
        let return_address = self.pc + 1;
        let high = (return_address >> 8) as u8;
        let low = (return_address & 0xff) as u8;
        self.push_to_stack(high); 
        self.push_to_stack(low); 
        self.pc = new_address;
    }

    fn store_a(&mut self, addressing_mode: AddressingMode) {
        let address = match addressing_mode {
            AddressingMode::ZeroPage => self.read_next_byte() as u16,
            AddressingMode::ZeroPageX => self.read_next_byte().wrapping_add(self.x) as u16,
            AddressingMode::Absolute => self.read_next_word(),
            AddressingMode::AbsoluteX => self.read_next_word().wrapping_add(self.x as u16),
            AddressingMode::AbsoluteY => self.read_next_word().wrapping_add(self.y as u16),
            AddressingMode::IndirectX => {
                let loc = self.read_next_byte().wrapping_add(self.x);
                let first = self.memory[loc as usize];
                let second = self.memory[loc as usize + 1];
                u16::from_le_bytes([first, second]).wrapping_add(self.y as u16)
            },
            AddressingMode::IndirectY => {
                let loc = self.read_next_byte();
                let first = self.memory[loc as usize];
                let second = self.memory[loc as usize + 1];
                u16::from_le_bytes([first, second]).wrapping_add(self.y as u16)
            },
            _ => panic!("STA called with unsupported addressing mode {addressing_mode:?}")
        };

        self.memory[address as usize] = self.a;
    }

    fn store_x(&mut self, addressing_mode: AddressingMode) {
        let address = match addressing_mode {
            AddressingMode::ZeroPage => self.read_next_byte() as u16,
            AddressingMode::ZeroPageY => self.read_next_byte().wrapping_add(self.y) as u16,
            AddressingMode::Absolute => self.read_next_word(),
            _ => panic!("STX called with unsupported addressing mode {addressing_mode:?}")
        };

        self.memory[address as usize] = self.x;
    }

    fn store_y(&mut self) {
        // Zero page
        let address = self.read_next_byte();
        self.memory[address as usize] = self.y;
    }

    fn load_a(&mut self) {
        self.a = self.read_next_byte();

        self.flags.zero = self.a == 0;
        self.flags.negative = self.a >> 7 == 1;
    }

    fn load_x(&mut self) {
        self.x = self.read_next_byte();

        self.flags.zero = self.x == 0;
        self.flags.negative = self.x >> 7 == 1;
    }

    fn load_y(&mut self) {
        self.y = self.read_next_byte();

        self.flags.zero = self.y == 0;
        self.flags.negative = self.y >> 7 == 1;
    }

    fn clear_decimal(&mut self) {
        self.flags.decimal = false;
    }

    fn transfer_x_to_stack_pointer(&mut self) {
        self.sp = self.x;
    }

    fn increment_x(&mut self) {
        self.x = self.x.wrapping_add(1);

        self.flags.zero = self.x == 0;
        self.flags.negative = self.x >> 7 == 1;
    }
}

fn main() {
    let mut cpu = Cpu::new();
    cpu.run();
}
