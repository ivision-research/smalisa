use std::io::{self, BufReader, Bytes, Read};

use thiserror::Error;

use crate::Arena;
use crate::{AccessFlag, Directive, Primitive, Register, Token};

#[cfg(feature = "annotations")]
use crate::AnnotationVisibility;

pub type LexResult<'a> = Result<(), LexError<'a>>;

#[derive(Error, Debug, Clone)]
pub enum LexError<'a> {
    #[error("IO error: {0:?}")]
    IO(io::ErrorKind),
    #[error("EOF")]
    EOF,
    #[error("unexpected EOF")]
    UnexpectedEOF,
    #[error("unterminated string at offset {0}")]
    UnterminatedString(usize),
    #[error("bad string escape \\{0} at offset {1}")]
    BadStringEscape(char, usize),
    #[error("unexpected byte {} at offset {}", (*.0 as char), *.1)]
    UnexpectedByte(u8, usize),
    #[error("unknown directive {0} at offset {1}")]
    UnknownDirective(&'a str, usize),
    #[error("unknown annotation visibility {0} at offset {1}")]
    UnknownAnnotationVisibility(&'a str, usize),
    #[error("unknown instruction {0} at offset {1}")]
    UnknownInstruction(&'a str, usize),
    #[error("unknown access spec {0} at offset {1}")]
    UnknownAccessSpec(&'a str, usize),
    #[error("bad numeric: {0} at offset {1}")]
    BadNumeric(&'a str, usize),
    #[error("bad register: {0} at offset {1}")]
    BadRegister(&'a str, usize),
    #[error("input contains invalid smali at offset {0}")]
    InvalidInput(usize),
}

impl<'a> LexError<'a> {
    #[inline]
    pub fn is_eof(&self) -> bool {
        matches!(*self, LexError::EOF)
    }
}

pub trait Lex<'a> {
    /// lex a single token into the passed token
    fn lex(&mut self, into: &mut Token<'a>) -> LexResult<'a>;

    /// skip_to_next_method tells the lexer to skip over the rest of the
    /// results until the first method directive. The method directive is
    /// consumed and the next lex call will be inside the header.
    fn skip_to_next_method(&mut self) -> LexResult<'a>;

    fn get_offset(&self) -> usize {
        0
    }
}

// Making the lexer stateful just makes things easier in general.
#[derive(Debug, PartialEq, Eq)]
enum LexState {
    Normal,
    MethodHeader,
    ClassDefinition,
    Field,
    SimpleName,
    Colon,
    Equal,
    #[cfg(feature = "annotations")]
    Annotation {
        is_values: bool,
        depth: u32,
    },
    #[cfg(feature = "annotations")]
    AnnotationEqual {
        depth: u32,
    },
    #[cfg(feature = "annotations")]
    AnnotationEnum {
        seen_name: bool,
        depth: u32,
    },
    #[cfg(feature = "annotations")]
    AnnotationMethod {
        seen_args: bool,
        seen_arrow: bool,
        depth: u32,
    },
}

/// Implements the default Lex for this library. This lexer is aware of which
/// tokens should just be skipped over for this library and therefore isn't
/// suitable for general usage. For example, comments are ignored.
pub struct Lexer<'a, R>
where
    R: Read,
{
    state: LexState,
    bytes: Bytes<R>,
    offset: usize,
    arena: &'a Arena,
    // Scratch space for the current token we're lexing
    scratch: Vec<u8>,
    peeked: u8,
}

impl<'a, R> Lexer<'a, R>
where
    R: Read,
{
    /// Create a new lexer with the given Read implementation. Strings are
    /// allocated in `arena`, which must outlive everything lexed from this
    /// reader.
    ///
    /// Note that the type is used directly without extra buffering. Use
    /// Lexer::new_buffered to wrap in a BufReader.
    pub fn new(reader: R, arena: &'a Arena) -> Self {
        Self {
            peeked: 0,
            state: LexState::Normal,
            bytes: reader.bytes(),
            offset: 0,
            arena,
            scratch: Vec::new(),
        }
    }
}

impl<'a, R> Lexer<'a, BufReader<R>>
where
    R: Read,
{
    /// Create a new Lexer by wrapping the provided Read implementation in a BufReader
    pub fn new_buffered(reader: R, arena: &'a Arena) -> Self {
        Self::new(BufReader::new(reader), arena)
    }
}

macro_rules! cant_eof {
    ($expr:expr) => {
        if let Err(e) = $expr {
            match e {
                LexError::EOF => {
                    return Err(LexError::UnexpectedEOF);
                }
                _ => return Err(e),
            }
        }
    };
}

impl<'a, R: Read> Lex<'a> for Lexer<'a, R> {
    fn lex(&mut self, into: &mut Token<'a>) -> LexResult<'a> {
        let c = self.next_byte_nowhitespace()?;
        match self.state {
            LexState::Normal => self.lex_normal(c, into),
            LexState::ClassDefinition => self.lex_class_definition(c, into),
            LexState::MethodHeader => self.lex_method_header(c, into),
            LexState::Field => self.lex_field(c, into),
            LexState::Colon => self.lex_after_colon(c, into),
            LexState::Equal => self.lex_after_equal(c, into),
            #[cfg(feature = "annotations")]
            LexState::Annotation { is_values, depth } => {
                self.lex_annotation(is_values, depth, c, into)
            }

            #[cfg(feature = "annotations")]
            LexState::AnnotationMethod {
                seen_args,
                seen_arrow,
                depth,
            } => self.lex_annotation_method(seen_args, seen_arrow, depth, c, into),
            #[cfg(feature = "annotations")]
            LexState::AnnotationEnum { seen_name, depth } => {
                self.lex_enum(seen_name, depth, c, into)
            }
            #[cfg(feature = "annotations")]
            LexState::AnnotationEqual { depth } => {
                let ret = self.annotation_lex_after_equal(c, into);
                if let Token::Directive(d) = *into {
                    match d {
                        Directive::Subannotation => {
                            self.state = LexState::Annotation {
                                is_values: false,
                                depth: depth + 1,
                            };
                        }
                        Directive::Enum => {
                            self.state = LexState::AnnotationEnum {
                                seen_name: false,
                                depth,
                            };
                        }
                        _ => {
                            return Err(LexError::InvalidInput(self.offset));
                        }
                    }
                } else if let Token::ArrayTypePrefix = *into {
                    // noop here because we want to fall back into this spot
                } else if let Token::OpenBrace = *into {
                    // noop here because we want to fall back into this spot
                } else if self.peek()? == b'-' {
                    // Incoming arrow
                    self.state = LexState::AnnotationMethod {
                        seen_args: false,
                        seen_arrow: false,
                        depth,
                    }
                } else {
                    self.state = LexState::Annotation {
                        is_values: true,
                        depth,
                    };
                }
                ret
            }
            LexState::SimpleName => {
                if c == b':' {
                    *into = Token::Colon;
                } else {
                    self.lex_simple_name(c, into)?;
                    self.state = LexState::Normal;
                }
                Ok(())
            }
        }
    }

    fn get_offset(&self) -> usize {
        self.offset
    }

    fn skip_to_next_method(&mut self) -> LexResult<'a> {
        let mut into = Token::Unknown;
        // Make sure we're in a normal state before moving forward
        while self.state != LexState::Normal {
            self.lex(&mut into)?;
        }
        loop {
            let c = self.next_byte()?;
            match c {
                b'.' => {
                    self.lex_directive(&mut into)?;
                    match into {
                        Token::Directive(d) => {
                            match d {
                                Directive::Method => {
                                    self.state = LexState::MethodHeader;
                                    return Ok(());
                                }
                                Directive::Field => {
                                    self.state = LexState::Normal;
                                }
                                _ => {}
                            }
                            self.discard_line();
                        }
                        _ => self.discard_line(),
                    }
                }
                _ => self.discard_line(),
            }
        }
    }
}

impl<'a, R: Read> Lexer<'a, R> {
    fn lex_normal(&mut self, c: u8, into: &mut Token<'a>) -> LexResult<'a> {
        match c {
            b'\'' => self.lex_char(into)?,
            b'"' => self.quoted_str(into)?,
            // Ignore comments
            b'#' => {
                self.discard_line();
                self.lex(into)?;
            }
            b'=' => {
                *into = Token::Equal;
                self.state = LexState::Equal;
            }
            b':' => {
                *into = Token::Colon;
                self.state = LexState::Colon;
            }
            // TODO I think the parser works without the Comma token in all
            // cases so maybe this can be reworked to just ignore the comma
            // and lex the next token?
            b',' => {
                *into = Token::Comma;
                //self.lex(into)?;
            }
            b'-' | b'0'..=b'9' => {
                let b = self.peek()?;
                if b == b'>' {
                    self.consume_peek();
                    *into = Token::Arrow;
                    self.state = LexState::SimpleName;
                } else {
                    self.lex_numeric(c, into)?
                }
            }
            b'(' => {
                self.take_until_byte(b')')?;
                self.consume_peek();
                let method_args = self.take_str();
                *into = Token::MethodArgs(method_args);
                //*into = Token::OpenParen;
            }
            // Don't need this
            //b')' => {
            //    *into = Token::CloseParen;
            //}
            b'{' => {
                *into = Token::OpenBrace;
            }
            b'}' => {
                *into = Token::CloseBrace;
            }
            b'[' => {
                *into = Token::ArrayTypePrefix;
            }
            b'L' => self.lex_class_descriptor(c, into)?,
            b'J' => {
                *into = Token::PrimitiveType(Primitive::Long);
            }
            b'C' => {
                *into = Token::PrimitiveType(Primitive::Char);
            }
            b'I' => {
                *into = Token::PrimitiveType(Primitive::Int);
            }
            b'B' => {
                *into = Token::PrimitiveType(Primitive::Byte);
            }
            b'S' => {
                *into = Token::PrimitiveType(Primitive::Short);
            }
            b'Z' => {
                *into = Token::PrimitiveType(Primitive::Bool);
            }
            b'V' => {
                *into = Token::PrimitiveType(Primitive::Void);
            }
            b'F' => {
                *into = Token::PrimitiveType(Primitive::Float);
            }
            b'D' => {
                *into = Token::PrimitiveType(Primitive::Double);
            }
            b'p' | b'v' => {
                let b = self.peek()?;
                if b.is_ascii_digit() {
                    self.push(c);
                    self.take_while(|b| b.is_ascii_digit())?;

                    let reg = self
                        .check_str(|s| Register::parse(s))
                        .ok_or_else(|| LexError::BadRegister(self.take_str(), self.offset))?;
                    self.clear_buf();
                    *into = Token::Register(reg);
                } else {
                    self.lex_instruction_wrapper(c, into)?;
                }
            }
            b'.' => {
                let next = self.peek()?;
                if next == b'.' {
                    *into = Token::DotDot;
                    self.consume_peek();
                    return Ok(());
                }
                self.lex_directive(into)?;
                if let Token::Directive(d) = *into {
                    match d {
                        // Some directives we don't really care about
                        Directive::Source
                        | Directive::Line
                        | Directive::Locals
                        | Directive::Registers
                        | Directive::Prologue
                        | Directive::RestartLocal
                        | Directive::Local
                        | Directive::EndLocal => {
                            self.discard_line();
                            return self.lex(into);
                        }
                        Directive::Class => {
                            self.state = LexState::ClassDefinition;
                        }
                        Directive::Method => {
                            self.state = LexState::MethodHeader;
                        }
                        Directive::Field => {
                            self.state = LexState::Field;
                        }
                        #[cfg(feature = "annotations")]
                        Directive::Annotation => {
                            self.state = LexState::Annotation {
                                is_values: false,
                                depth: 0,
                            };
                        }
                        #[cfg(not(feature = "annotations"))]
                        Directive::Annotation => {
                            self.skip_annotation()?;
                            return self.lex(into);
                        }
                        _ => {}
                    }
                }
            }
            b'a'..=b'z' => self.lex_instruction_wrapper(c, into)?,
            _ => return Err(LexError::UnexpectedByte(c, self.offset)),
        }
        Ok(())
    }

    fn lex_instruction_wrapper(&mut self, c: u8, into: &mut Token<'a>) -> LexResult<'a> {
        self.lex_instruction(c, into)?;
        match into {
            // We use the lexer to just skip these for now since we haven't implemented them
            Token::Instruction(INS_INVOKE_CUSTOM) | Token::Instruction(INS_INVOKE_CUSTOM_RANGE) => {
                self.discard_line();
                self.lex(into)
            }
            _ => Ok(()),
        }
    }

    #[cfg(not(feature = "annotations"))]
    fn skip_annotation(&mut self) -> LexResult<'a> {
        let mut into = Token::Unknown;
        // Keep reading until we get an end annotation
        loop {
            let c = self.next_byte_nowhitespace()?;
            if c != b'.' {
                self.discard_line();
            } else {
                self.lex_directive(&mut into)?;
                match into {
                    Token::Directive(Directive::EndAnnotation) => return Ok(()),
                    _ => continue,
                }
            }
        }
    }

    #[cfg(feature = "annotations")]
    fn lex_annotation_method(
        &mut self,
        seen_args: bool,
        seen_arrow: bool,
        depth: u32,
        c: u8,
        into: &mut Token<'a>,
    ) -> LexResult<'a> {
        if seen_args {
            match c {
                b'[' => {
                    *into = Token::ArrayTypePrefix;
                }
                b'L' => self.lex_class_descriptor(c, into)?,
                b'J' => {
                    *into = Token::PrimitiveType(Primitive::Long);
                }
                b'C' => {
                    *into = Token::PrimitiveType(Primitive::Char);
                }
                b'I' => {
                    *into = Token::PrimitiveType(Primitive::Int);
                }
                b'B' => {
                    *into = Token::PrimitiveType(Primitive::Byte);
                }
                b'S' => {
                    *into = Token::PrimitiveType(Primitive::Short);
                }
                b'Z' => {
                    *into = Token::PrimitiveType(Primitive::Bool);
                }
                b'V' => {
                    *into = Token::PrimitiveType(Primitive::Void);
                }
                b'F' => {
                    *into = Token::PrimitiveType(Primitive::Float);
                }
                b'D' => {
                    *into = Token::PrimitiveType(Primitive::Double);
                }
                _ => return Err(LexError::InvalidInput(self.offset)),
            }
            if c != b'[' {
                self.state = LexState::Annotation {
                    is_values: true,
                    depth,
                };
            }
            return Ok(());
        }
        match c {
            b'-' if !seen_arrow => {
                let b = self.next_byte()?;
                if b == b'>' {
                    *into = Token::Arrow;
                    self.state = LexState::AnnotationMethod {
                        seen_arrow: true,
                        seen_args,
                        depth,
                    };
                    Ok(())
                } else {
                    Err(LexError::InvalidInput(self.offset))
                }
            }
            b'(' => {
                self.take_until_byte(b')')?;
                self.consume_peek();
                let method_args = self.take_str();
                *into = Token::MethodArgs(method_args);
                self.state = LexState::AnnotationMethod {
                    seen_args: true,
                    seen_arrow,
                    depth,
                };
                Ok(())
            }
            _ => self.lex_simple_name(c, into),
        }
    }

    #[cfg(feature = "annotations")]
    fn lex_enum(
        &mut self,
        seen_name: bool,
        depth: u32,
        c: u8,
        into: &mut Token<'a>,
    ) -> LexResult<'a> {
        match c {
            b':' => {
                self.state = LexState::AnnotationEnum {
                    seen_name: true,
                    depth,
                };
                *into = Token::Colon;
            }
            b'-' => {
                let b = self.next_byte()?;
                if b == b'>' {
                    *into = Token::Arrow;
                } else {
                    return Err(LexError::InvalidInput(self.offset));
                }
            }
            b'L' => {
                self.push(c);
                let mut is_clazz = true;
                self.take_while(|c| c != b';' && c != b':')?;
                let b = self.next_byte()?;
                if b == b':' {
                    is_clazz = false;
                    self.set_peeked(b);
                } else {
                    self.push(b);
                }
                let s = self.take_str();
                if is_clazz {
                    if seen_name {
                        self.state = LexState::Annotation {
                            is_values: true,
                            depth,
                        };
                    }
                    *into = Token::ClassDescriptor(s);
                } else {
                    *into = Token::SimpleName(s);
                }
            }
            _ => {
                self.lex_simple_name(c, into)?;
            }
        }
        Ok(())
    }

    #[cfg(feature = "annotations")]
    fn annotation_lex_after_equal(&mut self, c: u8, into: &mut Token<'a>) -> LexResult<'a> {
        if c == b'{' {
            *into = Token::OpenBrace;
            return Ok(());
        } else if c == b'}' {
            *into = Token::CloseBrace;
            return Ok(());
        } else if c == b'[' {
            *into = Token::ArrayTypePrefix;
            return Ok(());
        }
        let peeked = self.peek()?;
        if peeked.is_ascii_whitespace() {
            self.consume_peek();
            match c {
                b'J' => {
                    *into = Token::PrimitiveType(Primitive::Long);
                    return Ok(());
                }
                b'C' => {
                    *into = Token::PrimitiveType(Primitive::Char);
                    return Ok(());
                }
                b'I' => {
                    *into = Token::PrimitiveType(Primitive::Int);
                    return Ok(());
                }
                b'B' => {
                    *into = Token::PrimitiveType(Primitive::Byte);
                    return Ok(());
                }
                b'S' => {
                    *into = Token::PrimitiveType(Primitive::Short);
                    return Ok(());
                }
                b'Z' => {
                    *into = Token::PrimitiveType(Primitive::Bool);
                    return Ok(());
                }
                b'V' => {
                    *into = Token::PrimitiveType(Primitive::Void);
                    return Ok(());
                }
                b'F' => {
                    *into = Token::PrimitiveType(Primitive::Float);
                    return Ok(());
                }
                b'D' => {
                    *into = Token::PrimitiveType(Primitive::Double);
                    return Ok(());
                }
                _ => {}
            }
        }
        self.lex_after_equal(c, into)
    }

    fn lex_after_equal(&mut self, c: u8, into: &mut Token<'a>) -> LexResult<'a> {
        self.state = LexState::Normal;
        match c {
            b'"' => self.quoted_str(into),
            b'\'' => self.lex_char(into),
            b't' | b'f' => {
                *into = Token::BoolLiteral(c == b't');
                self.skip_until_whitespace()
            }
            b'n' => {
                *into = Token::NullLiteral;
                self.skip_until_whitespace()
            }
            // NaNf
            b'N' => {
                *into = Token::NumericLiteral("NaNf");
                self.skip_until_whitespace()
            }
            b'I' | b'-' | b'0'..=b'9' => self.lex_numeric(c, into),
            b'.' => self.lex_directive(into),
            b'{' => {
                *into = Token::OpenBrace;
                Ok(())
            }
            b'L' => self.lex_class_descriptor(c, into),
            _ => Err(LexError::InvalidInput(self.offset)),
        }
    }

    // This function exists because, while lexing in the normal state, :cond_12 and
    // FIELD:I are indistinguishable. Luckily, the only things that can follow a colon
    // in the field case are the primitive types, L classes, or an array descriptor.
    fn lex_after_colon(&mut self, c: u8, into: &mut Token<'a>) -> LexResult<'a> {
        self.state = LexState::Normal;
        match c {
            b'a'..=b'z' => self.lex_simple_name(c, into),
            _ => self.lex_normal(c, into),
        }
    }

    #[cfg(feature = "annotations")]
    fn lex_annotation_header(&mut self, depth: u32, c: u8, into: &mut Token<'a>) -> LexResult<'a> {
        let is_sub = depth > 0;
        if c == b'L' {
            self.lex_class_descriptor(c, into)?;
            self.state = LexState::Annotation {
                is_values: true,
                depth,
            };
            return Ok(());
        }
        if is_sub {
            return Err(LexError::InvalidInput(self.offset));
        }
        let vis = match c {
            b's' => AnnotationVisibility::System,
            b'b' => AnnotationVisibility::Build,
            b'r' => AnnotationVisibility::Runtime,
            _ => {
                self.take_until_whitespace()?;
                let s = self.take_str();
                return Err(LexError::UnknownAnnotationVisibility(s, self.offset));
            }
        };
        *into = Token::AnnotationVisibility(vis);
        self.skip_until_whitespace()
    }

    #[cfg(feature = "annotations")]
    fn lex_annotation(
        &mut self,
        is_values: bool,
        depth: u32,
        c: u8,
        into: &mut Token<'a>,
    ) -> LexResult<'a> {
        if is_values {
            self.lex_annotation_values(depth, c, into)
        } else {
            self.lex_annotation_header(depth, c, into)
        }
    }

    #[cfg(feature = "annotations")]
    fn lex_annotation_values(&mut self, depth: u32, c: u8, into: &mut Token<'a>) -> LexResult<'a> {
        match c {
            b'=' => {
                *into = Token::Equal;
                self.state = LexState::AnnotationEqual { depth };
            }
            b'.' => {
                self.lex_directive(into)?;
                if let Token::Directive(d) = *into {
                    match d {
                        Directive::Subannotation => {
                            // Into a header
                            self.state = LexState::Annotation {
                                is_values: false,
                                depth: depth + 1,
                            };
                        }
                        Directive::EndSubannotation => {
                            if depth == 0 {
                                return Err(LexError::InvalidInput(self.offset));
                            }
                            self.state = LexState::Annotation {
                                is_values: true,
                                depth: depth - 1,
                            };
                        }
                        Directive::EndAnnotation => {
                            if depth > 0 {
                                return Err(LexError::InvalidInput(self.offset));
                            }
                            self.state = LexState::Normal;
                        }
                        Directive::Enum => {
                            self.state = LexState::AnnotationEnum {
                                seen_name: false,
                                depth,
                            };
                        }
                        _ => {
                            return Err(LexError::InvalidInput(self.offset));
                        }
                    }
                }
            }
            b'}' => {
                *into = Token::CloseBrace;
            }
            b',' => {
                *into = Token::Comma;
                self.state = LexState::AnnotationEqual { depth };
            }
            _ => self.lex_simple_name(c, into)?,
        }
        Ok(())
    }

    #[inline]
    fn lex_simple_name(&mut self, c: u8, into: &mut Token<'a>) -> LexResult<'a> {
        if c == b'`' {
            self.lex_backtic(into)
        } else {
            self.push(c);
            self.take_while(|b| !b.is_ascii_whitespace() && b != b'(' && b != b':' && b != b'}')?;
            *into = Token::SimpleName(self.take_str());
            Ok(())
        }
    }

    fn lex_char(&mut self, into: &mut Token<'a>) -> LexResult<'a> {
        // It will be either 'ASCII' or '\uXXXX'
        let mut escaped = false;
        // A bit lame, but we were forgetting about escape sequences other than \uXXXX
        loop {
            let c = self.next_byte()?;
            if escaped {
                if c == b'u' {
                    self.push(b'\\');
                }
                self.push(c);
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'\'' {
                break;
            } else {
                self.push(c);
            }
        }
        let s = self.take_str();
        *into = Token::CharLiteral(s);
        Ok(())
    }

    fn lex_numeric(&mut self, c: u8, into: &mut Token<'a>) -> LexResult<'a> {
        self.push(c);
        // TODO This is kinda a hack, but whatever, low priority

        loop {
            let pc = self.peek()?;
            if pc.is_ascii_whitespace() {
                self.consume_peek();
                if let Err(e) = self.skip_while(|b| b != b'\n' && b.is_ascii_whitespace()) {
                    match e {
                        LexError::EOF => break,
                        _ => return Err(e),
                    }
                }
                let pc = self.peek()?;
                if pc == b'#' {
                    self.clear_buf();
                    self.consume_peek();
                    self.skip_while(|b| b.is_ascii_whitespace())?;
                    continue;
                } else {
                    break;
                }
            } else if pc == b',' {
                break;
            } else {
                self.push(pc);
            }
            self.consume_peek();
        }
        let s = self.take_str();
        *into = Token::NumericLiteral(s);
        Ok(())
    }

    #[inline]
    fn lex_backtic(&mut self, into: &mut Token<'a>) -> LexResult<'a> {
        self.take_until_byte(b'`')?;
        self.consume_peek();
        *into = Token::SimpleName(self.take_str());
        Ok(())
    }

    fn lex_class_definition(&mut self, c: u8, into: &mut Token<'a>) -> LexResult<'a> {
        match c {
            b'L' => {
                let res = self.lex_class_descriptor(c, into);
                self.state = LexState::Normal;
                res
            }
            _ => {
                self.push(c);
                self.take_until_whitespace()?;
                let s = self.take_str();
                *into = Token::AccessSpec(AccessFlag::parse(s));
                Ok(())
            }
        }
    }

    fn lex_field(&mut self, c: u8, into: &mut Token<'a>) -> LexResult<'a> {
        if c == b':' {
            *into = Token::Colon;
            self.state = LexState::Normal;
            return Ok(());
        } else if c == b'`' {
            return self.lex_backtic(into);
        }
        self.push(c);
        loop {
            let c = self.peek()?;
            if c == b':' {
                // Leave the : for the next lex.
                let name = self.take_str();
                *into = Token::SimpleName(name);
                self.state = LexState::Normal;
                return Ok(());
            } else if c == b' ' {
                let af = self
                    .check_str(|s| AccessFlag::maybe_parse(s))
                    .ok_or_else(|| {
                        let s = self.take_str();
                        LexError::UnknownAccessSpec(s, self.offset)
                    })?;

                self.clear_buf();

                *into = Token::AccessSpec(af);
                self.consume_peek();
                return Ok(());
            }
            self.push(c);
            self.consume_peek();
        }
    }

    fn lex_method_header(&mut self, c: u8, into: &mut Token<'a>) -> LexResult<'a> {
        if c == b'(' {
            // Ignore the ( and ), the parser knows what comes next without
            // those
            self.take_until_byte(b')')?;
            self.consume_peek();
            let method_args = self.take_str();
            *into = Token::MethodArgs(method_args);
            self.state = LexState::Normal;
            return Ok(());
        } else if c == b'`' {
            return self.lex_backtic(into);
        }
        self.push(c);
        loop {
            let c = self.peek()?;
            if c == b'(' {
                // Leave the b'(' for the next lex.
                let name = self.take_str();
                *into = Token::SimpleName(name);
                return Ok(());
            } else if c == b' ' {
                let af = self
                    .check_str(|s| AccessFlag::maybe_parse(s))
                    .ok_or_else(|| {
                        let s = self.take_str();
                        LexError::UnknownAccessSpec(s, self.offset)
                    })?;
                self.clear_buf();
                *into = Token::AccessSpec(af);
                self.consume_peek();
                return Ok(());
            }
            self.push(c);
            self.consume_peek();
        }
    }

    #[inline]
    fn lex_class_descriptor(&mut self, c: u8, into: &mut Token<'a>) -> LexResult<'a> {
        self.push(c);
        cant_eof!(self.take_until_byte_consume(b';'));
        self.push(b';');
        let s = self.take_str();
        *into = Token::ClassDescriptor(s);
        Ok(())
    }

    #[inline]
    fn skip_until_whitespace(&mut self) -> LexResult<'a> {
        self.skip_while(|b| !b.is_ascii_whitespace())
    }

    #[inline]
    fn take_until_whitespace(&mut self) -> LexResult<'a> {
        self.take_while(|b| !b.is_ascii_whitespace())
    }

    #[inline]
    fn take_until_byte_consume(&mut self, c: u8) -> LexResult<'a> {
        self.take_while_consume(|b| b != c)
    }

    #[inline]
    fn take_until_byte(&mut self, c: u8) -> LexResult<'a> {
        self.take_while(|b| b != c)
    }

    #[inline]
    fn take_while_consume<F>(&mut self, cond: F) -> LexResult<'a>
    where
        F: Fn(u8) -> bool,
    {
        loop {
            let c = self.next_byte()?;
            if !cond(c) {
                break;
            }
            self.push(c);
        }
        Ok(())
    }

    #[allow(dead_code)]
    #[inline]
    fn skip_while_consume<F>(&mut self, cond: F) -> LexResult<'a>
    where
        F: Fn(u8) -> bool,
    {
        loop {
            let c = self.next_byte()?;
            if !cond(c) {
                break;
            }
        }
        Ok(())
    }

    #[inline]
    fn skip_while<F>(&mut self, cond: F) -> LexResult<'a>
    where
        F: Fn(u8) -> bool,
    {
        loop {
            let c = self.next_byte()?;
            if !cond(c) {
                self.set_peeked(c);
                break;
            }
        }
        Ok(())
    }

    #[inline]
    fn take_while<F>(&mut self, cond: F) -> LexResult<'a>
    where
        F: Fn(u8) -> bool,
    {
        loop {
            let c = self.next_byte()?;
            if !cond(c) {
                self.set_peeked(c);
                break;
            }
            self.push(c);
        }
        Ok(())
    }

    #[inline]
    fn set_peeked(&mut self, b: u8) {
        self.peeked = b;
    }

    #[inline]
    fn consume_peek(&mut self) {
        self.peeked = 0;
    }

    #[inline]
    fn peek(&mut self) -> Result<u8, LexError<'a>> {
        if self.peeked == 0 {
            let b = self.next_byte()?;
            self.peeked = b;
        }
        Ok(self.peeked)
    }

    fn skip_n(&mut self, n: usize) -> LexResult<'a> {
        for _ in 0..n {
            self.next_byte()?;
        }
        Ok(())
    }

    fn next_byte(&mut self) -> Result<u8, LexError<'a>> {
        if self.peeked != 0 {
            let b = self.peeked;
            self.peeked = 0;
            return Ok(b);
        }
        let res = self.bytes.next().ok_or(LexError::EOF)?;
        match res {
            Err(e) => Err(LexError::IO(e.kind())),
            Ok(c) => {
                self.offset += 1;
                Ok(c)
            }
        }
    }

    #[inline]
    fn discard_line(&mut self) {
        while let Ok(b) = self.next_byte() {
            if b == b'\n' {
                break;
            }
        }
    }

    #[inline]
    fn next_byte_nowhitespace(&mut self) -> Result<u8, LexError<'a>> {
        loop {
            let b = self.next_byte()?;
            if !b.is_ascii_whitespace() {
                return Ok(b);
            }
        }
    }

    fn next_byte_no_eof(&mut self) -> Result<u8, LexError<'a>> {
        self.next_byte().map_err(|e| match e {
            LexError::EOF => LexError::UnexpectedEOF,
            _ => e,
        })
    }

    #[inline]
    fn push(&mut self, b: u8) {
        self.scratch.push(b)
    }

    #[inline]
    fn push_all(&mut self, bytes: &[u8]) {
        self.scratch.extend_from_slice(bytes)
    }

    #[inline]
    fn clear_buf(&mut self) {
        self.scratch.clear()
    }

    #[inline]
    fn check_str<T, F>(&self, f: F) -> T
    where
        F: FnOnce(&str) -> T,
    {
        f(&String::from_utf8_lossy(&self.scratch))
    }

    /// Moves the token being lexed into the arena. Input smali is expected to be
    /// UTF-8 and invalid characters will be mapped to the replacement.
    #[inline]
    fn take_str(&mut self) -> &'a str {
        let taken = self
            .arena
            .alloc_str(&String::from_utf8_lossy(&self.scratch));
        self.scratch.clear();
        taken
    }

    fn quoted_str(&mut self, into: &mut Token<'a>) -> LexResult<'a> {
        let start = self.offset;
        loop {
            let c = self.next_byte()?;
            match c {
                b'\\' => {
                    let c2 = self.next_byte_no_eof()?;
                    match c2 {
                        // For some reason ' is also escaped in double quoted
                        // strings.
                        b'"' | b'\'' => self.push(c2),
                        // These are all of the valid escapes in smali strings
                        b'b' | b't' | b'n' | b'f' | b'r' | b'u' | b'\\' => {
                            self.push(c);
                            self.push(c2);
                        }
                        _ => return Err(LexError::BadStringEscape(c2 as char, self.offset)),
                    }
                }
                b'\n' => return Err(LexError::UnterminatedString(start)),
                b'"' => break,
                _ => self.push(c),
            }
        }
        let s = self.take_str();
        *into = Token::StringLiteral(s);
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use super::*;

    macro_rules! reg {
        ($r:ident) => {
            Register::parse(stringify!($r)).expect(concat!("bad register: ", stringify!($r)))
        };
    }

    macro_rules! next_token {
        ($lex:ident, $expected_token:expr) => {{
            let mut into = Token::Unknown;
            let res = $lex.lex(&mut into);
            assert_eq!(
                res.is_err(),
                false,
                "expected no error (and token {:?}) got {:?}: {}",
                $expected_token,
                res.as_ref(),
                res.as_ref().err().unwrap()
            );
            let exp = $expected_token;
            assert_eq!(into, exp, "expected type {:?} but got {:?}", exp, into);
        }};
    }

    #[test]
    fn lex_string_lit() {
        let arena = Arena::new();
        let b = "\"\"".as_bytes();
        let mut lex = Lexer::new(b, &arena);
        next_token!(lex, Token::StringLiteral(""));
        let b = "\"simplest\"".as_bytes();
        let mut lex = Lexer::new(b, &arena);
        next_token!(lex, Token::StringLiteral("simplest"));

        let b = r#""\'escaping\"\t\n\u{0000}\b\r\\\"""#.as_bytes();
        let mut lex = Lexer::new(b, &arena);
        let e = r#"'escaping"\t\n\u{0000}\b\r\\""#;
        next_token!(lex, Token::StringLiteral(e));
    }

    #[test]
    fn lex_ignores_comments() {
        let arena = Arena::new();
        let b = "# Such comment\n\"string\"".as_bytes();
        let mut lex = Lexer::new(b, &arena);
        next_token!(lex, Token::StringLiteral("string"));
    }

    #[test]
    fn lex_ignores_whitespace() {
        let arena = Arena::new();
        let b = "\t\r\n\t           \"string\"".as_bytes();
        let mut lex = Lexer::new(b, &arena);
        next_token!(lex, Token::StringLiteral("string"));
    }

    #[test]
    fn lex_bad_directive() {
        let arena = Arena::new();
        let b = ".cdoesntexist\n".as_bytes();
        let mut into = Token::Unknown;
        let mut lex = Lexer::new(b, &arena);
        let res = lex.lex(&mut into);
        assert!(res.is_err(), "expected error got {:?}", res);
        let e = res.err().unwrap();
        match e {
            LexError::UnknownDirective(d, _) => {
                assert_eq!(d, "cdoesntexist");
            }
            _ => panic!("expected UnknownDirective but got {:?}", e),
        }
    }

    #[test]
    fn lex_good_directive() {
        let arena = Arena::new();
        let d = ".catch\n".as_bytes();
        let mut lex = Lexer::new(d, &arena);
        next_token!(lex, Token::Directive(Directive::Catch));

        let d = ".catchall\n".as_bytes();
        let mut lex = Lexer::new(d, &arena);
        next_token!(lex, Token::Directive(Directive::CatchAll));
    }

    #[test]
    fn lex_invoke_method() {
        let arena = Arena::new();
        let d = "invoke-virtual {v2}, La/b/cd;->method()V\n".as_bytes();
        let mut lex = Lexer::new(d, &arena);
        next_token!(lex, Token::Instruction(INS_INVOKE_VIRTUAL));
        next_token!(lex, Token::OpenBrace);
        next_token!(lex, Token::Register(reg!(v2)));
        next_token!(lex, Token::CloseBrace);
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::ClassDescriptor("La/b/cd;"));
        next_token!(lex, Token::Arrow);
        next_token!(lex, Token::SimpleName("method"));
        next_token!(lex, Token::MethodArgs(""));
        next_token!(lex, Token::PrimitiveType(Primitive::Void));
    }

    #[test]
    fn lex_types() {
        let arena = Arena::new();
        macro_rules! lex_primitive {
            ($c:literal, $ty:ident) => {{
                let mut lex = Lexer::new($c.as_bytes(), &arena);
                let mut token = Token::Unknown;
                let res = lex.lex(&mut token);
                assert!(res.is_ok());
                assert_eq!(token, Token::PrimitiveType(Primitive::$ty));
            }};
        }
        lex_primitive!("J", Long);
        lex_primitive!("Z", Bool);
        lex_primitive!("S", Short);
        lex_primitive!("I", Int);
        lex_primitive!("B", Byte);
        lex_primitive!("F", Float);
        lex_primitive!("C", Char);
        lex_primitive!("V", Void);

        let d = "Ljava/lang/String;";
        let mut lex = Lexer::new(d.as_bytes(), &arena);
        next_token!(lex, Token::ClassDescriptor(d));
    }

    #[test]
    fn lex_field() {
        let arena = Arena::new();
        let hdr = ".field public static `tick field`:La/b/c/d/E;\n".as_bytes();
        let mut lex = Lexer::new(hdr, &arena);
        next_token!(lex, Token::Directive(Directive::Field));
        next_token!(lex, Token::AccessSpec(AccessFlag::PUBLIC));
        next_token!(lex, Token::AccessSpec(AccessFlag::STATIC));
        next_token!(lex, Token::SimpleName("tick field"));
        next_token!(lex, Token::Colon);
        next_token!(lex, Token::ClassDescriptor("La/b/c/d/E;"));
    }

    #[test]
    fn lex_field_with_value() {
        let arena = Arena::new();
        let hdr = ".field public static final FIELD:J = -0x10L\n".as_bytes();
        let mut lex = Lexer::new(hdr, &arena);
        next_token!(lex, Token::Directive(Directive::Field));
        next_token!(lex, Token::AccessSpec(AccessFlag::PUBLIC));
        next_token!(lex, Token::AccessSpec(AccessFlag::STATIC));
        next_token!(lex, Token::AccessSpec(AccessFlag::FINAL));
        next_token!(lex, Token::SimpleName("FIELD"));
        next_token!(lex, Token::Colon);
        next_token!(lex, Token::PrimitiveType(Primitive::Long));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::NumericLiteral("-0x10L"));
    }

    #[test]
    fn lex_last_field() {
        let arena = Arena::new();
        let smali = ".field static final blacklist TRANSACTION_unregisterPredictionUpdates:I = 0x6\n\n# direct methods\n".as_bytes();
        let mut lex = Lexer::new(smali, &arena);
        next_token!(lex, Token::Directive(Directive::Field));
        next_token!(lex, Token::AccessSpec(AccessFlag::STATIC));
        next_token!(lex, Token::AccessSpec(AccessFlag::FINAL));
        next_token!(lex, Token::AccessSpec(AccessFlag::BLACKLIST));
        next_token!(
            lex,
            Token::SimpleName("TRANSACTION_unregisterPredictionUpdates")
        );
        next_token!(lex, Token::Colon);
        next_token!(lex, Token::PrimitiveType(Primitive::Int));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::NumericLiteral("0x6"));
    }

    #[test]
    fn lex_method_header() {
        let arena = Arena::new();
        let hdr = ".method public static final L(JZCLjava/lang/String;[Z)V\n".as_bytes();
        let mut lex = Lexer::new(hdr, &arena);
        next_token!(lex, Token::Directive(Directive::Method));
        next_token!(lex, Token::AccessSpec(AccessFlag::PUBLIC));
        next_token!(lex, Token::AccessSpec(AccessFlag::STATIC));
        next_token!(lex, Token::AccessSpec(AccessFlag::FINAL));
        next_token!(lex, Token::SimpleName("L"));
        //next_token!(lex, Token::OpenParen);
        //next_token!(lex, Token::PrimitiveType(Primitive::Long));
        //next_token!(lex, Token::PrimitiveType(Primitive::Bool));
        //next_token!(lex, Token::PrimitiveType(Primitive::Char));
        //next_token!(lex, Token::ClassDescriptor("Ljava/lang/String;"));
        //next_token!(lex, Token::ArrayTypePrefix);
        //next_token!(lex, Token::PrimitiveType(Primitive::Bool));
        //next_token!(lex, Token::CloseParen);
        next_token!(lex, Token::MethodArgs("JZCLjava/lang/String;[Z"));
        next_token!(lex, Token::PrimitiveType(Primitive::Void));

        let hdr = ".method constructor <init>()V\n".as_bytes();
        let mut lex = Lexer::new(hdr, &arena);
        next_token!(lex, Token::Directive(Directive::Method));
        next_token!(lex, Token::AccessSpec(AccessFlag::CONSTRUCTOR));
        next_token!(lex, Token::SimpleName("<init>"));
        //next_token!(lex, Token::OpenParen);
        //next_token!(lex, Token::CloseParen);
        next_token!(lex, Token::MethodArgs(""));
        next_token!(lex, Token::PrimitiveType(Primitive::Void));

        let hdr = ".method constructor `a method`()V\n".as_bytes();
        let mut lex = Lexer::new(hdr, &arena);
        next_token!(lex, Token::Directive(Directive::Method));
        next_token!(lex, Token::AccessSpec(AccessFlag::CONSTRUCTOR));
        next_token!(lex, Token::SimpleName("a method"));
        //next_token!(lex, Token::OpenParen);
        //next_token!(lex, Token::CloseParen);
        next_token!(lex, Token::MethodArgs(""));
        next_token!(lex, Token::PrimitiveType(Primitive::Void));
    }

    #[test]
    fn lex_char() {
        let arena = Arena::new();
        let v = "'b'";
        let mut lex = Lexer::new(v.as_bytes(), &arena);
        next_token!(lex, Token::CharLiteral("b"));
        let v = "'\\u2764'";
        let mut lex = Lexer::new(v.as_bytes(), &arena);
        next_token!(lex, Token::CharLiteral("\\u2764"));
    }

    #[test]
    fn lex_numeric() {
        let arena = Arena::new();
        let num = "-10\n";
        let mut lex = Lexer::new(num.as_bytes(), &arena);
        next_token!(lex, Token::NumericLiteral("-10"));

        let num = "10\n";
        let mut lex = Lexer::new(num.as_bytes(), &arena);
        next_token!(lex, Token::NumericLiteral("10"));

        let num = "0xf\n";
        let mut lex = Lexer::new(num.as_bytes(), &arena);
        next_token!(lex, Token::NumericLiteral("0xf"));

        let num = "-0xf\n";
        let mut lex = Lexer::new(num.as_bytes(), &arena);
        next_token!(lex, Token::NumericLiteral("-0xf"));

        let num = "-10t\n";
        let mut lex = Lexer::new(num.as_bytes(), &arena);
        next_token!(lex, Token::NumericLiteral("-10t"));

        let num = "10t\n";
        let mut lex = Lexer::new(num.as_bytes(), &arena);
        next_token!(lex, Token::NumericLiteral("10t"));

        let num = "-0xft\n";
        let mut lex = Lexer::new(num.as_bytes(), &arena);
        next_token!(lex, Token::NumericLiteral("-0xft"));

        let num = "0xft\n";
        let mut lex = Lexer::new(num.as_bytes(), &arena);
        next_token!(lex, Token::NumericLiteral("0xft"));

        let num = "-1000000000L\n";
        let mut lex = Lexer::new(num.as_bytes(), &arena);
        next_token!(lex, Token::NumericLiteral("-1000000000L"));

        let num = "1000000000L\n";
        let mut lex = Lexer::new(num.as_bytes(), &arena);
        next_token!(lex, Token::NumericLiteral("1000000000L"));

        let num = "0x40000000    # 2.0f\n";
        let mut lex = Lexer::new(num.as_bytes(), &arena);
        next_token!(lex, Token::NumericLiteral("2.0f"));

        let num = "0x40800000    # -1.0f\n";
        let mut lex = Lexer::new(num.as_bytes(), &arena);
        next_token!(lex, Token::NumericLiteral("-1.0f"));

        let num = "0x40800000    # Float.NaN\n";
        let mut lex = Lexer::new(num.as_bytes(), &arena);
        next_token!(lex, Token::NumericLiteral("Float.NaN"));
    }

    #[test]
    fn lex_sswitch_entry() {
        let arena = Arena::new();
        let sswitch = r#".sparse-switch
        -0x7d14f855 -> :sswitch_3c7
        -0x304ed112 -> :sswitch_388
        0xa480416 -> :sswitch_37d
        0x6f08f706 -> :sswitch_2fd
    .end sparse-switch
"#;
        let mut lex = Lexer::new(sswitch.as_bytes(), &arena);
        macro_rules! sswitch_entry {
            ($num:literal, $label:literal) => {
                next_token!(lex, Token::NumericLiteral($num));
                next_token!(lex, Token::Arrow);
                next_token!(lex, Token::Colon);
                next_token!(lex, Token::SimpleName($label));
            };
        }
        next_token!(lex, Token::Directive(Directive::SparseSwitch));
        sswitch_entry!("-0x7d14f855", "sswitch_3c7");
        sswitch_entry!("-0x304ed112", "sswitch_388");
        sswitch_entry!("0xa480416", "sswitch_37d");
        sswitch_entry!("0x6f08f706", "sswitch_2fd");
        next_token!(lex, Token::Directive(Directive::EndSparseSwitch));
    }

    #[cfg(not(feature = "annotations"))]
    #[test]
    fn lex_skip_field_annotations() {
        let arena = Arena::new();
        let lines = r#"
# instance fields
.field private final blacklist mCache:Ljava/util/concurrent/ConcurrentHashMap;
    .annotation system Ldalvik/annotation/Signature;
        value = {
            "Ljava/util/concurrent/ConcurrentHashMap<",
            "Ljava/lang/String;",
            "Lcom/sec/android/iaft/SmLib_IafdSmAPIManager$Result;",
            ">;"
        }
    .end annotation
.end field
"#;
        let mut lex = Lexer::new(lines.as_bytes(), &arena);
        next_token!(lex, Token::Directive(Directive::Field));
        next_token!(lex, Token::AccessSpec(AccessFlag::PRIVATE));
        next_token!(lex, Token::AccessSpec(AccessFlag::FINAL));
        next_token!(lex, Token::AccessSpec(AccessFlag::BLACKLIST));
        next_token!(lex, Token::SimpleName("mCache"));
        next_token!(lex, Token::Colon);
        next_token!(
            lex,
            Token::ClassDescriptor("Ljava/util/concurrent/ConcurrentHashMap;")
        );
        next_token!(lex, Token::Directive(Directive::EndField));
    }

    #[cfg(not(feature = "annotations"))]
    #[test]
    fn lex_skips_annotations() {
        let lines = r#".method public m(Z)V
.registers 1
.annotation system Ldalvik/annotation/MemberClasses;
    value = {
        Landroid/app/slice/SliceItem$SliceType;
    }
.end annotation
.annotation system Ldalvik/annotation/MemberClasses;
    value = {
        Landroid/app/slice/SliceItem$SliceType;
    }
.end annotation
.param p1, "cool"
"#;
        let mut lex = Lexer::new(lines.as_bytes(), &arena);
        next_token!(lex, Token::Directive(Directive::Method));
        next_token!(lex, Token::AccessSpec(AccessFlag::PUBLIC));
        next_token!(lex, Token::SimpleName("m"));
        next_token!(lex, Token::MethodArgs("Z"));
        next_token!(lex, Token::PrimitiveType(Primitive::Void));
        next_token!(lex, Token::Directive(Directive::Param));
        next_token!(lex, Token::Register(Register::parse("p1").unwrap()));
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn lex_annotation_1() {
        let annotation = r#".annotation system Ldalvik/annotation/MemberClasses;
    value = {
        Landroid/app/slice/SliceItem$SliceType;
    }
.end annotation
"#;
        let arena = Arena::new();
        let mut lex = Lexer::new(annotation.as_bytes(), &arena);
        next_token!(lex, Token::Directive(Directive::Annotation));
        next_token!(
            lex,
            Token::AnnotationVisibility(AnnotationVisibility::System)
        );
        next_token!(
            lex,
            Token::ClassDescriptor("Ldalvik/annotation/MemberClasses;")
        );
        next_token!(lex, Token::SimpleName("value"));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::OpenBrace);
        next_token!(
            lex,
            Token::ClassDescriptor("Landroid/app/slice/SliceItem$SliceType;")
        );
        next_token!(lex, Token::CloseBrace);
        next_token!(lex, Token::Directive(Directive::EndAnnotation));
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn lex_annotation_2() {
        let arena = Arena::new();
        let annotation = r#".annotation system Ldalvik/annotation/AnnotationDefault;
    value = .subannotation Lorg/intellij/lang/annotations/MagicConstant;
        flags = {}
        flagsFromClass = V
        intValues = {}
        stringValues = {}
        valuesFromClass = V
    .end subannotation
.end annotation
"#;
        let mut lex = Lexer::new(annotation.as_bytes(), &arena);
        next_token!(lex, Token::Directive(Directive::Annotation));
        next_token!(
            lex,
            Token::AnnotationVisibility(AnnotationVisibility::System)
        );
        next_token!(
            lex,
            Token::ClassDescriptor("Ldalvik/annotation/AnnotationDefault;")
        );
        next_token!(lex, Token::SimpleName("value"));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::Directive(Directive::Subannotation));
        next_token!(
            lex,
            Token::ClassDescriptor("Lorg/intellij/lang/annotations/MagicConstant;")
        );
        next_token!(lex, Token::SimpleName("flags"));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::OpenBrace);
        next_token!(lex, Token::CloseBrace);
        next_token!(lex, Token::SimpleName("flagsFromClass"));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::PrimitiveType(Primitive::Void));

        next_token!(lex, Token::SimpleName("intValues"));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::OpenBrace);
        next_token!(lex, Token::CloseBrace);

        next_token!(lex, Token::SimpleName("stringValues"));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::OpenBrace);
        next_token!(lex, Token::CloseBrace);

        next_token!(lex, Token::SimpleName("valuesFromClass"));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::PrimitiveType(Primitive::Void));

        next_token!(lex, Token::Directive(Directive::EndSubannotation));
        next_token!(lex, Token::Directive(Directive::EndAnnotation));
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn lex_annotation_3() {
        let annotation = r#".annotation runtime Lkotlin/Metadata;
    d1 = {
        "\u0000*\n"
    }
    d2 = {
        "Lokio/Segment;",
        "",
        "()V",
        "data",
        "",
        "pos",
        "",
        "limit",
        "shared",
        "",
        "owner",
        "([BIIZZ)V",
        "next",
        "prev",
        "compact",
        "",
        "pop",
        "push",
        "segment",
        "sharedCopy",
        "split",
        "byteCount",
        "unsharedCopy",
        "writeTo",
        "sink",
        "Companion",
        "okio"
    }
    k = 0x1
    mv = {
        0x1,
        0x6,
        0x0
    }
    xi = 0x30
.end annotation
"#;

        let arena = Arena::new();
        let mut lex = Lexer::new(annotation.as_bytes(), &arena);
        next_token!(lex, Token::Directive(Directive::Annotation));
        next_token!(
            lex,
            Token::AnnotationVisibility(AnnotationVisibility::Runtime)
        );
        next_token!(lex, Token::ClassDescriptor("Lkotlin/Metadata;"));
        next_token!(lex, Token::SimpleName("d1"));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::OpenBrace);
        next_token!(lex, Token::StringLiteral("\\u0000*\\n"));
        next_token!(lex, Token::CloseBrace);
        next_token!(lex, Token::SimpleName("d2"));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::OpenBrace);
        next_token!(lex, Token::StringLiteral("Lokio/Segment;"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral(""));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("()V"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("data"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral(""));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("pos"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral(""));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("limit"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("shared"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral(""));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("owner"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("([BIIZZ)V"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("next"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("prev"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("compact"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral(""));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("pop"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("push"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("segment"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("sharedCopy"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("split"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("byteCount"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("unsharedCopy"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("writeTo"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("sink"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("Companion"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("okio"));
        next_token!(lex, Token::CloseBrace);
        next_token!(lex, Token::SimpleName("k"));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::NumericLiteral("0x1"));
        next_token!(lex, Token::SimpleName("mv"));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::OpenBrace);
        next_token!(lex, Token::NumericLiteral("0x1"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::NumericLiteral("0x6"));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::NumericLiteral("0x0"));
        next_token!(lex, Token::CloseBrace);
        next_token!(lex, Token::SimpleName("xi"));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::NumericLiteral("0x30"));
        next_token!(lex, Token::Directive(Directive::EndAnnotation));
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn lex_annotation_4() {
        let annotation = r#".annotation runtime Lcom/android/systemui/plugins/annotations/Dependencies;
    value = {
        .subannotation Lcom/android/systemui/plugins/annotations/DependsOn;
            target = Lcom/android/systemui/plugins/GlobalActionsPanelPlugin$Callbacks;
        .end subannotation,
        .subannotation Lcom/android/systemui/plugins/annotations/DependsOn;
            target = Lcom/android/systemui/plugins/GlobalActionsPanelPlugin$PanelViewController;
        .end subannotation
    }
    .end annotation
    "#;

        let arena = Arena::new();
        let mut lex = Lexer::new(annotation.as_bytes(), &arena);
        next_token!(lex, Token::Directive(Directive::Annotation));
        next_token!(
            lex,
            Token::AnnotationVisibility(AnnotationVisibility::Runtime)
        );
        next_token!(
            lex,
            Token::ClassDescriptor("Lcom/android/systemui/plugins/annotations/Dependencies;")
        );
        next_token!(lex, Token::SimpleName("value"));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::OpenBrace);
        next_token!(lex, Token::Directive(Directive::Subannotation));
        next_token!(
            lex,
            Token::ClassDescriptor("Lcom/android/systemui/plugins/annotations/DependsOn;")
        );
        next_token!(lex, Token::SimpleName("target"));
        next_token!(lex, Token::Equal);
        next_token!(
            lex,
            Token::ClassDescriptor(
                "Lcom/android/systemui/plugins/GlobalActionsPanelPlugin$Callbacks;"
            )
        );
        next_token!(lex, Token::Directive(Directive::EndSubannotation));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::Directive(Directive::Subannotation));
        next_token!(
            lex,
            Token::ClassDescriptor("Lcom/android/systemui/plugins/annotations/DependsOn;")
        );
        next_token!(lex, Token::SimpleName("target"));
        next_token!(lex, Token::Equal);
        next_token!(
            lex,
            Token::ClassDescriptor(
                "Lcom/android/systemui/plugins/GlobalActionsPanelPlugin$PanelViewController;"
            )
        );
        next_token!(lex, Token::Directive(Directive::EndSubannotation));
        next_token!(lex, Token::CloseBrace);
        next_token!(lex, Token::Directive(Directive::EndAnnotation));
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn lex_annotation_5() {
        let arena = Arena::new();
        let annotation = r#".annotation system Ldalvik/annotation/EnclosingMethod;
        value = Lokhttp3/OkHttpClient$Builder;->-addInterceptor(Lkotlin/jvm/functions/Function1;)Lokhttp3/OkHttpClient$Builder;
.end annotation
"#;
        let mut lex = Lexer::new(annotation.as_bytes(), &arena);
        next_token!(lex, Token::Directive(Directive::Annotation));
        next_token!(
            lex,
            Token::AnnotationVisibility(AnnotationVisibility::System)
        );
        next_token!(
            lex,
            Token::ClassDescriptor("Ldalvik/annotation/EnclosingMethod;")
        );
        next_token!(lex, Token::SimpleName("value"));
        next_token!(lex, Token::Equal);
        next_token!(
            lex,
            Token::ClassDescriptor("Lokhttp3/OkHttpClient$Builder;")
        );
        next_token!(lex, Token::Arrow);
        next_token!(lex, Token::SimpleName("-addInterceptor"));
        next_token!(lex, Token::MethodArgs("Lkotlin/jvm/functions/Function1;"));
        next_token!(
            lex,
            Token::ClassDescriptor("Lokhttp3/OkHttpClient$Builder;")
        );
        next_token!(lex, Token::Directive(Directive::EndAnnotation));
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn lex_annotation_6() {
        let arena = Arena::new();
        let annotation = r#".annotation runtime Lcom/oracle/svm/core/annotate/RecomputeFieldValue;
        declClass = [B
    .end annotation
"#;
        let mut lex = Lexer::new(annotation.as_bytes(), &arena);
        next_token!(lex, Token::Directive(Directive::Annotation));
        next_token!(
            lex,
            Token::AnnotationVisibility(AnnotationVisibility::Runtime)
        );
        next_token!(
            lex,
            Token::ClassDescriptor("Lcom/oracle/svm/core/annotate/RecomputeFieldValue;")
        );
        next_token!(lex, Token::SimpleName("declClass"));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::ArrayTypePrefix);
        next_token!(lex, Token::PrimitiveType(Primitive::Byte));
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn lex_annotation_7() {
        let annotation = r#".annotation system Ldalvik/annotation/Record;
    componentAnnotationVisibilities = {
        {}
    }
.end annotation"#;
        let arena = Arena::new();
        let mut lex = Lexer::new(annotation.as_bytes(), &arena);
        next_token!(lex, Token::Directive(Directive::Annotation));
        next_token!(
            lex,
            Token::AnnotationVisibility(AnnotationVisibility::System)
        );
        next_token!(lex, Token::ClassDescriptor("Ldalvik/annotation/Record;"));
        next_token!(lex, Token::SimpleName("componentAnnotationVisibilities"));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::OpenBrace);
        next_token!(lex, Token::OpenBrace);
        next_token!(lex, Token::CloseBrace);
        next_token!(lex, Token::CloseBrace);
    }

    #[test]
    fn lex_method_name_starts_with_neg() {
        let arena = Arena::new();
        let method = r#".method static synthetic -access$100(La/b/C;)[[Ljava/lang/String;
"#;
        let mut lex = Lexer::new(method.as_bytes(), &arena);
        next_token!(lex, Token::Directive(Directive::Method));
        next_token!(lex, Token::AccessSpec(AccessFlag::STATIC));
        next_token!(lex, Token::AccessSpec(AccessFlag::SYNTHETIC));
        next_token!(lex, Token::SimpleName("-access$100"));
        next_token!(lex, Token::MethodArgs("La/b/C;"));
        next_token!(lex, Token::ArrayTypePrefix);
        next_token!(lex, Token::ArrayTypePrefix);
        next_token!(lex, Token::ClassDescriptor("Ljava/lang/String;"));
    }

    #[test]
    fn test_lex_escaped_char() {
        let arena = Arena::new();
        let raw = r#".field static final APOSTROPHE:C = '\''

.field static final BACKSLASH:C = '\\'
"#;
        let mut lex = Lexer::new(raw.as_bytes(), &arena);
        next_token!(lex, Token::Directive(Directive::Field));
        next_token!(lex, Token::AccessSpec(AccessFlag::STATIC));
        next_token!(lex, Token::AccessSpec(AccessFlag::FINAL));
        next_token!(lex, Token::SimpleName("APOSTROPHE"));
        next_token!(lex, Token::Colon);
        next_token!(lex, Token::PrimitiveType(Primitive::Char));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::CharLiteral("'"));

        next_token!(lex, Token::Directive(Directive::Field));
        next_token!(lex, Token::AccessSpec(AccessFlag::STATIC));
        next_token!(lex, Token::AccessSpec(AccessFlag::FINAL));
        next_token!(lex, Token::SimpleName("BACKSLASH"));
        next_token!(lex, Token::Colon);
        next_token!(lex, Token::PrimitiveType(Primitive::Char));
        next_token!(lex, Token::Equal);
        next_token!(lex, Token::CharLiteral("\\"));
    }

    #[test]
    fn lex_method() {
        let arena = Arena::new();
        let method = r#".method static synthetic access$100(La/b/C;)[[Ljava/lang/String;
        .registers 1

    sget-object p0, La/b/C;->FIELD:[[Ljava/lang/String;

    return-object p0
.end method"#;

        let mut lex = Lexer::new(method.as_bytes(), &arena);
        next_token!(lex, Token::Directive(Directive::Method));
        next_token!(lex, Token::AccessSpec(AccessFlag::STATIC));
        next_token!(lex, Token::AccessSpec(AccessFlag::SYNTHETIC));
        next_token!(lex, Token::SimpleName("access$100"));
        next_token!(lex, Token::MethodArgs("La/b/C;"));
        next_token!(lex, Token::ArrayTypePrefix);
        next_token!(lex, Token::ArrayTypePrefix);
        next_token!(lex, Token::ClassDescriptor("Ljava/lang/String;"));
        next_token!(lex, Token::Instruction(INS_SGET_OBJECT));
        next_token!(lex, Token::Register(Register::parse("p0").unwrap()));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::ClassDescriptor("La/b/C;"));
        next_token!(lex, Token::Arrow);
        next_token!(lex, Token::SimpleName("FIELD"));
        next_token!(lex, Token::Colon);
        next_token!(lex, Token::ArrayTypePrefix);
        next_token!(lex, Token::ArrayTypePrefix);
        next_token!(lex, Token::ClassDescriptor("Ljava/lang/String;"));
        next_token!(lex, Token::Instruction(INS_RETURN_OBJECT));
        next_token!(lex, Token::Register(Register::parse("p0").unwrap()));
        next_token!(lex, Token::Directive(Directive::EndMethod));

        let method = r#".method final `tick method`(IZ)V
        .registers 2
        .param p1, "number"
        .param p2, "boolean"

        invoke-virtual/range { p1 .. p2 }, La/b/C;->`tick function`(IZ)V

        return-void

.end method"#;

        let mut lex = Lexer::new(method.as_bytes(), &arena);
        next_token!(lex, Token::Directive(Directive::Method));
        next_token!(lex, Token::AccessSpec(AccessFlag::FINAL));
        next_token!(lex, Token::SimpleName("tick method"));
        next_token!(lex, Token::MethodArgs("IZ"));
        next_token!(lex, Token::PrimitiveType(Primitive::Void));
        next_token!(lex, Token::Directive(Directive::Param));
        next_token!(lex, Token::Register(Register::parse("p1").unwrap()));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("number"));
        next_token!(lex, Token::Directive(Directive::Param));
        next_token!(lex, Token::Register(Register::parse("p2").unwrap()));
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::StringLiteral("boolean"));
        next_token!(lex, Token::Instruction(INS_INVOKE_VIRTUAL_RANGE));
        next_token!(lex, Token::OpenBrace);
        next_token!(lex, Token::Register(Register::parse("p1").unwrap()));
        next_token!(lex, Token::DotDot);
        next_token!(lex, Token::Register(Register::parse("p2").unwrap()));
        next_token!(lex, Token::CloseBrace);
        next_token!(lex, Token::Comma);
        next_token!(lex, Token::ClassDescriptor("La/b/C;"));
        next_token!(lex, Token::Arrow);
        next_token!(lex, Token::SimpleName("tick function"));
        next_token!(lex, Token::MethodArgs("IZ"));
        next_token!(lex, Token::PrimitiveType(Primitive::Void));
        next_token!(lex, Token::Instruction(INS_RETURN_VOID));
        next_token!(lex, Token::Directive(Directive::EndMethod));
    }
}

use crate::instructions::*;
include!(concat!(env!("OUT_DIR"), "/lex_gen.rs"));
