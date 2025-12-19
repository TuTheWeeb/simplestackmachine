use std::fmt;
use std::fs;
use std::io;
use std::io::{Read, Result, Write};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum TokenType {
    Push8,
    Pop8,
    Add8,
    Sub8,
    Prt8,
    Int8,
    Err,
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "(TokenType: {})",
            match *self {
                TokenType::Push8 => "Push8",
                TokenType::Pop8 => "Pop8",
                TokenType::Add8 => "Add8",
                TokenType::Sub8 => "Sub8",
                TokenType::Prt8 => "Prt8",
                TokenType::Int8 => "Uint8",
                TokenType::Err => "Err",
            }
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct Token<'a> {
    kind: TokenType,
    value: &'a str,
}

impl<'a> Token<'a> {
    pub fn new(text: &'a str) -> Token<'a> {
        Self {
            value: text,
            kind: match text {
                "push8" => TokenType::Push8,
                "pop8" => TokenType::Pop8,
                "add8" => TokenType::Add8,
                "sub8" => TokenType::Sub8,
                "prt8" => TokenType::Prt8,
                _ => TokenType::Int8,
            },
        }
    }
}

impl<'a> fmt::Display for Token<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(value: {}, kind: {})", self.value, self.kind)
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct ByteCode {
    pub opcode: u8,
    pub value: u64,
}

#[allow(dead_code)]
impl ByteCode {
    fn new(inst: Token, arg: Option<Token>) -> ByteCode {
        if let Some(argument) = arg {
            Self {
                opcode: inst.kind as u8,
                value: argument
                    .value
                    .parse::<u64>()
                    .expect("Expected at least a u8 value!"),
            }
        } else {
            Self {
                opcode: inst.kind as u8,
                value: 0,
            }
        }
    }

    fn write_to_bin<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Write both opcode and value to a writer
        writer.write_all(&self.opcode.to_le_bytes())?;
        writer.write_all(&self.value.to_le_bytes())?;
        Ok(())
    }

    fn read_from_bin<R: Read>(reader: &mut R) -> Result<Self> {
        // Reads the opcode
        let mut opcode_buf = [0u8; 1];
        reader.read_exact(&mut opcode_buf)?;
        let opcode = u8::from_le_bytes(opcode_buf);

        // Reads the value
        let mut value_buf = [0u8; 8];
        reader.read_exact(&mut value_buf)?;
        let value = u64::from_le_bytes(value_buf);

        Ok(Self { opcode, value })
    }
}

fn byte_code_compiler(code: &str) -> Option<Vec<ByteCode>> {
    // transforms all the asm to code
    let tokens: Vec<Token> = code.split_whitespace().map(Token::new).collect();
    // make it into a iter
    let mut iter = tokens.into_iter();
    // create a buffer vector for bytecodes
    let mut byts: Vec<ByteCode> = Vec::new();
    // Read the Tokens and transform each into a bytecode
    while let Some(current) = iter.next() {
        match current.kind {
            TokenType::Push8 => {
                let arg = iter.next()?;
                if let TokenType::Int8 = arg.kind {
                    byts.push(ByteCode::new(current, Some(arg)));
                } else {
                    println!("Cannot push8 {}", arg);
                    return None;
                }
            }
            TokenType::Int8 => {
                println!("Cannot use int8 alone: {}", current);
                return None;
            }
            TokenType::Err => {
                println!("Cannot use: {}", current);
                return None;
            }
            _ => byts.push(ByteCode::new(current, None)),
        }
    }

    Some(byts)
}

pub fn write_bin(path: &str, bin: Vec<ByteCode>) -> Result<()> {
    let f = fs::File::create(path)?;
    {
        let mut writer = io::BufWriter::new(f);
        // Writes the file size;
        let _ = writer.write(&(bin.len() as u64).to_le_bytes());
        for binary in &bin {
            let v = binary.write_to_bin(&mut writer);
            match v {
                Ok(()) => (),
                Err(err) => {
                    eprintln!("ERROR: {}", err);
                    return Err(err);
                }
            }
        }
    }

    Ok(())
}

pub fn read_bin(path: &str) -> Result<Vec<ByteCode>> {
    let mut bin: Vec<ByteCode> = Vec::new();

    let f = fs::File::open(path)?;

    let mut reader = io::BufReader::new(f);
    // Reads the file size so that it can know when to stop
    let mut len_buf = [0u8; 8];
    reader.read_exact(&mut len_buf)?;
    let len: u64 = u64::from_le_bytes(len_buf);

    for _ in 0..len {
        bin.push(ByteCode::read_from_bin(&mut reader)?);
    }

    Ok(bin)
}

pub fn compile_file(path: &str) -> Option<Vec<ByteCode>> {
    match fs::read_to_string(path) {
        Ok(value) => byte_code_compiler(&value),
        Err(error) => {
            println!("Error when opening file in path: {}", path);
            eprintln!("Error: {}", error);
            None
        }
    }
}
