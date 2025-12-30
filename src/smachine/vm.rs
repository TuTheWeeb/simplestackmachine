use crate::smachine::compiler::TokenType;

use dynasmrt::{DynasmApi, dynasm};
use memmap2::MmapOptions;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::mem;
use std::thread::sleep;
use std::time::Duration;

use super::compiler::ByteCode;
const MAX_SIZE: usize = 10; //524288;
const INTERPRETED_EXECUTIONS: u64 = 1;

fn core_dump(stack: &[u64; MAX_SIZE]) -> std::io::Result<()> {
    let data = format!("{:?}", stack);
    fs::write("./coredump.txt", data)?;
    Ok(())
}

#[derive(Debug)]
struct CompileError;

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Could not compile!")
    }
}

impl Error for CompileError {}

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

trait NumberBitsFloat:
    Copy
    + std::ops::Add<Output = Self>
    + std::ops::Sub<Output = Self>
    + std::cmp::PartialEq
    + std::fmt::Debug
{
    fn from_bits(bits: u64) -> Self;
    fn into_bits(self) -> u64;
    fn max() -> f64;
}

macro_rules! impl_bits_float {
    ($($type:ty, $cast:ty);+) => {
        $(
        impl NumberBitsFloat for $type {
            fn from_bits(bits: u64) -> Self {
                Self::from_bits(bits as $cast)
            }
            fn into_bits(self) -> u64 {
                self.to_bits() as u64
            }

            fn max() -> f64 {
                <$type>::MAX as f64
            }
        }
        )+
    };
}

macro_rules! impl_bits_int {
    ($($type:ty);+) => {
        $(
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
        )+
    };
}

impl_bits_float!(f64, u64; f32, u32);
impl_bits_int!(u8; u16; u32; u64; i8; i16; i32; i64);

#[allow(dead_code)]
#[derive(Debug)]
pub struct VM {
    pc: usize,
    bin: Vec<ByteCode>,
    stack: Box<[u64; MAX_SIZE]>,
    sp: usize,
    proc_pc: usize,
    should_increment_pc: bool,
    // used to keep the jit functions alive
    jit_memory_store: Vec<memmap2::Mmap>,
    funcs_used: HashMap<usize, u64>,
    compiled_procs: HashMap<usize, extern "C" fn(*const u64, usize) -> u64>,
}

#[allow(dead_code)]
impl VM {
    pub fn new(bin: Vec<ByteCode>) -> VM {
        Self {
            stack: Box::new([0u64; MAX_SIZE]),
            bin,
            pc: 0,
            sp: 0,
            proc_pc: 0,
            should_increment_pc: true,
            funcs_used: HashMap::new(),
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

    fn jit(&mut self, bin: Vec<ByteCode>) -> Result<(), CompileError> {
        // check if can open the writer
        if let Ok(mut ops) = dynasmrt::x64::Assembler::new() {
            // rdi (first argument) rsi (second argument)
            // compile each instruction
            for binary in bin {
                match TokenType::from(binary.opcode) {
                    TokenType::Push => {
                        dynasm!(ops
                            ; .arch x64
                            ;  mov rax, QWORD binary.value as i64
                            ;  push rax
                        )
                    }
                    TokenType::Pop => {
                        dynasm!(ops
                            ; .arch x64
                            ; pop rcx
                        )
                    }
                    TokenType::Swap => {
                        let dist = (binary.value * 8) as i64;
                        dynasm!(ops
                            ; .arch x64
                            ; mov rax, [rsp]
                            ; mov rcx, QWORD dist
                            ; xchg rax, [rsp + rcx]
                            ; mov [rsp], rax
                        )
                    }
                    TokenType::Uadd64 => {
                        dynasm!(ops
                            ; .arch x64
                            ; pop rax
                            ; pop rbx
                            ; add rax, rbx
                            ; push rax
                        )
                    }
                    TokenType::Ret => {
                        dynasm!(ops
                            ; .arch x64
                            ; pop rax
                            ; ret
                        )
                    }
                    _ => {
                        return Err(CompileError);
                    }
                }
            }

            let code_buffer = ops.finalize().unwrap();
            let machine_code = code_buffer.to_vec();
            // check if can create the memory_map;
            if let Ok(mut mmap) = MmapOptions::new().len(machine_code.len()).map_anon() {
                mmap.copy_from_slice(&machine_code);
                let exec_mmap = mmap.make_exec();

                if let Ok(memory_map) = exec_mmap {
                    let code_ptr = memory_map.as_ptr();
                    self.jit_memory_store.push(memory_map);

                    let jit_fn: extern "C" fn(*const u64, usize) -> u64 =
                        unsafe { mem::transmute(code_ptr) };
                    self.compiled_procs.insert(self.proc_pc, jit_fn);
                    // Only returns Ok if can execute all the if blocks
                    return Ok(());
                }
            }
        }

        Err(CompileError)
    }

    pub fn run(&mut self) {
        let mut ret = Some(0);
        while self.pc < self.bin.len() {
            self.should_increment_pc = true;
            let binary = self.bin[self.pc];

            ret = self.eval(binary);
            if ret.is_none() {
                break;
            }
        }
        println!("Stack state: {:?}", self.stack);
        if ret.is_none() {
            println!("Segmentation fault (core dumped)");
            return;
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

    fn addf<T: NumberBitsFloat>(&mut self) -> Option<u64> {
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

    fn subf<T: NumberBitsFloat>(&mut self) -> Option<u64> {
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
        if self.sp > swap_value as usize
            && let Some(sf) = self.pop()
        {
            let pos = self.sp - (swap_value as usize);
            let val = self.stack[pos].clone();
            self.stack[pos] = sf;
            self.push(val);

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
        if let Some(func) = self.compiled_procs.get(&pc) {
            let res = func(self.stack.as_ptr(), pc.clone());
            self.push(res);
            return Some(0);
        }

        if let Some(func_value) = self.funcs_used.get(&pc) {
            self.funcs_used.insert(pc, func_value + 1);
        } else {
            self.funcs_used.insert(pc, 1);
        }

        self.push(self.sp as u64);
        self.push(self.pc as u64);

        self.proc_pc = pc;
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
            self.push((value1 as i64 - value2 as i64) as u64);
            return Some(0);
        }

        None
    }

    fn ret(&mut self) -> Option<u64> {
        if let Some(func_value) = self.funcs_used.get(&self.proc_pc) {
            if *func_value == INTERPRETED_EXECUTIONS {
                let _ = self.jit(self.bin[self.proc_pc..self.pc + 1].to_vec());
            }
        }

        // Ret always takes the last value on the stack
        let mut ret: u64 = 0;
        if let Some(val) = self.pop() {
            ret = val;
        }

        // takes the return addres
        if let Some(pc) = self.pop() {
            if pc as usize > self.bin.len() {
                return None;
            }
            // takes the stack pointer back
            if let Some(sp) = self.pop() {
                if sp as usize > self.stack.len() {
                    return None;
                }
                self.sp = sp as usize;
            }
            self.pc = pc as usize;
            self.push(ret);
            return Some(0);
        }

        None
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
