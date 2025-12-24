use std::collections::HashMap;
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
    Inc8,
    Jmp,
    Jmpp,
    Halt,
    Ret,
    Swap,
    Int8,
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
                TokenType::Push8 => "Push8",
                TokenType::Pop8 => "Pop8",
                TokenType::Add8 => "Add8",
                TokenType::Sub8 => "Sub8",
                TokenType::Prt8 => "Prt8",
                TokenType::Int8 => "Uint8",
                TokenType::Inc8 => "Inc8",
                TokenType::Label => "Label",
                TokenType::Jmp => "Jmp",
                TokenType::Jmpp => "Jmpp",
                TokenType::Halt => "Halt",
                TokenType::Ret => "Ret",
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
                "push8" => TokenType::Push8,
                "pop8" => TokenType::Pop8,
                "add8" => TokenType::Add8,
                "sub8" => TokenType::Sub8,
                "prt8" => TokenType::Prt8,
                "inc8" => TokenType::Inc8,
                "jmp" => TokenType::Jmp,
                "jmpp" => TokenType::Jmpp,
                "halt" => TokenType::Halt,
                "swap" => TokenType::Swap,
                "ret" => TokenType::Ret,
                val => {
                    if val.ends_with(':') {
                        TokenType::Label
                    } else if val.parse::<u64>().is_ok() {
                        TokenType::Int8
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
                        if tok.value.parse::<u64>().is_ok() {
                            tok.value
                                .parse::<u64>()
                                .expect("Expected at least a u8 value!")
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
                TokenType::Int8 | TokenType::Name | TokenType::Label | TokenType::Err => {}
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
                TokenType::Push8 => {
                    let arg = iter.next()?;
                    let partial_byt = ByteCode::new(current, Some(Data::Token(arg.clone())));
                    if let Some(byt) = partial_byt {
                        byts.push(byt);
                    } else {
                        println!("Cannot push8 {}", arg);
                        return None;
                    }
                }

                TokenType::Jmp | TokenType::Swap => {
                    let arg = iter.next()?;
                    let partial_byt = ByteCode::new(current, Some(Data::Token(arg.clone())));
                    if let Some(byt) = partial_byt {
                        byts.push(byt);
                    } else {
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
