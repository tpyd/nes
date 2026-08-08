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
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
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
                // LDA
                0xa9 => self.load_a(AddressingMode::Immediate),
                0xa5 => self.load_a(AddressingMode::ZeroPage),
                0xb5 => self.load_a(AddressingMode::ZeroPageX),
                0xad => self.load_a(AddressingMode::Absolute),
                0xbd => self.load_a(AddressingMode::AbsoluteX),
                0xb9 => self.load_a(AddressingMode::AbsoluteY),
                0xa1 => self.load_a(AddressingMode::IndirectX),
                0xb1 => self.load_a(AddressingMode::IndirectY),

                // LDX
                0xa2 => self.load_x(AddressingMode::Immediate),
                0xa6 => self.load_x(AddressingMode::ZeroPage),
                0xb6 => self.load_x(AddressingMode::ZeroPageY),
                0xae => self.load_x(AddressingMode::Absolute),
                0xbe => self.load_x(AddressingMode::AbsoluteY),

                // LDY
                0xa0 => self.load_y(AddressingMode::Immediate),
                0xa4 => self.load_y(AddressingMode::ZeroPage),
                0xb4 => self.load_y(AddressingMode::ZeroPageX),
                0xac => self.load_y(AddressingMode::Absolute),
                0xbc => self.load_y(AddressingMode::AbsoluteX),

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

                // STY
                0x84 => self.store_y(AddressingMode::ZeroPage),
                0x94 => self.store_y(AddressingMode::ZeroPageX),
                0x8c => self.store_y(AddressingMode::Absolute),

                // INC, INX & INY
                0xe6 => self.increment_memory(AddressingMode::ZeroPage),
                0xf6 => self.increment_memory(AddressingMode::ZeroPageX),
                0xee => self.increment_memory(AddressingMode::Absolute),
                0xfe => self.increment_memory(AddressingMode::AbsoluteX),
                0xe8 => self.increment_x(),
                0xc8 => self.increment_y(),

                // DEC, DEX, DEY
                0xc6 => self.decrement_memory(AddressingMode::ZeroPage),
                0xd6 => self.decrement_memory(AddressingMode::ZeroPageX),
                0xce => self.decrement_memory(AddressingMode::Absolute),
                0xde => self.decrement_memory(AddressingMode::AbsoluteX),
                0xca => self.decrement_x(),
                0x88 => self.decrement_y(),

                // ADC
                0x69 => self.add_with_carry(AddressingMode::Immediate),
                0x65 => self.add_with_carry(AddressingMode::ZeroPage),
                0x75 => self.add_with_carry(AddressingMode::ZeroPageX),
                0x6d => self.add_with_carry(AddressingMode::Absolute),
                0x7d => self.add_with_carry(AddressingMode::AbsoluteX),
                0x79 => self.add_with_carry(AddressingMode::AbsoluteY),
                0x61 => self.add_with_carry(AddressingMode::IndirectX),
                0x71 => self.add_with_carry(AddressingMode::IndirectY),

                // SBC
                0xe9 => self.subtract_with_carry(AddressingMode::Immediate),
                0xe5 => self.subtract_with_carry(AddressingMode::ZeroPage),
                0xf5 => self.subtract_with_carry(AddressingMode::ZeroPageX),
                0xed => self.subtract_with_carry(AddressingMode::Absolute),
                0xfd => self.subtract_with_carry(AddressingMode::AbsoluteX),
                0xf9 => self.subtract_with_carry(AddressingMode::AbsoluteY),
                0xe1 => self.subtract_with_carry(AddressingMode::IndirectX),
                0xf1 => self.subtract_with_carry(AddressingMode::IndirectY),

                // TAX, TAY, TXA, TYA, TSX & TXS
                0xaa => self.transfer_a_to_x(),
                0xa8 => self.transfer_a_to_y(),
                0x8a => self.transfer_x_to_a(),
                0x98 => self.transfer_y_to_a(),
                0xba => self.transfer_stack_pointer_to_x(),
                0x9a => self.transfer_x_to_stack_pointer(),

                // ASL
                0x0a => self.arithmetic_shift_left(AddressingMode::Accumulator),
                0x06 => self.arithmetic_shift_left(AddressingMode::ZeroPage),
                0x16 => self.arithmetic_shift_left(AddressingMode::ZeroPageX),
                0x0e => self.arithmetic_shift_left(AddressingMode::Absolute),
                0x1e => self.arithmetic_shift_left(AddressingMode::AbsoluteX),

                // LSR
                0x4a => self.logical_shift_right(AddressingMode::Accumulator),
                0x46 => self.logical_shift_right(AddressingMode::ZeroPage),
                0x56 => self.logical_shift_right(AddressingMode::ZeroPageX),
                0x4e => self.logical_shift_right(AddressingMode::Absolute),
                0x5e => self.logical_shift_right(AddressingMode::AbsoluteX),

                // ROL
                0x2a => self.rotate_left(AddressingMode::Accumulator),
                0x26 => self.rotate_left(AddressingMode::ZeroPage),
                0x36 => self.rotate_left(AddressingMode::ZeroPageX),
                0x2e => self.rotate_left(AddressingMode::Absolute),
                0x3e => self.rotate_left(AddressingMode::AbsoluteX),

                // ROR
                0x6a => self.rotate_right(AddressingMode::Accumulator),
                0x66 => self.rotate_right(AddressingMode::ZeroPage),
                0x76 => self.rotate_right(AddressingMode::ZeroPageX),
                0x6e => self.rotate_right(AddressingMode::Absolute),
                0x7e => self.rotate_right(AddressingMode::AbsoluteX),

                // AND
                0x29 => self.bitwise_and(AddressingMode::Immediate),
                0x25 => self.bitwise_and(AddressingMode::ZeroPage),
                0x35 => self.bitwise_and(AddressingMode::ZeroPageX),
                0x2d => self.bitwise_and(AddressingMode::Absolute),
                0x3d => self.bitwise_and(AddressingMode::AbsoluteX),
                0x39 => self.bitwise_and(AddressingMode::AbsoluteY),
                0x21 => self.bitwise_and(AddressingMode::IndirectX),
                0x31 => self.bitwise_and(AddressingMode::IndirectY),

                // ORA
                0x09 => self.bitwise_or(AddressingMode::Immediate),
                0x05 => self.bitwise_or(AddressingMode::ZeroPage),
                0x15 => self.bitwise_or(AddressingMode::ZeroPageX),
                0x0d => self.bitwise_or(AddressingMode::Absolute),
                0x1d => self.bitwise_or(AddressingMode::AbsoluteX),
                0x19 => self.bitwise_or(AddressingMode::AbsoluteY),
                0x01 => self.bitwise_or(AddressingMode::IndirectX),
                0x11 => self.bitwise_or(AddressingMode::IndirectY),

                // EOR
                0x49 => self.bitwise_exclusive_or(AddressingMode::Immediate),
                0x45 => self.bitwise_exclusive_or(AddressingMode::ZeroPage),
                0x55 => self.bitwise_exclusive_or(AddressingMode::ZeroPageX),
                0x4d => self.bitwise_exclusive_or(AddressingMode::Absolute),
                0x5d => self.bitwise_exclusive_or(AddressingMode::AbsoluteX),
                0x59 => self.bitwise_exclusive_or(AddressingMode::AbsoluteY),
                0x41 => self.bitwise_exclusive_or(AddressingMode::IndirectX),
                0x51 => self.bitwise_exclusive_or(AddressingMode::IndirectY),

                // BIT
                0x24 => self.bit_test(AddressingMode::ZeroPage),
                0x2c => self.bit_test(AddressingMode::Absolute),

                // CMP
                0xc9 => self.compare_a(AddressingMode::Immediate),
                0xc5 => self.compare_a(AddressingMode::ZeroPage),
                0xd5 => self.compare_a(AddressingMode::ZeroPageX),
                0xcd => self.compare_a(AddressingMode::Absolute),
                0xdd => self.compare_a(AddressingMode::AbsoluteX),
                0xd9 => self.compare_a(AddressingMode::AbsoluteY),
                0xc1 => self.compare_a(AddressingMode::IndirectX),
                0xd1 => self.compare_a(AddressingMode::IndirectY),

                // CPX
                0xe0 => self.compare_x(AddressingMode::Immediate),
                0xe4 => self.compare_x(AddressingMode::ZeroPage),
                0xec => self.compare_x(AddressingMode::Absolute),

                // CPY
                0xc0 => self.compare_y(AddressingMode::Immediate),
                0xc4 => self.compare_y(AddressingMode::ZeroPage),
                0xcc => self.compare_y(AddressingMode::Absolute),

                // BCC, BCS, BEQ, BNE, BPL, BMI, BVC, BVS
                0x90 => self.branch_if_carry_clear(),
                0xb0 => self.branch_if_carry_set(),
                0xf0 => self.branch_if_equal(),
                0xd0 => self.branch_if_not_equal(),
                0x10 => self.branch_if_plus(),
                0x30 => self.branch_if_minus(),
                0x50 => self.branch_if_overflow_clear(),
                0x70 => self.branch_if_overflow_set(),

                // JMP, JSR, RTS, BRK & RTI
                0x4c => self.jump(AddressingMode::Absolute),
                0x6c => self.jump(AddressingMode::Indirect),
                0x20 => self.jump_to_subroutine(),
                0x60 => self.return_to_subroutine(),
                0x00 => self.r#break(),
                0x40 => self.return_from_interrupt(),

                // PHA, PLA, PHP, PLP
                0x48 => self.push_a(),
                0x68 => self.pull_a(),
                0x08 => self.push_processor_status(),
                0x28 => self.pull_processor_status(),

                // SEC, SEI, SED, CLC, CLI, CLD, CLV
                0x38 => self.set_carry(),
                0x78 => self.set_interrupt_disable(),
                0xf8 => self.set_decimal(),
                0x18 => self.clear_carry(),
                0x58 => self.clear_interrupt_disable(),
                0xd8 => self.clear_decimal(),
                0xb8 => self.clear_overflow(),

                // NOP
                0xea => {},

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

    fn pop_from_stack(&mut self) -> u8 {
        let value = self.memory[0x100 + self.sp as usize];
        self.sp = self.sp.wrapping_add(1);
        value
    }
}

// Instructions
impl Cpu {
    fn load_a(&mut self, addressing_mode: AddressingMode) {
        let value = match addressing_mode {
            AddressingMode::Immediate => self.read_next_byte(),
            AddressingMode::ZeroPage => self.memory[self.read_next_byte() as usize],
            AddressingMode::ZeroPageX => self.memory[self.read_next_byte().wrapping_add(self.x) as usize],
            AddressingMode::ZeroPageY => self.memory[self.read_next_byte().wrapping_add(self.y) as usize],
            AddressingMode::Absolute => self.memory[self.read_next_word() as usize],
            AddressingMode::AbsoluteX => self.memory[self.read_next_word().wrapping_add(self.x as u16) as usize],
            AddressingMode::AbsoluteY => self.memory[self.read_next_word().wrapping_add(self.y as u16) as usize],
            AddressingMode::IndirectX => {
                let loc = self.read_next_byte().wrapping_add(self.x);
                let first = self.memory[loc as usize];
                let second = self.memory[loc.wrapping_add(1) as usize];
                self.memory[u16::from_le_bytes([first, second]) as usize]
            },
            AddressingMode::IndirectY => {
                let loc = self.read_next_byte();
                let first = self.memory[loc as usize];
                let second = self.memory[loc.wrapping_add(1) as usize];
                self.memory[u16::from_le_bytes([first, second]) as usize]
            },
            _ => panic!("LDA called with unsupported addressing mode {addressing_mode:?}")
        };

        self.a = value;

        self.flags.zero = self.a == 0;
        self.flags.negative = self.a >> 7 == 1;
    }

    fn load_x(&mut self, addressing_mode: AddressingMode) {
        let value = match addressing_mode {
            AddressingMode::Immediate => self.read_next_byte(),
            AddressingMode::ZeroPage => self.memory[self.read_next_byte() as usize],
            AddressingMode::ZeroPageY => self.memory[self.read_next_byte().wrapping_add(self.y) as usize],
            AddressingMode::Absolute => self.memory[self.read_next_word() as usize],
            AddressingMode::AbsoluteY => self.memory[self.read_next_word().wrapping_add(self.y as u16) as usize],
            _ => panic!("LDX called with unsupported addressing mode {addressing_mode:?}")
        };

        self.x = value;

        self.flags.zero = self.x == 0;
        self.flags.negative = self.x >> 7 == 1;
    }

    fn load_y(&mut self, addressing_mode: AddressingMode) {
        let value = match addressing_mode {
            AddressingMode::Immediate => self.read_next_byte(),
            AddressingMode::ZeroPage => self.memory[self.read_next_byte() as usize],
            AddressingMode::ZeroPageX => self.memory[self.read_next_byte().wrapping_add(self.x) as usize],
            AddressingMode::Absolute => self.memory[self.read_next_word() as usize],
            AddressingMode::AbsoluteX => self.memory[self.read_next_word().wrapping_add(self.x as u16) as usize],
            _ => panic!("LDY called with unsupported addressing mode {addressing_mode:?}")
        };

        self.y = value;

        self.flags.zero = self.y == 0;
        self.flags.negative = self.y >> 7 == 1;
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
                let second = self.memory[loc.wrapping_add(1) as usize];
                u16::from_le_bytes([first, second])
            },
            AddressingMode::IndirectY => {
                let loc = self.read_next_byte();
                let first = self.memory[loc as usize];
                let second = self.memory[loc.wrapping_add(1) as usize];
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

    fn store_y(&mut self, addressing_mode: AddressingMode) {
        let address = match addressing_mode {
            AddressingMode::ZeroPage => self.read_next_byte() as u16,
            AddressingMode::ZeroPageX => self.read_next_byte().wrapping_add(self.x) as u16,
            AddressingMode::Absolute => self.read_next_word(),
            _ => panic!("STY called with unsupported addressing mode {addressing_mode:?}")
        };

        self.memory[address as usize] = self.y;
    }

    fn increment_memory(&mut self, addressing_mode: AddressingMode) {
        let address =  match addressing_mode {
            AddressingMode::ZeroPage => self.read_next_byte() as u16,
            AddressingMode::ZeroPageX => self.read_next_byte().wrapping_add(self.x) as u16,
            AddressingMode::Absolute => self.read_next_word(),
            AddressingMode::AbsoluteX => self.read_next_word().wrapping_add(self.x as u16),
            _ => panic!("INC called with unsupported addressing mode {addressing_mode:?}")
        };

        let value = self.memory[address as usize].wrapping_add(1);
        self.memory[address as usize] = value; 

        self.flags.zero = value == 0;
        self.flags.negative = value >> 7 == 1;
    }

    fn increment_x(&mut self) {
        self.x = self.x.wrapping_add(1);

        self.flags.zero = self.x == 0;
        self.flags.negative = self.x >> 7 == 1;
    }

    fn increment_y(&mut self) {
        self.y = self.y.wrapping_add(1);

        self.flags.zero = self.y == 0;
        self.flags.negative = self.y >> 7 == 1;
    }

    fn decrement_memory(&mut self, addressing_mode: AddressingMode) {
        let address =  match addressing_mode {
            AddressingMode::ZeroPage => self.read_next_byte() as u16,
            AddressingMode::ZeroPageX => self.read_next_byte().wrapping_add(self.x) as u16,
            AddressingMode::Absolute => self.read_next_word(),
            AddressingMode::AbsoluteX => self.read_next_word().wrapping_add(self.x as u16),
            _ => panic!("DEC called with unsupported addressing mode {addressing_mode:?}")
        };

        let value = self.memory[address as usize].wrapping_sub(1);
        self.memory[address as usize] = value; 

        self.flags.zero = value == 0;
        self.flags.negative = value >> 7 == 1;
    }

    fn decrement_x(&mut self) {
        self.x = self.x.wrapping_sub(1);

        self.flags.zero = self.x == 0;
        self.flags.negative = self.x >> 7 == 1;
    }

    fn decrement_y(&mut self) {
        self.y = self.y.wrapping_sub(1);

        self.flags.zero = self.y == 0;
        self.flags.negative = self.y >> 7 == 1;
    }

    fn add_with_carry(&mut self, addressing_mode: AddressingMode) {
        let memory_value = match addressing_mode {
            AddressingMode::Immediate => self.read_next_byte(),
            AddressingMode::ZeroPage => self.memory[self.read_next_byte() as usize],
            AddressingMode::ZeroPageX => self.memory[self.read_next_byte().wrapping_add(self.x) as usize],
            AddressingMode::Absolute => self.memory[self.read_next_word() as usize],
            AddressingMode::AbsoluteX => self.memory[self.read_next_word().wrapping_add(self.x as u16) as usize],
            AddressingMode::AbsoluteY => self.memory[self.read_next_word().wrapping_add(self.y as u16) as usize],
            AddressingMode::IndirectX => {
                let loc = self.read_next_byte().wrapping_add(self.x);
                let first = self.memory[loc as usize];
                let second = self.memory[loc.wrapping_add(1) as usize];
                self.memory[u16::from_le_bytes([first, second]) as usize]
            },
            AddressingMode::IndirectY => {
                let loc = self.read_next_byte();
                let first = self.memory[loc as usize];
                let second = self.memory[loc.wrapping_add(1) as usize];
                self.memory[u16::from_le_bytes([first, second]) as usize]
            },
            _ => panic!("ADC called with unsupported addressing mode {addressing_mode:?}")
        };
        
        let old_value = self.a;
        let to_add = memory_value.wrapping_add(self.flags.carry as u8);
        let (new_value, carry) = self.a.overflowing_add(to_add);
        self.a = new_value;

        self.flags.carry = carry;
        self.flags.zero = self.a == 0;
        self.flags.overflow = (old_value ^ new_value) & (to_add ^ new_value) & 0x80 != 0;
        self.flags.negative = self.a >> 7 == 1;
    }

    fn subtract_with_carry(&mut self, addressing_mode: AddressingMode) {
        let memory_value = match addressing_mode {
            AddressingMode::Immediate => self.read_next_byte(),
            AddressingMode::ZeroPage => self.memory[self.read_next_byte() as usize],
            AddressingMode::ZeroPageX => self.memory[self.read_next_byte().wrapping_add(self.x) as usize],
            AddressingMode::Absolute => self.memory[self.read_next_word() as usize],
            AddressingMode::AbsoluteX => self.memory[self.read_next_word().wrapping_add(self.x as u16) as usize],
            AddressingMode::AbsoluteY => self.memory[self.read_next_word().wrapping_add(self.y as u16) as usize],
            AddressingMode::IndirectX => {
                let loc = self.read_next_byte().wrapping_add(self.x);
                let first = self.memory[loc as usize];
                let second = self.memory[loc.wrapping_add(1) as usize];
                self.memory[u16::from_le_bytes([first, second]) as usize]
            },
            AddressingMode::IndirectY => {
                let loc = self.read_next_byte();
                let first = self.memory[loc as usize];
                let second = self.memory[loc.wrapping_add(1) as usize];
                self.memory[u16::from_le_bytes([first, second]) as usize]
            },
            _ => panic!("ADC called with unsupported addressing mode {addressing_mode:?}")
        };

        let old_value = self.a;
        let to_sub = memory_value.wrapping_sub(!self.flags.carry as u8);
        let (new_value, carry) = self.a.overflowing_sub(to_sub);
        self.a = new_value;

        self.flags.carry = carry;
        self.flags.zero = self.a == 0;
        self.flags.overflow = (old_value ^ new_value) & (!to_sub ^ new_value) & 0x80 != 0;
        self.flags.negative = self.a >> 7 == 1;
    }

    fn transfer_a_to_x(&mut self) {
        self.x = self.a;

        self.flags.zero = self.x == 0;
        self.flags.negative = self.x >> 7 == 1;  // TODO self.x & 0x80 != 0 may be better?
    }

    fn transfer_a_to_y(&mut self) {
        self.y = self.a;

        self.flags.zero = self.y == 0;
        self.flags.negative = self.y >> 7 == 1;
    }

    fn transfer_x_to_a(&mut self) {
        self.a = self.x;

        self.flags.zero = self.a == 0;
        self.flags.negative = self.a >> 7 == 1;
    }

    fn transfer_y_to_a(&mut self) {
        self.a = self.y;

        self.flags.zero = self.a == 0;
        self.flags.negative = self.a >> 7 == 1;
    }

    fn transfer_stack_pointer_to_x(&mut self) {
        self.x = self.sp;

        self.flags.zero = self.x == 0;
        self.flags.negative = self.x >> 7 == 1;
    }

    fn transfer_x_to_stack_pointer(&mut self) {
        self.sp = self.x;
    }

    fn arithmetic_shift_left(&mut self, addressing_mode: AddressingMode) {
        let mut value = match addressing_mode {
            AddressingMode::Accumulator => self.a,
            AddressingMode::ZeroPage => self.memory[self.read_next_byte() as usize],
            AddressingMode::ZeroPageX => self.memory[self.read_next_byte().wrapping_add(self.x) as usize],
            AddressingMode::Absolute => self.memory[self.read_next_word() as usize],
            AddressingMode::AbsoluteX => self.memory[self.read_next_word().wrapping_add(self.x as u16) as usize],
            _ => panic!("ASL called with unsupported addressing mode {addressing_mode:?}")
        };

        self.flags.carry = value & 0x80 != 0;
        value = value << 1;
        self.flags.zero = value == 0;
        self.flags.negative = value & 0x80 != 0;

        match addressing_mode {
            AddressingMode::Accumulator => self.a = value,
            AddressingMode::ZeroPage => self.memory[self.read_next_byte() as usize] = value,
            AddressingMode::ZeroPageX => self.memory[self.read_next_byte().wrapping_add(self.x) as usize] = value,
            AddressingMode::Absolute => self.memory[self.read_next_word() as usize] = value,
            AddressingMode::AbsoluteX => self.memory[self.read_next_word().wrapping_add(self.x as u16) as usize] = value,
            _ => panic!("ASL called with unsupported addressing mode {addressing_mode:?}")
        };
    }

    fn logical_shift_right(&mut self, addressing_mode: AddressingMode) {
        todo!()
    }

    fn rotate_left(&mut self, addressing_mode: AddressingMode) {
        todo!()
    }

    fn rotate_right(&mut self, addressing_mode: AddressingMode) {
        todo!()
    }

    fn bitwise_and(&mut self, addressing_mode: AddressingMode) {
        todo!()
    }

    fn bitwise_or(&mut self, addressing_mode: AddressingMode) {
        todo!()
    }

    fn bitwise_exclusive_or(&mut self, addressing_mode: AddressingMode) {
        todo!()
    }

    fn bit_test(&mut self, addressing_mode: AddressingMode) {
        todo!()
    }

    fn compare_a(&mut self, addressing_mode: AddressingMode) {
        todo!()
    }

    fn compare_x(&mut self, addressing_mode: AddressingMode) {
        todo!()
    }

    fn compare_y(&mut self, addressing_mode: AddressingMode) {
        todo!()
    }

    fn branch_if_carry_clear(&mut self) {
        todo!()
    }

    fn branch_if_carry_set(&mut self) {
        todo!()
    }

    fn branch_if_equal(&mut self) {
        todo!()
    }

    fn branch_if_not_equal(&mut self) {
        let address = self.read_next_byte() as i8 as i16;

        if self.flags.zero == false {
            self.pc = self.pc.wrapping_add_signed(address);
        }
    }

    fn branch_if_plus(&mut self) {
        todo!()
    }

    fn branch_if_minus(&mut self) {
        todo!()
    }

    fn branch_if_overflow_clear(&mut self) {
        todo!()
    }

    fn branch_if_overflow_set(&mut self) {
        todo!()
    }

    fn jump(&mut self, addressing_mode: AddressingMode) {
        let address = match addressing_mode {
            AddressingMode::Absolute => self.peek_next_word(),
            AddressingMode::Indirect => {
                let loc = self.peek_next_word();
                let first = self.memory[loc as usize];
                // Emulate CPU bug that skips page wrap
                let second = if loc & 0xff == 0xff {
                    self.memory[(loc & 0xff00) as usize] 
                } else {
                    self.memory[loc.wrapping_add(1) as usize]
                };
                u16::from_le_bytes([first, second])
            },
            _ => panic!("JMP called with unsupported addressing mode {addressing_mode:?}")
        };

        self.pc = address;
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

    fn return_to_subroutine(&mut self) {
        let low = self.pop_from_stack();
        let high = self.pop_from_stack();
        let address = u16::from_le_bytes([low, high]);
        self.pc = address.wrapping_add(1);
    }

    fn r#break(&mut self) {
        let pc = self.pc.wrapping_add(2);
        let high = (pc >> 8) as u8;
        self.push_to_stack(high);
        let low = pc as u8;
        self.push_to_stack(low);

        // Push NV11DIZC to stack
        let flags = (self.flags.negative as u8) << 7 |
                    (self.flags.overflow as u8) << 6 |
                    1 << 5 |
                    1 << 4 |
                    (self.flags.decimal as u8) << 3 |
                    (self.flags.interrupt_disable as u8) << 2 |
                    (self.flags.zero as u8) << 1 |
                    self.flags.carry as u8;
        self.push_to_stack(flags);

        let low = self.memory[0xfffe as usize];
        let high = self.memory[0xffff as usize];
        self.pc = u16::from_le_bytes([low, high]);

        self.flags.interrupt_disable = true;
    }

    fn return_from_interrupt(&mut self) {
        let flags = self.pop_from_stack();
        self.flags.negative = flags & 0x80 != 0;
        self.flags.overflow = flags & 0x40 != 0;
        self.flags.decimal = flags & 0x8 != 0;
        self.flags.interrupt_disable = flags & 0x4 != 0;
        self.flags.zero = flags & 0x2 != 0;
        self.flags.carry = flags & 0x1 != 0;

        let low = self.pop_from_stack();
        let high = self.pop_from_stack();
        self.pc = u16::from_le_bytes([low, high]);
    }

    fn push_a(&mut self) {
        todo!()
    }

    fn pull_a(&mut self) {
        todo!()
    }

    fn push_processor_status(&mut self) {
        todo!()
    }

    fn pull_processor_status(&mut self) {
        todo!()
    }

    fn set_carry(&mut self) {
        todo!()
    }

    fn set_interrupt_disable(&mut self) {
        self.interrupt_disable_called = true;    
    }

    fn set_decimal(&mut self) {
        todo!()
    }

    fn clear_carry(&mut self) {
        todo!()
    }

    fn clear_interrupt_disable(&mut self) {
        todo!()
    }

    fn clear_decimal(&mut self) {
        self.flags.decimal = false;
    }

    fn clear_overflow(&mut self) {
        todo!()
    }
}

fn main() {
    let mut cpu = Cpu::new();
    cpu.run();
}
