use memmap2::MmapOptions;

use crate::smachine::compiler::TokenType;
use std::collections::HashMap;
use std::fs;
use std::mem;
use std::thread::sleep;
use std::time::Duration;

use super::compiler::ByteCode;
const MAX_SIZE: usize = 524288;

fn core_dump(stack: &[u64; MAX_SIZE]) -> std::io::Result<()> {
    let data = format!("{:?}", stack);
    fs::write("./coredump.txt", data)?;
    Ok(())
}

trait NumberBits:
    Copy
    + std::ops::Add<Output = Self>
    + std::ops::Sub<Output = Self>
    + std::cmp::PartialEq
    + std::fmt::Debug
{
    fn from_bits(bits: u64) -> Self;
    fn into_bits(self) -> u64;
    fn max() -> u64;
}

macro_rules! impl_bits_float {
    ($type:ty, $cast:ty) => {
        impl NumberBits for $type {
            fn from_bits(bits: u64) -> Self {
                Self::from_bits(bits as $cast)
            }
            fn into_bits(self) -> u64 {
                self.to_bits() as u64
            }

            fn max() -> u64 {
                <$type>::MAX as u64
            }
        }
    };
}

macro_rules! impl_bits_int {
    ($type:ty) => {
        impl NumberBits for $type {
            fn from_bits(bits: u64) -> Self {
                bits as $type
            }
            fn into_bits(self) -> u64 {
                self as u64
            }

            fn max() -> u64 {
                <$type>::MAX as u64
            }
        }
    };
}

impl_bits_float!(f64, u64);
impl_bits_float!(f32, u32);

impl_bits_int!(u8);
impl_bits_int!(u16);
impl_bits_int!(u32);
impl_bits_int!(u64);
impl_bits_int!(i8);
impl_bits_int!(i16);
impl_bits_int!(i32);
impl_bits_int!(i64);

#[allow(dead_code)]
#[derive(Debug)]
pub struct VM {
    pc: usize,
    bin: Vec<ByteCode>,
    stack: Box<[u64; MAX_SIZE]>,
    sp: usize,
    last_pc: usize,
    proc_pc: usize,
    should_increment_pc: bool,
    // used to keep the jit functions alive
    jit_memory_store: Vec<memmap2::Mmap>,
    compiled_procs: HashMap<usize, extern "C" fn() -> u64>,
}

#[allow(dead_code)]
impl VM {
    pub fn new(bin: Vec<ByteCode>) -> VM {
        Self {
            stack: Box::new([0u64; MAX_SIZE]),
            bin,
            pc: 0,
            sp: 0,
            last_pc: 0,
            proc_pc: 0,
            should_increment_pc: true,
            compiled_procs: HashMap::new(),
            jit_memory_store: Vec::new(),
        }
    }

    #[inline(always)]
    fn eval(&mut self, binary: ByteCode) -> Option<u64> {
        let result = match TokenType::from(binary.opcode) {
            TokenType::Push => self.push(binary.value),
            TokenType::Pop => self.pop(),
            TokenType::Uadd8 => self.add::<u8>(),
            TokenType::Usub8 => self.sub::<u8>(),
            TokenType::Uadd16 => self.add::<u16>(),
            TokenType::Usub16 => self.sub::<u16>(),
            TokenType::Uadd32 => self.add::<u32>(),
            TokenType::Usub32 => self.sub::<u32>(),
            TokenType::Uadd64 => self.add::<u64>(),
            TokenType::Usub64 => self.sub::<u64>(),
            TokenType::Add8 => self.add::<i8>(),
            TokenType::Sub8 => self.add::<i8>(),
            TokenType::Add16 => self.add::<i16>(),
            TokenType::Sub16 => self.sub::<i16>(),
            TokenType::Add32 => self.add::<i32>(),
            TokenType::Sub32 => self.sub::<i32>(),
            TokenType::Add64 => self.add::<i64>(),
            TokenType::Sub64 => self.sub::<i64>(),
            TokenType::Addf64 => self.addf::<f64>(),
            TokenType::Subf64 => self.subf::<f64>(),
            TokenType::Addf32 => self.addf::<f32>(),
            TokenType::Subf32 => self.subf::<f32>(),
            TokenType::Prt => self.prt(),
            TokenType::Inc => self.inc::<u64>(),
            TokenType::Dup => self.dup(),
            TokenType::Swap => self.swap(binary.value),
            TokenType::Jmp => self.jmp(binary.value as usize),
            TokenType::Call => self.call(binary.value as usize),
            TokenType::Jmpp => self.jmpp(),
            TokenType::Cmp => self.cmp(),
            TokenType::Halt => self.halt(),
            TokenType::Ret => self.ret(),
            TokenType::Jeq => self.jeq(binary.value as usize),
            TokenType::Jnz => self.jnz(binary.value as usize),
            TokenType::Int => self.int(),
            _ => None,
        };

        if self.should_increment_pc {
            self.pc += 1;
        }

        result
    }

    fn jit(&mut self, bin: Vec<ByteCode>) -> Result<(), Box<dyn std::error::Error>> {
        let mut mmap = MmapOptions::new().len(4096).map_anon()?;

        let mut offset = 0;

        macro_rules! add_inst {
            ($num:expr) => {
                mmap[offset] = $num;
                offset += 1;
            };
        }

        for binary in bin {
            match TokenType::from(binary.opcode) {
                TokenType::Push => {
                    add_inst!(0x48);
                    add_inst!(0xB8);

                    let bytes = binary.value.to_le_bytes();
                    mmap[offset..offset + 8].copy_from_slice(&bytes);
                    offset += 8;

                    add_inst!(0x50);
                }
                TokenType::Pop => {
                    add_inst!(0x58);
                }
                TokenType::Uadd64 => {
                    add_inst!(0x58);
                    add_inst!(0x5B);
                    add_inst!(0x48);
                    add_inst!(0x01);
                    add_inst!(0xD8);
                }
                TokenType::Ret => {
                    add_inst!(0xC3);
                }
                value => {
                    todo!("Needs to do {} in jit", value);
                }
            };
        }

        let exec_mmap = mmap.make_exec();
        if let Ok(memory_map) = exec_mmap {
            let code_ptr = memory_map.as_ptr();
            self.jit_memory_store.push(memory_map);

            let jit_fn: extern "C" fn() -> u64 = unsafe { mem::transmute(code_ptr) };
            self.compiled_procs.insert(self.proc_pc, jit_fn);
        }

        Ok(())
    }

    pub fn run(&mut self) {
        while self.pc < self.bin.len() {
            self.should_increment_pc = true;
            let binary = self.bin[self.pc];

            let _ = self.eval(binary);
        }
    }

    pub fn debug_run(&mut self, flag: bool) {
        while self.pc < self.bin.len() {
            self.should_increment_pc = true;
            let binary = self.bin[self.pc];

            if flag == true {
                println!("{}, {}", TokenType::from(binary.opcode), binary.value);
                sleep(Duration::from_millis(100));
            }

            let result = self.eval(binary);

            if result.is_none() {
                println!(
                    "An error has occurred and was at instruction: {}",
                    TokenType::from(binary.opcode)
                );
                println!("Core dumped.");
                let _ = core_dump(&self.stack);
                return;
            }

            if flag == true {
                println!("stack state: {:?}, sp: {}", self.stack, self.sp);
            }
        }
    }

    fn push(&mut self, value: u64) -> Option<u64> {
        if self.sp == MAX_SIZE {
            println!("ERROR: STACK OVERFLOW!");
            return None;
        }

        self.stack[self.sp] = value;
        self.sp += 1;

        Some(0)
    }

    fn pop(&mut self) -> Option<u64> {
        if self.sp == 0 {
            return None;
        }

        self.sp -= 1;
        let value = self.stack[self.sp];

        Some(value)
    }

    fn add<T: NumberBits>(&mut self) -> Option<u64> {
        let v1 = self.pop();
        let v2 = self.pop();

        if let Some(value1) = v1
            && let Some(value2) = v2
        {
            return self.push((T::from_bits(value1) + T::from_bits(value2)).into_bits());
        }

        println!(
            "ERROR: Stack underflow, requires 2 values, but stack has {}",
            self.sp
        );
        None
    }

    fn addf<T: NumberBits>(&mut self) -> Option<u64> {
        if let (Some(bits_1), Some(bits_2)) = (self.pop(), self.pop()) {
            let v1 = T::from_bits(bits_1);
            let v2 = T::from_bits(bits_2);

            self.push((v1 + v2).into_bits());
            return Some(0);
        }
        None
    }

    fn sub<T: NumberBits>(&mut self) -> Option<u64> {
        let v1 = self.pop();
        let v2 = self.pop();

        if let Some(value1) = v1
            && let Some(value2) = v2
        {
            return self.push((T::from_bits(value2) - T::from_bits(value1)).into_bits());
        }

        println!(
            "ERROR: Stack underflow, requires 2 values, but stack has {}",
            self.sp
        );

        None
    }

    fn subf<T: NumberBits>(&mut self) -> Option<u64> {
        if let (Some(bits_1), Some(bits_2)) = (self.pop(), self.pop()) {
            let v1 = T::from_bits(bits_1);
            let v2 = T::from_bits(bits_2);

            self.push((v2 - v1).into_bits());
            return Some(0);
        }
        None
    }

    fn prt(&mut self) -> Option<u64> {
        let v1 = self.pop();

        if let Some(value1) = v1 {
            if let Some(valid) = char::from_u32(value1 as u32) {
                print!("{}", valid);
                return v1;
            } else {
                println!("{} is not a valid unicode!", value1)
            }
        }

        None
    }

    fn inc<T: NumberBits>(&mut self) -> Option<u64> {
        if let Some(value) = self.pop() {
            if T::from_bits(value) == T::from_bits(T::max()) {
                println!("ERROR: Arithmetic error, trying to add 1 to {}", T::max());
                return None;
            }

            self.push(value + 1);
            return Some(0);
        }
        None
    }

    fn dup(&mut self) -> Option<u64> {
        if let Some(value) = self.pop() {
            self.push(value);
            self.push(value);
            return Some(0);
        }
        None
    }

    fn swap(&mut self, swap_value: u64) -> Option<u64> {
        if let Some(sf) = self.pop() {
            let mut st_values: Vec<u64> = Vec::new();
            for _ in 0..swap_value {
                if let Some(val) = self.pop() {
                    st_values.push(val);
                } else {
                    println!("Tried to swap {} while sp at {}", swap_value, self.sp);
                    return None;
                }
            }
            self.push(sf);
            for val in st_values {
                self.push(val);
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

        self.pc = pc;

        Some(0)
    }

    fn call(&mut self, pc: usize) -> Option<u64> {
        self.proc_pc = pc;
        self.last_pc = self.pc;
        if let Some(v) = self.jmp(pc) {
            return Some(v);
        }
        None
    }

    fn jmpp(&mut self) -> Option<u64> {
        self.should_increment_pc = false;
        if let Some(pc) = self.pop() {
            let pc = pc as usize;
            if pc >= self.bin.len() {
                println!("ERROR: Out of bounds jump: {}", pc);
                return None;
            }

            return Some(0);
        }

        None
    }

    fn jeq(&mut self, address: usize) -> Option<u64> {
        if let Some(value) = self.pop() {
            if value == 0 {
                self.jmp(address);
            }
            return Some(0);
        }
        None
    }

    fn jnz(&mut self, address: usize) -> Option<u64> {
        if let Some(value) = self.pop() {
            if value != 0 {
                self.jmp(address);
            }
            return Some(0);
        }
        None
    }

    fn cmp(&mut self) -> Option<u64> {
        let (v1, v2) = (self.pop(), self.pop());

        if let Some(value1) = v1
            && let Some(value2) = v2
        {
            if value1 == value2 {
                self.push(0);
            } else {
                self.push(1);
            }

            return Some(0);
        }

        None
    }

    fn ret(&mut self) -> Option<u64> {
        let res = self.jit(self.bin[self.proc_pc..self.pc + 1].to_vec());
        if let Ok(_) = res {
            if let Some(func) = self.compiled_procs.get(&self.proc_pc) {
                println!("jit res: {}", func());
                self.push(func());
            }
        }
        self.pc = self.last_pc;
        Some(0)
    }

    fn int(&mut self) -> Option<u64> {
        if let Some(int_value) = self.pop() {
            match int_value {
                0 => {
                    self.halt();
                }
                _ => {}
            }
            return Some(0);
        }
        None
    }

    fn halt(&mut self) -> Option<u64> {
        self.pc = self.bin.len();
        Some(0)
    }
}
