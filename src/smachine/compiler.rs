use std::fmt;

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
}

pub fn byte_code_compiler(code: &str) -> Option<Vec<ByteCode>> {
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
