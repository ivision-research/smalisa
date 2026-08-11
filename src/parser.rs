use thiserror::Error;

use crate::class::ClassLineBuilder;
use crate::instructions::{InsBits, Instruction, InvArgs, Invocation};
use crate::method::MethodLineBuilder;
use crate::*;

pub type ParseResult<'a, T = ()> = Result<T, ParseError<'a>>;

/// LineParse is a trait for parsing a smali file by logical lines. These are
/// not always actual single lines, see the doc on the [Line] struct.
pub trait LineParse<'a> {
    fn parse_line(&mut self) -> ParseResult<'a, Line<'a>>;
    fn skip_to_next_method(&mut self) -> ParseResult<'a, Line<'a>>;
}

/// Takes any [LineParse] object and uses it to create a fully parsed [Class]. Note that this function
/// uses _unchecked variants of internal crate functions: you may get a panic on bad smali!
pub fn parse_class<'a, P: LineParse<'a>>(parser: &mut P) -> Result<Class<'a>, ParseError<'a>> {
    let mut builder = ClassLineBuilder::new();
    loop {
        match parser.parse_line() {
            Ok(line) => builder.push_line(line),
            Err(perr) if perr.is_eof() => return Ok(builder.finish()),
            Err(perr) => return Err(perr),
        }
    }
}

/// Parse a [Method] out of the [LineParse]
///
/// Note that this function will silently drop [Line]s until the first [Line::MethodHeader]
pub fn parse_method<'a, P: LineParse<'a>>(parser: &mut P) -> ParseResult<'a, Option<Method<'a>>> {
    loop {
        let line = match parser.parse_line() {
            Ok(line) => line,
            Err(perr) if perr.is_eof() => return Ok(None),
            Err(perr) => return Err(perr),
        };

        if let Line::MethodHeader(mh) = &line {
            let mut builder = MethodLineBuilder::new(mh);
            loop {
                let line = parser.parse_line()?;
                if matches!(line, Line::MethodEnd) {
                    return Ok(Some(builder.finish()));
                }
                builder.push_line(line);
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum ParseError<'a> {
    #[error("lexing: {0}")]
    Lex(LexError<'a>),
    #[error("unexpected token: {0:?} near offset {1}")]
    UnexpectedToken(Token<'a>, usize),
    #[error("expected token {0:?} but got token {1:?} near offset {2}")]
    WrongToken(Token<'a>, Token<'a>, usize),
    #[error("bad label: {0}")]
    BadLabel(&'a str),
    #[error("unsupported instruction: {0}")]
    UnsupportedInstruction(&'a str),
}

impl<'a> ParseError<'a> {
    pub fn is_eof(&self) -> bool {
        if let ParseError::Lex(lerr) = self {
            matches!(lerr, LexError::EOF)
        } else {
            false
        }
    }
}

impl<'a> From<LexError<'a>> for ParseError<'a> {
    fn from(le: LexError<'a>) -> ParseError<'a> {
        ParseError::Lex(le)
    }
}

pub struct Parser<'lex, L>
where
    L: Lex<'lex>,
{
    lexer: L,

    peeked: bool,
    token: Token<'lex>,

    cached_annotations: Vec<Annotation<'lex>>,
}

impl<'lex, L> Parser<'lex, L>
where
    L: Lex<'lex>,
{
    pub fn new(lexer: L) -> Self {
        Self {
            lexer,
            peeked: false,
            token: Token::Unknown,
            cached_annotations: Vec::new(),
        }
    }
}

impl<'lex, L> LineParse<'lex> for Parser<'lex, L>
where
    L: Lex<'lex>,
{
    fn parse_line(&mut self) -> ParseResult<'lex, Line<'lex>> {
        if let Some(ann) = self.cached_annotations.pop() {
            return Ok(Line::Annotation(ann));
        }
        self.lex_next()?;
        match self.token {
            Token::Directive(dir) => self.parse_directive(dir),
            Token::Instruction(ins) => self.parse_instruction(ins),
            Token::Colon => {
                // We do further processing on certain labels
                let label = self.parse_label()?;
                if label.is_array() {
                    self.lex_token_expect(Token::Directive(Directive::ArrayData))?;
                    self.parse_array_data(label)
                } else if label.is_sparse_switch_data() {
                    self.lex_token_expect(Token::Directive(Directive::SparseSwitch))?;
                    self.parse_sparse_switch(label)
                } else if label.is_packed_switch_data() {
                    self.lex_token_expect(Token::Directive(Directive::PackedSwitch))?;
                    self.parse_packed_switch(label)
                } else {
                    Ok(Line::LabelDefinition(label))
                }
            }
            _ => Err(ParseError::UnexpectedToken(
                self.token,
                self.lexer.get_offset(),
            )),
        }
    }

    fn skip_to_next_method(&mut self) -> ParseResult<'lex, Line<'lex>> {
        self.peek_next()?;
        if let Token::Directive(Directive::Method) = self.token {
            self.consume_peek();
            return self.parse_method_header();
        }
        self.lexer.skip_to_next_method()?;
        self.consume_peek();
        self.parse_method_header()
    }
}

macro_rules! parse_tok_return {
    ($fn:ident, $ty:ty, $tok_ty:ident) => {
        fn $fn(&mut self) -> ParseResult<'lex, $ty> {
            self.lex_next()?;
            if let Token::$tok_ty(val) = self.token {
                Ok(val)
            } else {
                unexpected_tok!(self);
            }
        }
    };
}

macro_rules! expect {
    ($self:ident, $($tok:tt)+) => {{
        $self.lex_token_expect(Token::$($tok)+)?;
    }};
}

macro_rules! unexpected_tok {
    ($self:ident) => {
        return Err(ParseError::UnexpectedToken(
            $self.token,
            $self.lexer.get_offset(),
        ))
    };
}

macro_rules! wrong_tok {
    ($self:ident, $expected:ident) => {
        return Err(ParseError::WrongToken(
            $self.token,
            Token::$expected,
            $self.lexer.get_offset(),
        ));
    };
}

impl<'lex, L> Parser<'lex, L>
where
    L: Lex<'lex>,
{
    #[inline]
    fn lex_next(&mut self) -> ParseResult<'lex> {
        if self.peeked {
            self.consume_peek();
        } else {
            self.lexer.lex(&mut self.token)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn peek_next(&mut self) -> ParseResult<'lex> {
        if !self.peeked {
            self.lexer.lex(&mut self.token)?;
            self.peeked = true;
        }
        Ok(())
    }

    #[inline]
    fn consume_peek(&mut self) {
        self.peeked = false;
    }

    fn parse_directive(&mut self, dir: Directive) -> ParseResult<'lex, Line<'lex>> {
        match dir {
            Directive::Class => {
                let mut acc = AccessFlag::UNSET;
                loop {
                    self.lex_next()?;
                    match self.token {
                        Token::AccessSpec(a) => {
                            acc |= a;
                        }
                        Token::ClassDescriptor(clazz) => {
                            acc.ensure_access();
                            return Ok(Line::Class(acc, clazz));
                        }
                        _ => unexpected_tok!(self),
                    }
                }
            }
            Directive::Super => {
                let name = self.parse_class_name()?;
                Ok(Line::Super(name))
            }
            Directive::Implements => {
                let name = self.parse_class_name()?;
                Ok(Line::Interface(name))
            }
            Directive::Catch => self.parse_catch(),
            Directive::CatchAll => self.parse_catch_all(),
            Directive::Param => self.parse_param(),
            Directive::Method => self.parse_method_header(),
            Directive::Field => self.parse_field_def(),
            Directive::Annotation => {
                let mut ann: Annotation = Default::default();
                self.parse_annotation_into(&mut ann)?;
                Ok(Line::Annotation(ann))
            }
            Directive::EndMethod => Ok(Line::MethodEnd),
            // See the comment over in RawArrayData::to_parsed. I didn't think this was valid
            // smali but I guess it is.
            Directive::ArrayData => self.parse_array_data(RawLabel::new("")),
            _ => unexpected_tok!(self),
        }
    }

    fn parse_array_data(&mut self, label: RawLabel<'lex>) -> ParseResult<'lex, Line<'lex>> {
        let mut data = RawArrayData {
            label,
            data_size: self.parse_numeric()?,
            data: Vec::new(),
        };
        loop {
            self.lex_next()?;
            match self.token {
                Token::Directive(Directive::EndArrayData) => {
                    return Ok(Line::ArrayData(data));
                }
                Token::NumericLiteral(num) => {
                    data.data.push(num);
                }
                _ => unexpected_tok!(self),
            }
        }
    }

    fn parse_catch(&mut self) -> ParseResult<'lex, Line<'lex>> {
        let mut catch = RawNamedCatch {
            class: self.parse_class_name()?,
            ..Default::default()
        };
        expect!(self, OpenBrace);
        expect!(self, Colon);
        catch.start_label = self.parse_label()?;
        expect!(self, DotDot);
        expect!(self, Colon);
        catch.end_label = self.parse_label()?;
        expect!(self, CloseBrace);
        expect!(self, Colon);
        catch.dest_label = self.parse_label()?;
        Ok(Line::NamedCatch(catch))
    }

    fn parse_sparse_switch(&mut self, label: RawLabel<'lex>) -> ParseResult<'lex, Line<'lex>> {
        let mut data = RawSparseSwitchData::new(label);
        loop {
            self.lex_next()?;
            match self.token {
                Token::Directive(Directive::EndSparseSwitch) => {
                    return Ok(Line::SparseSwitchData(data));
                }
                Token::NumericLiteral(num) => {
                    expect!(self, Arrow);
                    expect!(self, Colon);
                    let lab = self.expect_simple_name()?;
                    data.data.push(RawSwitchPair {
                        num,
                        label: RawLabel::new(lab.trim_start_matches(":")),
                    });
                }
                _ => unexpected_tok!(self),
            }
        }
    }

    fn parse_packed_switch(&mut self, label: RawLabel<'lex>) -> ParseResult<'lex, Line<'lex>> {
        let raw_num = self.parse_numeric()?;
        let mut data = RawPackedSwitchData::new(label, raw_num);
        loop {
            self.lex_next()?;
            match self.token {
                Token::Directive(Directive::EndPackedSwitch) => {
                    return Ok(Line::PackedSwitchData(data));
                }
                Token::Colon => {
                    let lab = self.expect_simple_name()?;
                    data.labels.push(RawLabel::new(lab));
                }
                _ => unexpected_tok!(self),
            }
        }
    }

    fn parse_catch_all(&mut self) -> ParseResult<'lex, Line<'lex>> {
        expect!(self, OpenBrace);
        expect!(self, Colon);

        let mut catch = RawCatchAll {
            start_label: self.parse_label()?,
            ..Default::default()
        };
        expect!(self, DotDot);
        expect!(self, Colon);
        catch.end_label = self.parse_label()?;
        expect!(self, CloseBrace);
        expect!(self, Colon);
        catch.dest_label = self.parse_label()?;
        Ok(Line::CatchAll(catch))
    }

    parse_tok_return!(expect_simple_name, &'lex str, SimpleName);
    parse_tok_return!(parse_class_name, &'lex str, ClassDescriptor);
    parse_tok_return!(parse_lit_str, &'lex str, StringLiteral);
    parse_tok_return!(parse_register, Register, Register);
    parse_tok_return!(parse_numeric, &'lex str, NumericLiteral);
    parse_tok_return!(expect_method_args, &'lex str, MethodArgs);

    fn parse_field_ref(&mut self, into: &mut FieldRef<'lex>) -> ParseResult<'lex> {
        into.class = self.parse_class_name()?;
        expect!(self, Arrow);
        into.name = self.expect_simple_name()?;
        expect!(self, Colon);
        into.ty = self.parse_type()?;
        Ok(())
    }

    fn parse_type(&mut self) -> ParseResult<'lex, Type<'lex>> {
        let mut dim = 0;
        loop {
            self.lex_next()?;
            match self.token {
                Token::ArrayTypePrefix => {
                    dim += 1;
                }
                Token::PrimitiveType(prim) => {
                    return Ok(Type::new_prim_array(prim, dim));
                }
                Token::ClassDescriptor(clazz) => {
                    return Ok(Type::new_class_array(clazz, dim));
                }
                _ => unexpected_tok!(self),
            }
        }
    }

    fn parse_field_def(&mut self) -> ParseResult<'lex, Line<'lex>> {
        let mut field: Field = Default::default();

        loop {
            self.lex_next()?;
            match self.token {
                Token::AccessSpec(access_spec) => {
                    field.access |= access_spec;
                }
                Token::SimpleName(name) => {
                    if field.access == AccessFlag::UNSET {
                        field.access = AccessFlag::PUBLIC;
                    }
                    field.name = name;
                    break;
                }
                _ => unexpected_tok!(self),
            }
        }
        expect!(self, Colon);
        field.ty = self.parse_type()?;
        // We're allowed to EOF with a field
        if let Err(ParseError::Lex(LexError::EOF)) = self.peek_next() {
            return Ok(Line::Field(field));
        }
        match self.token {
            // Parse the annotation(s) and apply it/them
            Token::Directive(Directive::Annotation) => loop {
                self.consume_peek();
                let mut ann: Annotation = Default::default();
                self.parse_annotation_into(&mut ann)?;
                field.annotations.push(ann);
                if let Err(ParseError::Lex(LexError::EOF)) = self.peek_next() {
                    return Ok(Line::Field(field));
                }
                if let Token::Directive(d) = self.token {
                    if d == Directive::EndField {
                        self.consume_peek();
                        return Ok(Line::Field(field));
                    } else if d != Directive::Annotation {
                        unexpected_tok!(self);
                    }
                }
            },
            Token::Equal => {
                self.consume_peek();
            }
            _ => {
                // If the annotations feature is disabled, we could end up with an EndField
                // directive here
                #[cfg(not(feature = "annotations"))]
                if let Token::Directive(Directive::EndField) = self.token {
                    self.consume_peek();
                }
                return Ok(Line::Field(field));
            }
        }
        self.lex_next()?;
        if let Some(raw) = RawLiteral::from_token(&self.token) {
            field.raw_value = raw;
        } else {
            unexpected_tok!(self);
        }
        // Can have an annotation here too
        // We're allowed to EOF with a field
        if let Err(ParseError::Lex(LexError::EOF)) = self.peek_next() {
            return Ok(Line::Field(field));
        }
        match self.token {
            // Parse the annotation(s) and apply it/them
            Token::Directive(Directive::Annotation) => loop {
                self.consume_peek();
                let mut ann: Annotation = Default::default();
                self.parse_annotation_into(&mut ann)?;
                field.annotations.push(ann);
                if let Err(ParseError::Lex(LexError::EOF)) = self.peek_next() {
                    return Ok(Line::Field(field));
                }
                if let Token::Directive(d) = self.token {
                    if d == Directive::EndField {
                        self.consume_peek();
                        return Ok(Line::Field(field));
                    } else if d != Directive::Annotation {
                        unexpected_tok!(self);
                    }
                }
            },
            Token::Directive(Directive::EndField) => {
                self.consume_peek();
            }
            _ => {
                // noop
            }
        }
        Ok(Line::Field(field))
    }
    fn parse_method_header(&mut self) -> ParseResult<'lex, Line<'lex>> {
        let mut hdr: MethodHeader = Default::default();
        loop {
            self.lex_next()?;
            match self.token {
                Token::AccessSpec(acc) => {
                    hdr.access |= acc;
                }
                Token::SimpleName(name) => {
                    hdr.name = name;
                    break;
                }
                _ => unexpected_tok!(self),
            }
        }
        hdr.args = self.expect_method_args()?;
        hdr.return_type = self.parse_type()?;
        // Parameter annotations?
        Ok(Line::MethodHeader(hdr))
    }

    fn parse_method_ref(&mut self, into: &mut MethodRef<'lex>) -> ParseResult<'lex> {
        into.class_array_dim = 0;
        // We can have any number of array prefixes here
        loop {
            self.lex_next()?;
            match self.token {
                Token::ArrayTypePrefix => into.class_array_dim += 1,
                Token::ClassDescriptor(cd) => {
                    into.class = cd;
                    break;
                }
                Token::PrimitiveType(prim) => {
                    into.class = prim.as_smali_str();
                    break;
                }
                _ => unexpected_tok!(self),
            }
        }
        //into.class = self.parse_class_name()?;
        expect!(self, Arrow);
        into.name = self.expect_simple_name()?;
        into.args = self.expect_method_args()?;
        into.return_type = self.parse_type()?;
        Ok(())
    }

    #[inline]
    fn parse_subannotation_into(&mut self, into: &mut Annotation<'lex>) -> ParseResult<'lex> {
        self.parse_sub_or_annotation_into(Directive::EndSubannotation, into)
    }

    #[inline]
    fn parse_annotation_into(&mut self, into: &mut Annotation<'lex>) -> ParseResult<'lex> {
        self.parse_sub_or_annotation_into(Directive::EndAnnotation, into)
    }

    fn parse_nested_annotation_list(&mut self) -> ParseResult<'lex, AnnotationValue<'lex>> {
        // We already ate the {

        let mut v: Vec<AnnotationValue> = Vec::new();

        loop {
            self.lex_next()?;
            match self.token {
                Token::OpenBrace => {
                    v.push(self.parse_nested_annotation_list()?);
                }
                Token::CloseBrace => {
                    return Ok(AnnotationValue::List(v));
                }
                Token::ClassDescriptor(cd) => {
                    v.push(AnnotationValue::Type(Type::new_class(cd)));
                }

                Token::Comma => continue,
                _ => {
                    if let Some(raw) = RawLiteral::from_token(&self.token) {
                        v.push(raw.into());
                    } else {
                        unexpected_tok!(self);
                    }
                }
            }
        }
    }

    fn parse_annotation_eq(
        &mut self,
        key: &'lex str,
        end: Directive,
        into: &mut Annotation<'lex>,
    ) -> ParseResult<'lex> {
        self.lex_next()?;
        match self.token {
            Token::OpenBrace => {
                let mut v: Vec<AnnotationValue> = Vec::new();
                loop {
                    self.lex_next()?;
                    match self.token {
                        Token::Directive(d) if d == end => return Ok(()),
                        // Nested lists
                        Token::OpenBrace => {
                            let value = self.parse_nested_annotation_list()?;
                            v.push(value);
                        }
                        Token::CloseBrace => {
                            into.insert(key, AnnotationValue::List(v));
                            break;
                        }
                        Token::Comma => continue,
                        Token::Directive(Directive::Enum) => {
                            let mut en: Enum = Default::default();
                            self.parse_enum(&mut en)?;
                            v.push(AnnotationValue::Enum(en));
                        }
                        Token::Directive(Directive::Subannotation) => {
                            let mut subann: Annotation = Default::default();
                            self.parse_subannotation_into(&mut subann)?;
                            v.push(AnnotationValue::Subannotation(subann));
                        }
                        Token::ClassDescriptor(cd) => {
                            v.push(AnnotationValue::Type(Type::new_class(cd)));
                        }
                        _ => {
                            if let Some(raw) = RawLiteral::from_token(&self.token) {
                                v.push(raw.into());
                            } else {
                                unexpected_tok!(self);
                            }
                        }
                    }
                }
            }
            Token::ArrayTypePrefix => {
                let mut dim: u8 = 1;
                loop {
                    self.lex_next()?;
                    match self.token {
                        Token::ArrayTypePrefix => dim += 1,
                        Token::ClassDescriptor(cd) => {
                            into.insert(key, AnnotationValue::Type(Type::new_class_array(cd, dim)));
                            break;
                        }
                        Token::PrimitiveType(p) => {
                            into.insert(key, AnnotationValue::Type(Type::new_prim_array(p, dim)));
                            break;
                        }
                        _ => {
                            unexpected_tok!(self);
                        }
                    }
                }
            }
            Token::Directive(Directive::Subannotation) => {
                let mut subann: Annotation = Default::default();
                self.parse_subannotation_into(&mut subann)?;
                into.insert(key, subann.into());
            }
            Token::Directive(Directive::Enum) => {
                let mut en: Enum = Default::default();
                self.parse_enum(&mut en)?;
                into.insert(key, en.into());
            }
            Token::ClassDescriptor(cd) => {
                self.peek_next()?;
                // Method TODO
                if let Token::Arrow = self.token {
                    self.consume_peek();
                    let name = self.expect_simple_name()?;
                    let args = self.expect_method_args()?;
                    let ret_type = self.parse_type()?;
                    into.insert(
                        key,
                        AnnotationValue::Method(MethodRef::new(cd, name, args, ret_type)),
                    );
                } else {
                    into.insert(key, AnnotationValue::Type(Type::new_class(cd)));
                }
            }
            Token::PrimitiveType(p) => {
                into.insert(key, p.into());
            }
            _ => {
                if let Some(raw) = RawLiteral::from_token(&self.token) {
                    into.insert(key, raw.into());
                } else {
                    unexpected_tok!(self);
                }
            }
        }
        Ok(())
    }

    fn parse_sub_or_annotation_into(
        &mut self,
        end: Directive,
        into: &mut Annotation<'lex>,
    ) -> ParseResult<'lex> {
        // First line should always give us the visibility and class for outer
        // annotations
        self.lex_next()?;
        if end == Directive::EndAnnotation {
            if let Token::AnnotationVisibility(v) = self.token {
                into.visibility = v;
            } else {
                unexpected_tok!(self);
            }
            self.lex_next()?;
        }
        if let Token::ClassDescriptor(c) = self.token {
            into.class = c;
        } else {
            unexpected_tok!(self);
        }
        loop {
            self.lex_next()?;
            match self.token {
                Token::Directive(d) => {
                    if d == end {
                        return Ok(());
                    } else {
                        unexpected_tok!(self);
                    }
                }
                Token::SimpleName(key) => {
                    expect!(self, Equal);
                    self.parse_annotation_eq(key, end, into)?;
                }
                _ => unexpected_tok!(self),
            }
        }
    }

    fn parse_param(&mut self) -> ParseResult<'lex, Line<'lex>> {
        let reg = self.parse_register()?;
        self.peek_next()?;
        // In the case where there is no comma we should be pretty sure below
        // that the annotations belong to the param
        let name = if let Token::Comma = self.token {
            self.consume_peek();
            self.parse_lit_str()?
        } else {
            ""
        };
        let mut annotations = Vec::new();
        // Fun fact about smali, there is no way to determine if the annotations
        // following a .param are intended for the method or the param without
        // reading until we find a .end param or something else. This is
        // honestly a bummer, but we have to deal with it so ok.
        loop {
            self.peek_next()?;
            if let Token::Directive(d) = self.token {
                match d {
                    Directive::Annotation => {
                        self.consume_peek();
                        let mut ann: Annotation = Default::default();
                        self.parse_annotation_into(&mut ann)?;
                        annotations.push(ann);
                    }
                    Directive::EndParam => {
                        self.consume_peek();
                        return Ok(if annotations.len() > 0 {
                            // They belonged to the param
                            Line::ParamLine(reg, name, Some(annotations))
                        } else {
                            Line::ParamLine(reg, name, None)
                        });
                    }
                    // Any other directive means that they actually belonged
                    // to the method
                    _ => {
                        self.cached_annotations = annotations;
                        self.cached_annotations.reverse();
                        return Ok(Line::ParamLine(reg, name, None));
                    }
                }
            } else {
                // Once again means they probably didn't belong to the param
                self.cached_annotations = annotations;
                self.cached_annotations.reverse();
                return Ok(Line::ParamLine(reg, name, None));
            }
        }
    }

    fn parse_enum(&mut self, into: &mut Enum<'lex>) -> ParseResult<'lex> {
        self.lex_next()?;
        if let Token::ClassDescriptor(c) = self.token {
            into.owner = c;
        } else {
            unexpected_tok!(self);
        }
        expect!(self, Arrow);
        self.lex_next()?;
        if let Token::SimpleName(c) = self.token {
            into.name = c;
        } else {
            unexpected_tok!(self);
        }
        expect!(self, Colon);
        self.lex_next()?;
        if let Token::ClassDescriptor(c) = self.token {
            into.ty = c;
        } else {
            unexpected_tok!(self);
        }
        Ok(())
    }

    fn parse_instruction(&mut self, ins: Instruction) -> ParseResult<'lex, Line<'lex>> {
        let args = match ins.fmt() {
            InsBits::CFMT_BARE => InvArgs::Bare,
            InsBits::CFMT_REG => InvArgs::OneReg(self.parse_register()?),
            InsBits::CFMT_REG_REG => {
                let reg = self.parse_register()?;
                expect!(self, Comma);
                InvArgs::TwoReg(reg, self.parse_register()?)
            }
            InsBits::CFMT_REG_REG_REG => {
                let reg = self.parse_register()?;
                expect!(self, Comma);
                let reg2 = self.parse_register()?;
                expect!(self, Comma);
                InvArgs::ThreeReg(reg, reg2, self.parse_register()?)
            }
            InsBits::CFMT_REG_REG_ARR => {
                let reg = self.parse_register()?;
                expect!(self, Comma);
                let reg2 = self.parse_register()?;
                expect!(self, Comma);
                let arr = self.parse_type()?;
                InvArgs::TwoRegArray(reg, reg2, arr)
            }
            InsBits::CFMT_ARGS_ARR => {
                let regs = self.parse_variable_registers()?;
                expect!(self, Comma);
                let arr = self.parse_type()?;
                InvArgs::VarRegArray(regs, arr)
            }
            InsBits::CFMT_ARGS_METHOD_POLYMORPHIC => {
                let regs = self.parse_variable_registers()?;
                expect!(self, Comma);
                let mut mref: MethodRef = Default::default();
                self.parse_method_ref(&mut mref)?;
                expect!(self, Comma);
                let margs = self.expect_method_args()?;
                let ty = self.parse_type()?;
                InvArgs::Polymorphic(regs, mref, margs, ty)
            }
            InsBits::CFMT_ARGS_METHOD_CUSTOM => {
                return Err(ParseError::UnsupportedInstruction(
                    "invoke-custom;invoke-custom/range",
                ));
            }
            InsBits::CFMT_ARGS_METHOD => {
                let regs = self.parse_variable_registers()?;
                expect!(self, Comma);
                let mut mref: MethodRef = Default::default();
                self.parse_method_ref(&mut mref)?;
                InvArgs::VarRegMethod(regs, mref)
            }
            InsBits::CFMT_LABEL => {
                expect!(self, Colon);
                InvArgs::Label(self.parse_label()?)
            }
            InsBits::CFMT_REG_LABEL => {
                let reg = self.parse_register()?;
                expect!(self, Comma);
                expect!(self, Colon);
                InvArgs::OneRegLabel(reg, self.parse_label()?)
            }
            InsBits::CFMT_REG_REG_LABEL => {
                let reg = self.parse_register()?;
                expect!(self, Comma);
                let regb = self.parse_register()?;
                expect!(self, Comma);
                expect!(self, Colon);
                InvArgs::TwoRegLabel(reg, regb, self.parse_label()?)
            }
            InsBits::CFMT_REG_STR => {
                let reg = self.parse_register()?;
                expect!(self, Comma);
                InvArgs::RegStr(reg, self.parse_lit_str()?)
            }
            InsBits::CFMT_REG_FIELD => {
                let reg = self.parse_register()?;
                expect!(self, Comma);
                let mut fref: FieldRef = Default::default();
                self.parse_field_ref(&mut fref)?;
                InvArgs::OneRegField(reg, fref)
            }
            InsBits::CFMT_REG_REG_FIELD => {
                let reg = self.parse_register()?;
                expect!(self, Comma);
                let reg2 = self.parse_register()?;
                expect!(self, Comma);
                let mut fref: FieldRef = Default::default();
                self.parse_field_ref(&mut fref)?;
                InvArgs::TwoRegField(reg, reg2, fref)
            }
            InsBits::CFMT_REG_CLASS => {
                let reg = self.parse_register()?;
                expect!(self, Comma);
                let ty = self.parse_type()?;
                InvArgs::OneRegClass(reg, ty)
            }
            InsBits::CFMT_REG_REG_CLASS => {
                let reg = self.parse_register()?;
                expect!(self, Comma);
                let reg2 = self.parse_register()?;
                expect!(self, Comma);
                let ty = self.parse_type()?;
                InvArgs::TwoRegClass(reg, reg2, ty)
            }
            InsBits::CFMT_REG_NUM => {
                let reg = self.parse_register()?;
                expect!(self, Comma);
                let num = self.parse_numeric()?;
                InvArgs::OneRegNum(reg, num)
            }
            InsBits::CFMT_REG_REG_NUM => {
                let reg = self.parse_register()?;
                expect!(self, Comma);
                let regb = self.parse_register()?;
                expect!(self, Comma);
                InvArgs::TwoRegNum(reg, regb, self.parse_numeric()?)
            }
            _ => todo!(),
        };
        Ok(Line::InstructionInvocation(Invocation::new(ins, args)))
    }

    fn parse_label(&mut self) -> ParseResult<'lex, RawLabel<'lex>> {
        self.lex_next()?;
        if let Token::SimpleName(ref name) = self.token {
            Ok(RawLabel::new(name))
        } else {
            unexpected_tok!(self);
        }
    }

    fn parse_variable_registers(&mut self) -> ParseResult<'lex, VarRegister> {
        expect!(self, OpenBrace);
        self.lex_next()?;
        let first = match self.token {
            Token::CloseBrace => {
                return Ok(VarRegister::Empty);
            }
            Token::Register(reg) => reg,
            _ => unexpected_tok!(self),
        };
        self.lex_next()?;
        let varreg = match self.token {
            Token::DotDot => {
                let last = self.parse_register()?;
                expect!(self, CloseBrace);
                VarRegister::Range(RegisterRange::new(first, last))
            }
            Token::CloseBrace => {
                let mut arr = RegisterArray::new_empty();
                arr.push(first);
                VarRegister::Array(arr)
            }
            Token::Comma => {
                let mut arr = RegisterArray::new_empty();
                arr.push(first);
                loop {
                    arr.push(self.parse_register()?);
                    self.lex_next()?;
                    if matches!(self.token, Token::CloseBrace) {
                        break;
                    } else if !matches!(self.token, Token::Comma) {
                        wrong_tok!(self, Comma);
                    }
                }
                VarRegister::Array(arr)
            }
            _ => unexpected_tok!(self),
        };
        Ok(varreg)
    }

    #[inline]
    fn lex_token_expect(&mut self, tok: Token<'lex>) -> ParseResult<'lex> {
        self.lex_next()?;
        if self.token == tok {
            Ok(())
        } else {
            Err(ParseError::WrongToken(
                tok,
                self.token,
                self.lexer.get_offset(),
            ))
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::instructions::*;
    use crate::lexer::Lexer;
    #[cfg(feature = "annotations")]
    use crate::AnnotationVisibility;
    use crate::Primitive;

    macro_rules! reg {
        ($r:ident) => {
            Register::parse(stringify!($r)).expect(concat!("bad register: ", stringify!($r)))
        };
    }

    macro_rules! reg_range {
        ($start:ident .. $end:ident) => {
            RegisterRange::new(reg!($start), reg!($end))
        };
    }

    macro_rules! reg_array {
        ($($reg:ident),+) => {{
            let mut __arr = RegisterArray::new_empty();
            $(
                __arr.push(reg!($reg));
                assert!(__arr.count() <= crate::register::MAX_VAR_REGISTERS);
            )*
            __arr
        }};
    }

    macro_rules! parse_line {
        ($s:literal, $fn:expr) => {{
            let arena = Arena::new();
            let smali_line = $s.as_bytes();
            let lexer = Lexer::new(smali_line, &arena);
            let mut parser = Parser::new(lexer);
            let res = parser.parse_line();
            assert!(res.is_ok(), "expected no error: {:?}", res);
            $fn(&res.unwrap());
        }};
    }

    macro_rules! assert_line {
        ($s:literal, $expected:expr) => {{
            parse_line!($s, |__line: &Line<'_>| {
                assert_eq!(*__line, $expected);
            });
        }};
    }

    macro_rules! assert_ins {
        ($s:literal, $ins:tt) => {{
            assert_line!($s, Line::InstructionInvocation(Invocation::new(
                    $ins, InvArgs::Bare)));
        }};

        ($s:literal, $ins:tt, $argt:tt, $($args:expr),+) => {{
            assert_line!($s, Line::InstructionInvocation(Invocation::new(
                    $ins, InvArgs::$argt($($args),*))));
        }};
    }

    #[test]
    fn parse_fmt_reg_str() {
        assert_ins!(
            "const-string v0, \"string\"",
            INS_CONST_STRING,
            RegStr,
            reg!(v0),
            "string"
        );
    }

    #[test]
    fn parse_fmt_reg_field() {
        assert_ins!(
            "sget-object v0, La/t/f/D;->sVal:Ljava/lang/Object;",
            INS_SGET_OBJECT,
            OneRegField,
            reg!(v0),
            FieldRef {
                class: "La/t/f/D;",
                name: "sVal",
                ty: Type::new_class("Ljava/lang/Object;"),
            }
        );

        assert_ins!(
            "sput-object v3, Laaa/bbb/Ccc;->ARR_FIELD:[[Ljava/lang/Object;",
            INS_SPUT_OBJECT,
            OneRegField,
            reg!(v3),
            FieldRef {
                class: "Laaa/bbb/Ccc;",
                name: "ARR_FIELD",
                ty: Type::new_class_array("Ljava/lang/Object;", 2),
            }
        );
    }

    #[test]
    fn parse_fmt_label() {
        assert_ins!("goto :goto_22\n", INS_GOTO, Label, RawLabel::new("goto_22"));
    }

    #[test]
    fn parse_fmt_reg() {
        assert_ins!(
            "move-result-object v0\n",
            INS_MOVE_RESULT_OBJECT,
            OneReg,
            reg!(v0)
        );
    }

    #[test]
    fn parse_fmt_reg_reg() {
        assert_ins!("move v0, v1\n", INS_MOVE, TwoReg, reg!(v0), reg!(v1));
    }

    #[test]
    fn parse_fmt_reg_label() {
        assert_ins!(
            "fill-array-data v2, :array_3f\n",
            INS_FILL_ARRAY_DATA,
            OneRegLabel,
            reg!(v2),
            RawLabel::new("array_3f")
        );
    }

    #[test]
    fn parse_fmt_reg_reg_label() {
        assert_ins!(
            "if-eq p0, v3, :cond_1f\n",
            INS_IF_EQ,
            TwoRegLabel,
            reg!(p0),
            reg!(v3),
            RawLabel::new("cond_1f")
        );
    }

    #[test]
    fn parse_fmt_reg_num() {
        assert_ins!("const p12, 0xef\n", INS_CONST, OneRegNum, reg!(p12), "0xef");
    }

    #[test]
    fn parse_fmt_bare() {
        assert_ins!("nop\n", INS_NOP);
        assert_ins!("return-void\n", INS_RETURN_VOID);
    }

    #[test]
    fn parse_fmt_reg_reg_arr() {
        assert_ins!(
            "new-array v4, p2, [[I",
            INS_NEW_ARRAY,
            TwoRegArray,
            reg!(v4),
            reg!(p2),
            Type::new_prim_array(Primitive::Int.into(), 2)
        );

        assert_ins!(
            "filled-new-array/range {v0 .. v5}, [Ljava/lang/String;",
            INS_FILLED_NEW_ARRAY_RANGE,
            VarRegArray,
            VarRegister::Range(reg_range!(v0..v5)),
            Type::new_class_array("Ljava/lang/String;", 1)
        );

        assert_ins!(
            "filled-new-array {v0, p2, v5}, [Ljava/lang/String;",
            INS_FILLED_NEW_ARRAY,
            VarRegArray,
            VarRegister::Array(reg_array!(v0, p2, v5)),
            Type::new_class_array("Ljava/lang/String;", 1)
        );
    }

    #[test]
    fn parse_fmt_args_method() {
        assert_ins!(
            "invoke-virtual {v0}, La/b/c;->Method()Z\n",
            INS_INVOKE_VIRTUAL,
            VarRegMethod,
            VarRegister::Array(reg_array!(v0)),
            MethodRef::new("La/b/c;", "Method", "", Primitive::Bool.into())
        );

        assert_ins!(
            "invoke-static {}, La/b/c;->Lmethod()C\n",
            INS_INVOKE_STATIC,
            VarRegMethod,
            VarRegister::Empty,
            MethodRef::new("La/b/c;", "Lmethod", "", Primitive::Char.into())
        );

        assert_ins!(
            "invoke-virtual {v0, v1, v2, p1, p3}, La/b/c;->Method(IZLa/b/d;J)V\n",
            INS_INVOKE_VIRTUAL,
            VarRegMethod,
            VarRegister::Array(reg_array!(v0, v1, v2, p1, p3)),
            MethodRef::new("La/b/c;", "Method", "IZLa/b/d;J", Primitive::Void.into())
        );

        assert_ins!(
            "invoke-virtual/range {p0 .. p5}, La/b/c;->Method(IZLa/b/d;J)Ljava/lang/String;\n",
            INS_INVOKE_VIRTUAL_RANGE,
            VarRegMethod,
            VarRegister::Range(reg_range!(p0..p5)),
            MethodRef::new(
                "La/b/c;",
                "Method",
                "IZLa/b/d;J",
                Type::new_class("Ljava/lang/String;"),
            )
        );
    }

    #[test]
    fn parse_fmt_reg_reg_num() {
        assert_ins!(
            "add-int/lit8 v15, p12, 12\n",
            INS_ADD_INT_LIT8,
            TwoRegNum,
            reg!(v15),
            reg!(p12),
            "12"
        );
    }

    #[test]
    fn parse_fmt_reg_reg_reg() {
        assert_ins!(
            "aget v0, v2, p1\n",
            INS_AGET,
            ThreeReg,
            reg!(v0),
            reg!(v2),
            reg!(p1)
        );
    }

    #[test]
    fn parse_label_only() {
        assert_line!(
            ":cond_123e\n",
            Line::LabelDefinition(RawLabel::new("cond_123e"))
        );
    }

    #[test]
    fn parse_super() {
        assert_line!(".super Lfoo/bar/B;\n", Line::Super("Lfoo/bar/B;"));
    }

    #[test]
    fn parse_class() {
        assert_line!(
            ".class Lfoo/bar/B;\n",
            Line::Class(AccessFlag::PUBLIC, "Lfoo/bar/B;")
        );
    }

    #[test]
    fn parse_implements() {
        assert_line!(".implements Lfoo/bar/B;\n", Line::Interface("Lfoo/bar/B;"));
    }

    #[cfg(feature = "annotations")]
    macro_rules! annotation {
        (
            class: $class:expr,
            vis: $vis:ident,
            params: {$($key:literal = $value:expr),*}
        ) => {{
            let mut __ann = Annotation::new($class, AnnotationVisibility::$vis);
            $(
                __ann.insert($key, $value.into());
            )*
            __ann
        }};
    }

    #[cfg(feature = "annotations")]
    macro_rules! assert_field {
        (
            $str:literal,
            access: $($acc:ident)|*,
            name: $name:literal,
            ty: $ty:expr,
            raw_value: $val:expr,
            annotations: [$($annotations:expr),*]
        ) => {
            assert_line!($str, Line::Field(Field::new(
                $name,
                $(AccessFlag::$acc)|*,
                $ty,
                $val,
                vec![$($annotations),*]
            )));
        };
    }

    #[cfg(not(feature = "annotations"))]
    macro_rules! assert_field {
        (
            $str:literal,
            access: $($acc:ident)|*,
            name: $name:literal,
            ty: $ty:expr,
            raw_value: $val:expr,
            annotations: [$($annotations:expr),*]
        ) => {
            assert_line!($str, Line::Field(Field::new(
                $name,
                $(AccessFlag::$acc)|*,
                $ty,
                $val,
                vec![]
            )));
        };
    }

    #[cfg(not(feature = "annotations"))]
    #[test]
    fn parse_field_skip_annotations() {
        assert_field!(r#".field private final blacklist mCache:Ljava/util/concurrent/ConcurrentHashMap;
    .annotation system Ldalvik/annotation/Signature;
        value = {
            "Ljava/util/concurrent/ConcurrentHashMap<",
            "Ljava/lang/String;",
            "Lcom/sec/android/iaft/SmLib_IafdSmAPIManager$Result;",
            ">;"
        }
    .end annotation
.end field
"#,
        access: PRIVATE | FINAL | BLACKLIST,
        name: "mCache",
        ty: Type::new_class("Ljava/util/concurrent/ConcurrentHashMap;"),
        raw_value: RawLiteral::Unset,
        annotations: []
        );

        assert_field!(r#".field private blacklist hashMapOfRepairDBInfo:Ljava/util/HashMap;
    .annotation system Ldalvik/annotation/Signature;
        value = {
            "Ljava/util/HashMap<",
            "Ljava/lang/String;",
            "[",
            "Ljava/lang/String;",
            ">;"
        }
    .end annotation
.end field
"#,
        access: PRIVATE | BLACKLIST,
        name: "hashMapOfRepairDBInfo",
        ty: Type::new_class("Ljava/util/HashMap;"),
        raw_value: RawLiteral::Unset,
        annotations: []
        );
    }

    #[test]
    fn parse_field_def() {
        assert_field!(
            ".field public static final greylist NAME:Ljava/lang/String; = \"g\"",
            access: PUBLIC | GREYLIST | STATIC | FINAL,
            name: "NAME",
            ty: Type::new_class("Ljava/lang/String;"),
            raw_value: RawLiteral::String("g"),
            annotations: []
        );

        assert_field!(
            r#"# static fields
        .field public static final greylist NAME:Ljava/lang/String; = "Path""#,
            access: PUBLIC | GREYLIST | STATIC | FINAL,
            name: "NAME",
            ty: Type::new_class("Ljava/lang/String;"),
            raw_value: RawLiteral::String("Path"),
            annotations: []
        );

        assert_field!(
            ".field private static final NAME:[Ljava/lang/String;",
            access: PRIVATE | STATIC | FINAL,
            name: "NAME",
            ty: Type::new_class_array("Ljava/lang/String;", 1),
            raw_value: RawLiteral::Unset,
            annotations: []
        );

        assert_field!(
            ".field NAME:Ljava/lang/String; = null\n",
            access: PUBLIC,
            name: "NAME",
            ty: Type::new_class("Ljava/lang/String;"),
            raw_value: RawLiteral::Null,
            annotations: [
            ]
        );

        assert_field!(
            ".field C:C = '\\u2764'\n",
            access: PUBLIC,
            name: "C",
            ty: Primitive::Char.into(),
            raw_value: RawLiteral::Char("\\u2764"),
            annotations: []
        );

        assert_field!(
            ".field clazz:La/b/c/d/e/F;\n",
            access: PUBLIC,
            name: "clazz",
            ty: Type::new_class("La/b/c/d/e/F;"),
            raw_value: RawLiteral::Unset,
            annotations: []
        );

        assert_field!(
            r#"
.field static protected clazz:La/b/c;
    .annotation runtime Ljava/lang/Deprecated;
    .end annotation
.end field"#,
            access: STATIC | PROTECTED,
            name: "clazz",
            ty: Type::new_class("La/b/c;"),
            raw_value: RawLiteral::Unset,
            annotations: [
                annotation!(
                    class: "Ljava/lang/Deprecated;",
                    vis: Runtime,
                    params: {}
                )
            ]
        );

        assert_field!(
            r#"
        .field static protected clazz:La/b/c;
            .annotation system LAnnotation;
                value = "wow"
                numeric = 0x23
                null = null
            .end annotation
        .end field"#,
            access: STATIC | PROTECTED,
            name: "clazz",
            ty: Type::new_class("La/b/c;"),
            raw_value: RawLiteral::Unset,
            annotations: [
                annotation!(
                    class: "LAnnotation;",
                    vis: System,
                    params: {
                        "value" = RawLiteral::String("wow"),
                        "numeric" = RawLiteral::Numeric("0x23"),
                        "null" = RawLiteral::Null
                    }
                )
            ]
        );

        assert_field!(
            r#"
        .field static protected clazz:La/b/c;
            .annotation system LAnnotation;
                value = .subannotation LSubann;
                    meta = "very meta"
                .end subannotation
                numeric = 0x23
                null = null
            .end annotation
        .end field"#,
            access: STATIC | PROTECTED,
            name: "clazz",
            ty: Type::new_class("La/b/c;"),
            raw_value: RawLiteral::Unset,
            annotations: [
                annotation!(
                    class: "LAnnotation;",
                    vis: System,
                    params: {
                        "value" = annotation!(
                            class: "LSubann;",
                            vis: Unset,
                            params: {
                                "meta" = RawLiteral::String("very meta")
                            }
                        ),
                        "numeric" = RawLiteral::Numeric("0x23"),
                        "null" = RawLiteral::Null
                    }
                )
            ]
        );

        assert_field!(
            r#"
        .field static protected clazz:La/b/c;
            .annotation system LAnnotation;
                value = .subannotation LSubann;
                    nested = .subannotation LSub/Subann;
                        value = true
                    .end subannotation
                .end subannotation
                numeric = 0x23
                null = null
            .end annotation
        .end field"#,
            access: STATIC | PROTECTED,
            name: "clazz",
            ty: Type::new_class("La/b/c;"),
            raw_value: RawLiteral::Unset,
            annotations: [
                annotation!(
                    class: "LAnnotation;",
                    vis: System,
                    params: {
                        "value" = annotation!(
                            class: "LSubann;",
                            vis: Unset,
                            params: {
                                "nested" = annotation!(
                                    class: "LSub/Subann;",
                                    vis: Unset,
                                    params: {
                                        "value" = RawLiteral::Bool(true)
                                    }
                                )
                            }
                        ),
                        "numeric" = RawLiteral::Numeric("0x23"),
                        "null" = RawLiteral::Null
                    }
                )
            ]
        );

        assert_field!(
                r#"
        .field static protected clazz:La/b/c;
            .annotation build LAnnotation;
                value = .enum Le/n/u/M;->ENUM:LE/n/u/m;
            .end annotation
        .end field"#,
        access: STATIC | PROTECTED,
        name: "clazz",
        ty: Type::new_class("La/b/c;"),
        raw_value: RawLiteral::Unset,
        annotations: [
            annotation!(
                class: "LAnnotation;",
                vis: Build,
                params: {
                    "value" = Enum{
                        owner: "Le/n/u/M;",
                        name: "ENUM",
                        ty: "LE/n/u/m;",
                    }
                }
            )
        ]

            );
    }

    #[test]
    fn parse_catch() {
        assert_line!(
            ".catch Ljava/lang/Exception; {:try_start_0 .. :try_end_9} :catch_b\n",
            Line::NamedCatch(RawNamedCatch::new(
                "Ljava/lang/Exception;",
                RawLabel::new("try_start_0"),
                RawLabel::new("try_end_9"),
                RawLabel::new("catch_b")
            ))
        )
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn parse_param_annotation() {
        assert_line!(
            // TODO maybe make an assert that lets an EOF error
            // make things happy with no EOF
            ".param p0, \"paramName\"\n.end param\n",
            Line::ParamLine(reg!(p0), "paramName", None)
        );
        assert_line!(
            r#".param p1    # I
    .annotation build Lcom/android/server/wm/RecentsAnimationController$ReorderMode;
    .end annotation
.end param
.param p2"#,
            Line::ParamLine(
                reg!(p1),
                "",
                Some(vec![
                    annotation!(class: "Lcom/android/server/wm/RecentsAnimationController$ReorderMode;", vis: Build, params: {})
                ])
            )
        );
    }

    #[cfg(not(feature = "annotations"))]
    #[test]
    fn parse_param_skips_annotation() {
        assert_line!(
            // TODO maybe make an assert that lets an EOF error
            // make things happy with no EOF
            ".param p0, \"paramName\"\n.end param\n",
            Line::ParamLine(reg!(p0), "paramName", None)
        );
        assert_line!(
            r#".param p1    # I
    .annotation build Lcom/android/server/wm/RecentsAnimationController$ReorderMode;
    .end annotation
.end param
.param p2"#,
            Line::ParamLine(reg!(p1), "", None)
        );
    }

    macro_rules! assert_method {
        (
            $str:literal,
            access: $($acc:ident)|*,
            name: $name:literal,
            inputs: $input:literal,
            ret: $ret:expr
        ) => {
            assert_line!($str, Line::MethodHeader(MethodHeader::new(
                 $name,
                 $(AccessFlag::$acc)|*,
                 $input,
                 $ret
            )));
        }
    }

    #[test]
    fn parse_method_header() {
        assert_method!(
            ".method public static final nop()V",
            access: PUBLIC | STATIC | FINAL,
            name: "nop",
            inputs: "",
            ret: Type::new_prim(Primitive::Void)
        );

        assert_method!(
            ".method private `has spaces`(IJZLjava/lang/Object;DF)Ljava/lang/String;",
            access: PRIVATE,
            name: "has spaces",
            inputs: "IJZLjava/lang/Object;DF",
            ret: Type::new_class("Ljava/lang/String;")
        );
    }

    macro_rules! assert_next_line {
        ($parser:ident, $expected:expr) => {{
            let res = $parser.parse_line();
            assert!(res.is_ok(), "expected no error: {:?}", res);
            assert_eq!(res.unwrap(), $expected);
        }};
    }

    #[test]
    fn parse_simple_beginning() {
        let arena = Arena::new();
        let file = r#".class private abstract La/b/C;
.super Ljava/lang/Object;
.source "Source.java"

.implements Lq/r/T;
.implements Lf/o/O;"#;

        let lexer = Lexer::new(file.as_bytes(), &arena);
        let mut parser = Parser::new(lexer);
        assert_next_line!(
            parser,
            Line::Class(AccessFlag::PRIVATE | AccessFlag::ABSTRACT, "La/b/C;")
        );
        assert_next_line!(parser, Line::Super("Ljava/lang/Object;"));
        assert_next_line!(parser, Line::Interface("Lq/r/T;"));
        assert_next_line!(parser, Line::Interface("Lf/o/O;"));
    }

    #[test]
    fn parse_packed_switch() {
        assert_line!(
            r#"
            :pswitch_data_22
            .packed-switch 0x0
                :pswitch_22
                :pswitch_26
                :pswitch_30
            .end packed-switch
            "#,
            Line::PackedSwitchData(RawPackedSwitchData {
                label: "pswitch_data_22".into(),
                start: "0x0",
                labels: vec![
                    RawLabel::new("pswitch_22"),
                    RawLabel::new("pswitch_26"),
                    RawLabel::new("pswitch_30"),
                ]
            })
        );
    }

    #[test]
    fn parse_sparse_switch() {
        assert_line!(
            r#"
            :sswitch_data_22
            .sparse-switch
                0x100 -> :sswitch_22
                0x1437 -> :sswitch_26
                -0x12f1 -> :sswitch_30
            .end sparse-switch
            "#,
            Line::SparseSwitchData(RawSparseSwitchData {
                label: "sswitch_data_22".into(),
                data: vec![
                    RawSwitchPair {
                        num: "0x100",
                        label: RawLabel::new("sswitch_22"),
                    },
                    RawSwitchPair {
                        num: "0x1437",
                        label: RawLabel::new("sswitch_26"),
                    },
                    RawSwitchPair {
                        num: "-0x12f1",
                        label: RawLabel::new("sswitch_30"),
                    }
                ]
            })
        );
    }

    #[test]
    fn parse_method_array_data() {
        let arena = Arena::new();
        let line = r#".method public final g()Ljava/lang/String;
    .registers 2

    const-string v0, "DEF"

    return-object v0

    nop

    .array-data 1
    .end array-data
.end method
"#;

        let lex = Lexer::new(line.as_bytes(), &arena);
        let mut parser = Parser::new(lex);
        parse_method(&mut parser).expect("failed to parse method");
    }

    #[test]
    fn parse_array_data() {
        assert_line!(
            r#"
            :array_12
            .array-data 1
                0x1ft
                0x31t
                0x27t
                0x3at
                -0x43t
            .end array-data
            "#,
            Line::ArrayData(RawArrayData {
                label: "array_12".into(),
                data_size: "1",
                data: vec!["0x1ft", "0x31t", "0x27t", "0x3at", "-0x43t"]
            })
        );
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn parse_annotation_1() {
        assert_line!(
            r#".annotation system Ldalvik/annotation/MemberClasses;
    value = {
        Landroid/app/slice/SliceItem$SliceType;
    }
.end annotation
"#,
            Line::Annotation(annotation!(
            class: "Ldalvik/annotation/MemberClasses;",
            vis: System,
            params: {
                "value" = AnnotationValue::List(vec![AnnotationValue::Type(Type::new_class("Landroid/app/slice/SliceItem$SliceType;"))])
            }
            ))
        );
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn parse_annotation_2() {
        assert_line!(
            r#".annotation system Ldalvik/annotation/MemberClasses;
    value = V
.end annotation
"#,
            Line::Annotation(annotation!(
            class: "Ldalvik/annotation/MemberClasses;",
            vis: System,
            params: {
                "value" = Type::new_prim(Primitive::Void)
            }
            ))
        );
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn parse_annotation_3() {
        assert_line!(
            r#".annotation runtime Lkotlin/Metadata;
    d1 = {
        "\u0000*\n"
    }
    d2 = {
        "Lokio/Segment;",
        "",
        "()V",
    }
    k = 0x1
    mv = {
        0x1,
        0x6,
        0x0
    }
    xi = 0x30
.end annotation
"#,
            Line::Annotation(annotation!(
            class: "Lkotlin/Metadata;",
            vis: Runtime,
            params: {
                    "d1" = AnnotationValue::List(vec![AnnotationValue::Lit(RawLiteral::String("\\u0000*\\n"))]),
                    "d2" = AnnotationValue::List(vec![
                        AnnotationValue::Lit(RawLiteral::String("Lokio/Segment;")),
                        AnnotationValue::Lit(RawLiteral::String("")),
                        AnnotationValue::Lit(RawLiteral::String("()V")),
                    ]),
                    "k" = AnnotationValue::Lit(RawLiteral::Numeric("0x1")),
                    "mv" = AnnotationValue::List(vec![
                        AnnotationValue::Lit(RawLiteral::Numeric("0x1")),
                        AnnotationValue::Lit(RawLiteral::Numeric("0x6")),
                        AnnotationValue::Lit(RawLiteral::Numeric("0x0"))
                    ]),
                    "xi" = AnnotationValue::Lit(RawLiteral::Numeric("0x30"))
            }
            ))
        );
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn parse_annotation_4() {
        assert_line!(
            r#".annotation system Ldalvik/annotation/MemberClasses;
    value = {}
.end annotation
"#,
            Line::Annotation(annotation!(
            class: "Ldalvik/annotation/MemberClasses;",
            vis: System,
            params: {
                "value" = AnnotationValue::List(vec![])
            }
            ))
        );
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn parse_annotation_5() {
        assert_line!(
            r#".annotation runtime Lcom/android/systemui/plugins/annotations/Dependencies;
    value = {
        .subannotation Lcom/android/systemui/plugins/annotations/DependsOn;
            target = Lcom/android/systemui/plugins/GlobalActionsPanelPlugin$Callbacks;
        .end subannotation,
        .subannotation Lcom/android/systemui/plugins/annotations/DependsOn;
            target = Lcom/android/systemui/plugins/GlobalActionsPanelPlugin$PanelViewController;
        .end subannotation
    }
    .end annotation
    "#,
            Line::Annotation(annotation!(
            class: "Lcom/android/systemui/plugins/annotations/Dependencies;",
            vis: Runtime,
            params: {
                "value" = AnnotationValue::List(vec![
                        AnnotationValue::Subannotation(
                            annotation!(
                                class: "Lcom/android/systemui/plugins/annotations/DependsOn;",
                                vis: Unset,
                                params: {
                                    "target" = Type::new_class("Lcom/android/systemui/plugins/GlobalActionsPanelPlugin$Callbacks;")
                                }
                            )
                        ),
                        AnnotationValue::Subannotation(
                            annotation!(
                                class: "Lcom/android/systemui/plugins/annotations/DependsOn;",
                                vis: Unset,
                                params: {
                                    "target" = Type::new_class("Lcom/android/systemui/plugins/GlobalActionsPanelPlugin$PanelViewController;")
                                }
                            )
                        )
                ])
            }
            ))
        );
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn parse_annotation_6() {
        assert_line!(
            r#".annotation runtime Lcom/android/systemui/plugins/annotations/ProvidesInterface;
    action = "com.android.systemui.action.PLUGIN_GLOBAL_ACTIONS_PANEL"
    version = 0x0
.end annotation
    "#,
            Line::Annotation(annotation!(
            class: "Lcom/android/systemui/plugins/annotations/ProvidesInterface;",
            vis: Runtime,
            params: {
                "action" = AnnotationValue::Lit(RawLiteral::String("com.android.systemui.action.PLUGIN_GLOBAL_ACTIONS_PANEL")),
                "version" = AnnotationValue::Lit(RawLiteral::Numeric("0x0"))
            }
            ))
        );
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn parse_annotation_7() {
        assert_line!(
            r#".annotation system Ldalvik/annotation/MemberClasses;
    value = {
        Lcom/android/systemui/plugins/GlobalActionsPanelPlugin$PanelViewController;,
        Lcom/android/systemui/plugins/GlobalActionsPanelPlugin$Callbacks;
    }
.end annotation
    "#,
            Line::Annotation(annotation!(
            class: "Ldalvik/annotation/MemberClasses;",
            vis: System,
            params: {
                "value" = AnnotationValue::List(vec![
                        AnnotationValue::Type(Type::new_class("Lcom/android/systemui/plugins/GlobalActionsPanelPlugin$PanelViewController;")),
                        AnnotationValue::Type(Type::new_class("Lcom/android/systemui/plugins/GlobalActionsPanelPlugin$Callbacks;"))
                    ])
            }
            ))
        );
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn parse_annotation_8() {
        assert_line!(
            r#".annotation runtime Lcom/oracle/svm/core/annotate/RecomputeFieldValue;
        declClass = [B
    .end annotation
"#,
            Line::Annotation(annotation!(
                    class: "Lcom/oracle/svm/core/annotate/RecomputeFieldValue;",
                    vis: Runtime,
                    params: {
                        "declClass" = AnnotationValue::Type(
                            Type::new_prim_array(Primitive::Byte, 1)
                        )
                    }
            ))
        );

        assert_line!(
            r#".annotation runtime Lcom/oracle/svm/core/annotate/RecomputeFieldValue;
        declClass = [[La/b/c;
    .end annotation
"#,
            Line::Annotation(annotation!(
                    class: "Lcom/oracle/svm/core/annotate/RecomputeFieldValue;",
                    vis: Runtime,
                    params: {
                        "declClass" = AnnotationValue::Type(
                            Type::new_class_array("La/b/c;", 2)
                        )
                    }
            ))
        );
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn parse_annotation_enclosing_method() {
        assert_line!(
            r#".annotation system Ldalvik/annotation/EnclosingMethod;
    value = La/b/c/IFoo$Stub;->onTransact(ILandroid/os/HwParcel;Landroid/os/HwParcel;I)V
.end annotation
"#,
            Line::Annotation(annotation!(
            class: "Ldalvik/annotation/EnclosingMethod;",
            vis: System,
            params: {
                "value" = MethodRef::new(
                    "La/b/c/IFoo$Stub;",
                    "onTransact",
                    "ILandroid/os/HwParcel;Landroid/os/HwParcel;I",
                    Type::Primitive(Primitive::Void, 0)
                )
            }
            ))
        );
    }

    #[cfg(feature = "annotations")]
    #[test]
    fn parse_annotation_nested_values() {
        assert_line!(
            r#"
.annotation system Ldalvik/annotation/Record;
    componentAnnotationVisibilities = {
        {{}, {}}
    }
.end annotation"#,
            Line::Annotation(annotation!(
                class: "Ldalvik/annotation/Record;",
                vis: System,
                params: {
                    "componentAnnotationVisibilities" = AnnotationValue::List(
                        vec![
                            AnnotationValue::List(
                                vec![
                                    AnnotationValue::List(vec![]),
                                    AnnotationValue::List(vec![])
                                ]
                            )
                        ]
                    )
                }
            ))
        );
    }

    #[cfg(not(feature = "annotations"))]
    #[test]
    fn parse_class_fields() {
        let arena = Arena::new();
        let raw = r#".class Lcom/sec/android/iaft/IAFDDiagnosis$IAFD_CONTROLINFO;
.super Ljava/lang/Object;

.field private blacklist field:Ljava/util/HashMap;
    .annotation system Ldalvik/annotation/Signature;
        value = {
            "Ljava/util/HashMap<",
            "Ljava/lang/String;",
            "[",
            "Ljava/lang/String;",
            ">;"
        }
    .end annotation
.end field

.field private blacklist anotherField:Z
"#;

        let smali_line = raw.as_bytes();
        let lexer = Lexer::new(smali_line, &arena);
        let mut parser = Parser::new(lexer);
        super::parse_class(&mut parser).expect("failed to parse class");
    }
}
