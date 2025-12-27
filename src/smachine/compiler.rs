use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io;
use std::io::{Read, Result, Write};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum TokenType {
    Push,
    Pop,
    Uadd8,
    Usub8,
    Uadd16,
    Usub16,
    Uadd32,
    Usub32,
    Uadd64,
    Usub64,
    Add8,
    Sub8,
    Add16,
    Sub16,
    Add32,
    Sub32,
    Add64,
    Sub64,
    Addf64,
    Subf64,
    Addf32,
    Subf32,
    Prt,
    Inc,
    Dup,
    Jmp,
    Call,
    Jmpp,
    Halt,
    Ret,
    Swap,
    Jeq,
    Jnz,
    Cmp,
    Int,
    Value,
    Label,
    Name,
    Err,
}

impl TokenType {
    pub fn from(opcode: u8) -> TokenType {
        if opcode < (TokenType::Err as u8) {
            unsafe { std::mem::transmute(opcode) }
        } else {
            TokenType::Err
        }
    }
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "(TokenType: {})",
            match *self {
                TokenType::Push => "Push",
                TokenType::Pop => "Pop",
                TokenType::Uadd8 => "Uadd8",
                TokenType::Usub8 => "Usub8",
                TokenType::Uadd16 => "uadd16",
                TokenType::Usub16 => "usub16",
                TokenType::Uadd32 => "uadd32",
                TokenType::Usub32 => "usub32",
                TokenType::Uadd64 => "uadd64",
                TokenType::Usub64 => "usub64",
                TokenType::Add8 => "sub8",
                TokenType::Sub8 => "sub8",
                TokenType::Add16 => "add16",
                TokenType::Sub16 => "sub16",
                TokenType::Add32 => "add32",
                TokenType::Sub32 => "sub32",
                TokenType::Add64 => "add64",
                TokenType::Sub64 => "sub64",
                TokenType::Addf64 => "addf64",
                TokenType::Subf64 => "subf64",
                TokenType::Addf32 => "addf32",
                TokenType::Subf32 => "subf32",
                TokenType::Prt => "Prt",
                TokenType::Value => "Value",
                TokenType::Inc => "Inc",
                TokenType::Dup => "Dup",
                TokenType::Label => "Label",
                TokenType::Jmp => "Jmp",
                TokenType::Call => "Call",
                TokenType::Jmpp => "Jmpp",
                TokenType::Jeq => "Jeq",
                TokenType::Jnz => "Jnz",
                TokenType::Cmp => "Cmp",
                TokenType::Halt => "Halt",
                TokenType::Ret => "Ret",
                TokenType::Int => "Int",
                TokenType::Swap => "Swap",
                TokenType::Name => "Name",
                TokenType::Err => "Err",
            }
        )
    }
}

#[derive(Clone, Debug)]
struct Token {
    kind: TokenType,
    value: String,
}

impl Token {
    pub fn new(text: &str) -> Token {
        Self {
            value: String::from(text),
            kind: match text {
                "push" => TokenType::Push,
                "pop" => TokenType::Pop,
                "uadd8" => TokenType::Uadd8,
                "usub8" => TokenType::Usub8,
                "uadd16" => TokenType::Uadd16,
                "usub16" => TokenType::Usub16,
                "uadd32" => TokenType::Uadd32,
                "usub32" => TokenType::Usub32,
                "uadd64" => TokenType::Uadd64,
                "usub64" => TokenType::Usub64,
                "add8" => TokenType::Add8,
                "sub8" => TokenType::Sub8,
                "add16" => TokenType::Add16,
                "sub16" => TokenType::Sub16,
                "add32" => TokenType::Add32,
                "sub32" => TokenType::Sub32,
                "add64" => TokenType::Add64,
                "sub64" => TokenType::Sub64,
                "addf64" => TokenType::Addf64,
                "subf64" => TokenType::Subf64,
                "addf32" => TokenType::Addf32,
                "subf32" => TokenType::Subf32,
                "prt" => TokenType::Prt,
                "inc" => TokenType::Inc,
                "dup" => TokenType::Dup,
                "jmp" => TokenType::Jmp,
                "call" => TokenType::Call,
                "jmpp" => TokenType::Jmpp,
                "jeq" => TokenType::Jeq,
                "jnz" => TokenType::Jnz,
                "cmp" => TokenType::Cmp,
                "halt" => TokenType::Halt,
                "int" => TokenType::Int,
                "swap" => TokenType::Swap,
                "ret" => TokenType::Ret,
                val => {
                    let is_label = val.ends_with(':');
                    let is_u64 = val.parse::<u64>().is_ok();
                    let is_i64 = val.parse::<i64>().is_ok();
                    let is_f64 = val.parse::<f64>().is_ok();

                    // check if ends with a f char and if the remaing is a parseable f32
                    let is_f32: bool = if val.ends_with('f') {
                        let mut n_val = String::from(val);
                        n_val.pop();
                        n_val.parse::<f32>().is_ok()
                    } else {
                        false
                    };

                    if is_label {
                        TokenType::Label
                    } else if is_u64 || is_i64 || is_f64 || is_f32 {
                        TokenType::Value
                    } else {
                        TokenType::Name
                    }
                }
            },
        }
    }
}

impl<'a> fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(value: {}, kind: {})", self.value, self.kind)
    }
}

#[derive(Debug)]
enum Data {
    Token(Token),
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct ByteCode {
    pub opcode: u8,
    pub value: u64,
}

#[allow(dead_code)]
impl ByteCode {
    fn new(inst: Token, arg: Option<Data>) -> Option<ByteCode> {
        if let Some(argument) = arg {
            Some(Self {
                opcode: inst.kind as u8,
                value: match argument {
                    Data::Token(tok) => {
                        let value_u64 = tok.value.parse::<u64>();
                        let value_i64 = tok.value.parse::<i64>();
                        let value_f64 = tok.value.parse::<f64>();
                        // check if ends with a f char and if the remaing is a parseable f32
                        let value_f32 = if tok.value.ends_with('f') {
                            let mut n_val = tok.value.clone();
                            n_val.pop();
                            n_val.parse::<f32>()
                        } else {
                            tok.value.parse::<f32>()
                        };

                        if let Ok(value) = value_u64 {
                            value
                        } else if let Ok(value) = value_i64 {
                            value as u64
                        } else if let Ok(value) = value_f64 {
                            value.to_bits()
                        } else if let Ok(value) = value_f32 {
                            value.to_bits() as u64
                        } else {
                            println!("ERROR: Expected at least a u8 value! {} and {}", inst, tok);
                            return None;
                        }
                    }
                },
            })
        } else {
            Some(Self {
                opcode: inst.kind as u8,
                value: 0,
            })
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

fn parse_code<'a>(code: &'a str) -> Option<Vec<Token>> {
    let mut tokens: Vec<Token> = Vec::new();
    let split_code = code.split_whitespace(); //.map(Token::new).collect();
    let mut labels: HashMap<String, String> = HashMap::new();
    let mut pos: u64 = 0;
    for val in split_code {
        if val.ends_with(':') {
            labels.insert(
                {
                    let n_val = String::from(val);
                    String::from(n_val.trim_end_matches(':'))
                },
                pos.to_string(),
            );
        } else {
            let token = Token::new(val);

            match token.kind {
                TokenType::Value | TokenType::Name | TokenType::Label | TokenType::Err => {}
                _ => {
                    pos += 1;
                }
            }

            tokens.push(token);
        }
    }

    for i in 0..tokens.len() {
        let token = &mut tokens[i];
        match token.kind {
            TokenType::Name => {
                let found = labels.get(&token.value);
                if let Some(val) = found {
                    token.value = val.clone();
                } else if !token.value.parse::<u64>().is_ok() {
                    println!("ERROR: label not found: {}", &token.value);
                    return None;
                }
            }
            _ => (),
        }
    }

    // if the last token is not halt, it then is inserted
    if let Some(token) = &tokens.last() {
        match token.kind {
            TokenType::Halt => {}
            _ => {
                tokens.push(Token::new("halt"));
            }
        };
    }

    Some(tokens)
}

fn byte_code_compiler(code: &str) -> Option<Vec<ByteCode>> {
    // transforms all the asm to code
    //let tokens: Vec<Token> = code.split_whitespace().map(Token::new).collect();
    let partial_tokens: Option<Vec<Token>> = parse_code(code);
    // create a buffer vector for bytecodes
    let mut byts: Vec<ByteCode> = Vec::new(); // make it into a iter

    if let Some(tokens) = partial_tokens {
        let mut iter = tokens.into_iter();

        // Read the Tokens and transform each into a bytecode
        while let Some(current) = iter.next() {
            match current.kind {
                TokenType::Push => {
                    let arg = iter.next()?;
                    let partial_byt = ByteCode::new(current, Some(Data::Token(arg.clone())));
                    if let Some(byt) = partial_byt {
                        byts.push(byt);
                    } else {
                        println!("Cannot push {}", arg);
                        return None;
                    }
                }

                TokenType::Jmp
                | TokenType::Jeq
                | TokenType::Jnz
                | TokenType::Swap
                | TokenType::Call => {
                    let arg = iter.next()?;
                    let partial_byt = ByteCode::new(current, Some(Data::Token(arg.clone())));
                    if let Some(byt) = partial_byt {
                        byts.push(byt);
                    } else {
                        return None;
                    }
                }

                TokenType::Value => {
                    println!("Cannot use int8 alone: {}", current);
                    return None;
                }
                TokenType::Err => {
                    println!("Cannot use: {}", current);
                    return None;
                }
                _ => {
                    let partial_byt = ByteCode::new(current, None);
                    if let Some(byt) = partial_byt {
                        byts.push(byt);
                    } else {
                        return None;
                    }
                }
            }
        }
    }

    Some(byts)
}

#[allow(dead_code)]
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

#[allow(dead_code)]
pub fn read_bin(path: String) -> Result<Vec<ByteCode>> {
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

#[allow(dead_code)]
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
