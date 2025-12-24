use crate::smachine::compiler::TokenType;
use std::fs;
use std::thread::sleep;
use std::time::Duration;

use super::compiler::ByteCode;
const MAX_SIZE: usize = 10;

#[allow(dead_code)]
#[derive(Debug)]
pub struct VM {
    pc: usize,
    bin: Vec<ByteCode>,
    stack: [u64; MAX_SIZE],
    sp: usize,
    last_pc: usize,
    should_increment_pc: bool,
}

fn core_dump(stack: &[u64; MAX_SIZE]) -> std::io::Result<()> {
    let data = format!("{:?}", stack);
    fs::write("./coredump.txt", data)?;
    Ok(())
}

#[allow(dead_code)]
impl VM {
    pub fn new(bin: Vec<ByteCode>) -> VM {
        Self {
            stack: [0u64; MAX_SIZE],
            bin,
            pc: 0,
            sp: 0,
            last_pc: 0,
            should_increment_pc: true,
        }
    }

    pub fn run(&mut self) {
        while self.pc < self.bin.len() {
            self.should_increment_pc = true;
            let binary = self.bin[self.pc].clone();
            println!("{}, {}", TokenType::from(binary.opcode), binary.value);
            sleep(Duration::from_millis(100));
            let result = match TokenType::from(binary.opcode) {
                TokenType::Push8 => self.push8(binary.value as u8),
                TokenType::Pop8 => self.pop8(),
                TokenType::Add8 => self.add8(),
                TokenType::Sub8 => self.sub8(),
                TokenType::Prt8 => self.prt8(),
                TokenType::Inc8 => self.inc8(),
                TokenType::Swap => self.swap(binary.value),
                TokenType::Jmp => self.jmp(binary.value as usize),
                TokenType::Jmpp => self.jmpp(),
                TokenType::Halt => self.halt(),
                TokenType::Ret => self.ret(),
                _ => None,
            };

            if result.is_none() {
                println!(
                    "An error has occurred and was at instruction: {}",
                    TokenType::from(binary.opcode)
                );
                println!("Core dumped.");
                let _ = core_dump(&self.stack);
                return;
            }

            if self.should_increment_pc {
                self.pc += 1;
            }
        }
        println!("stack state: {:?}", self.stack);
    }

    fn push8(&mut self, value: u8) -> Option<u64> {
        if self.sp == MAX_SIZE {
            println!("ERROR: STACK OVERFLOW!");
            return None;
        }

        self.stack[self.sp] = value as u64;
        self.sp += 1;

        Some(0)
    }
    fn pop8(&mut self) -> Option<u64> {
        if self.sp == 0 {
            return None;
        }

        self.sp -= 1;
        let value = self.stack[self.sp];

        Some(value)
    }
    fn add8(&mut self) -> Option<u64> {
        let v1 = self.pop8();
        let v2 = self.pop8();

        if let Some(value1) = v1
            && let Some(value2) = v2
        {
            return self.push8((value1 + value2) as u8);
        }

        println!(
            "ERROR: Stack underflow, add8 requires 2 values, but stack has {}",
            self.sp
        );
        None
    }
    fn sub8(&mut self) -> Option<u64> {
        let v1 = self.pop8();
        let v2 = self.pop8();

        if let Some(value1) = v1
            && let Some(value2) = v2
        {
            if value1 > value2 {
                println!(
                    "Aritmethic error: {} - {}, while both are u8.",
                    value2, value1
                );
                return None;
            }

            return self.push8((value2 - value1) as u8);
        }

        None
    }
    fn prt8(&mut self) -> Option<u64> {
        let v1 = self.pop8();

        if let Some(value1) = v1 {
            print!("{}", (value1 as u8) as char);
            return v1;
        }

        None
    }

    fn inc8(&mut self) -> Option<u64> {
        let v1 = self.pop8();

        if let Some(value1) = v1 {
            if value1 as u8 == u8::MAX {
                println!(
                    "ERROR: Arithmetic error, trying to add 1 to {} u8!",
                    u8::MAX
                );
                return None;
            }
            let v2: u8 = value1 as u8 + 1;
            self.push8(v2);
            return Some(v2 as u64);
        }

        None
    }

    fn swap(&mut self, swap_value: u64) -> Option<u64> {
        if let Some(sf) = self.pop8() {
            let mut st_values: Vec<u64> = Vec::new();
            for _ in 0..swap_value {
                if let Some(val) = self.pop8() {
                    st_values.push(val);
                } else {
                    println!("Tried to swap {} while sp at {}", swap_value, self.sp);
                    return None;
                }
            }
            self.push8(sf as u8);
            for val in st_values {
                self.push8(val as u8);
            }

            return Some(0);
        }

        None
    }

    fn jmp(&mut self, pc: usize) -> Option<u64> {
        self.should_increment_pc = false;
        if pc >= self.bin.len() {
            println!(
                "ERROR: Out of bounds jump to index {}, program length is {}",
                pc,
                self.bin.len()
            );
            return None;
        }
        self.last_pc = self.pc;
        self.pc = pc;

        Some(0)
    }

    fn jmpp(&mut self) -> Option<u64> {
        self.should_increment_pc = false;
        if let Some(pc) = self.pop8() {
            let pc = pc as usize;
            if pc >= self.bin.len() {
                println!("ERROR: Out of bounds jump: {}", pc);
                return None;
            }
            self.last_pc = self.pc;
            self.pc = pc;
            return Some(0);
        }

        None
    }

    fn ret(&mut self) -> Option<u64> {
        self.pc = self.last_pc;
        Some(0)
    }

    fn halt(&mut self) -> Option<u64> {
        self.pc = self.bin.len();
        Some(0)
    }
}
