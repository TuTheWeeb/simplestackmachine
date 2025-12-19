use super::compiler::ByteCode;
const MAX_SIZE: usize = 5;

#[allow(dead_code)]
#[derive(Debug)]
pub struct VM {
    pc: u64,
    bin: Vec<ByteCode>,
    stack: [u64; MAX_SIZE],
    sp: usize,
}

impl VM {
    pub fn new(bin: Vec<ByteCode>) -> VM {
        Self {
            stack: [0u64; MAX_SIZE],
            bin,
            pc: 0,
            sp: 0,
        }
    }

    pub fn run(&mut self) {
        for binary in self.bin.clone() {
            //println!("Current sp: {}, stack: {:?}", self.sp, self.stack);
            let value = match binary.opcode {
                0 => self.push8(binary.value as u8),
                1 => self.pop8(),
                2 => self.add8(),
                3 => self.sub8(),
                4 => self.prt8(),
                _ => None,
            };

            if value.is_none() {
                println!("An error has occurred, was at: {}", binary.opcode);
                return;
            }
        }
    }

    fn push8(&mut self, value: u8) -> Option<u64> {
        if self.sp == MAX_SIZE - 1 {
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
}
