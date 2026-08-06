use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    generate_lexer_matches();
    println!("cargo:rerun-if-changed=build.rs");
}

/*
 * We're generating a nested match statement for lexing directives,
 * access modifiers, and instructions. Since this crate is only supposed
 * to deal with valid smali, we match on the minimum unique length. For
 * example, .p will be all we need for the .param directive.
 *
 * The generate_lexer_matches outputs a ~8K line file that just defines
 * the lex_directive, lex_instruction, and lex_access_flag methods. It is
 * included into lexer.rs
 */

struct LexerStringMatch {
    match_string: &'static str,
    token_type: &'static str,
    can_eof: bool,
}

fn generate_lexer_matches() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let name = Path::new(&out_dir).join("lex_gen.rs");
    let mut out = File::create(&name).unwrap();
    out.write_all(
        r#"#[rust_analyzer::skip]
        impl <'a, 'b, R: Read> Lexer<'a, R>
where
    'a: 'b,
{
    "#
        .as_bytes(),
    )
    .unwrap();
    gen_string_match(&mut out, DIRECTIVES, "directive", "Directive", false);
    gen_string_match(&mut out, INSTRUCTIONS, "instruction", "Instruction", true);
    out.write_all(&[b'}']).unwrap();
    out.flush().unwrap();
}

fn gen_string_match(
    file: &mut File,
    matches: &[LexerStringMatch],
    func_name: &str,
    token_type: &str,
    takes_byte: bool,
) {
    if takes_byte {
        file.write_fmt(format_args!(
            "fn lex_{}(&mut self, c: u8, into: &mut Token<'b>) -> LexResult<'a> {{\n",
            func_name,
        ))
    } else {
        file.write_fmt(format_args!(
            "fn lex_{}(&mut self, into: &mut Token<'b>) -> LexResult<'a> {{\nlet c = self.next_byte_no_eof()?;\n",
            func_name,
        ))
    }
    .unwrap();
    gen_string_match_recurse(file, matches, token_type, "");
    file.write_fmt(format_args!(";\n*into = Token::{}(val);\n", token_type))
        .unwrap();
    file.write_all("Ok(())\n}".as_bytes()).unwrap();
}

fn gen_string_match_recurse(
    file: &mut File,
    matches: &[LexerStringMatch],
    token_type: &str,
    previous_chars: &str,
) {
    if !previous_chars.is_empty() {
        let s = if matches
            .iter()
            .any(|m| m.match_string.is_empty() && m.can_eof)
        {
            "let c = self.next_byte()?;\n"
        } else {
            "let c = self.next_byte_no_eof()?;\n"
        };
        file.write_all(s.as_bytes()).unwrap();
    } else {
        file.write_all("let val = ".as_bytes()).unwrap();
    }

    file.write_all("match c {\n".as_bytes()).unwrap();

    let mut poss: HashMap<&str, Vec<LexerStringMatch>> = HashMap::new();

    for m in matches {
        if m.match_string.is_empty() {
            file.write_fmt(format_args!(
                "b' ' | b'\\n' | b'\\t' => {{ *into = Token::{}({}); return Ok(()); }}\n",
                token_type, m.token_type
            ))
            .unwrap();
            continue;
        }
        let (first, rem) = m.match_string.split_at(1);
        let item = LexerStringMatch {
            match_string: rem,
            token_type: m.token_type,
            can_eof: m.can_eof,
        };
        if let Some(vec) = poss.get_mut(first) {
            vec.push(item);
        } else {
            poss.insert(first, vec![item]);
        }
    }

    for (b, vec) in poss {
        if vec.len() == 1 {
            let it = vec.first().unwrap();
            if it.can_eof {
                file.write_fmt(format_args!(
                    "b'{}' => {{ self.skip_n({})?; {} }},\n",
                    b,
                    it.match_string.len(),
                    it.token_type
                ))
                .unwrap();
            } else {
                file.write_fmt(format_args!(
                    "b'{}' => {{ self.skip_n({})?; {} }},\n",
                    b,
                    it.match_string.len(),
                    it.token_type
                ))
                .unwrap();
            }
        } else {
            file.write_fmt(format_args!("b'{}' => {{\n", b)).unwrap();
            let new_prev = format!("{}{}", previous_chars, b);
            gen_string_match_recurse(file, vec.as_slice(), token_type, &new_prev);
            file.write_all("}\n".as_bytes()).unwrap();
        }
    }
    file.write_all("_ => {\n".as_bytes()).unwrap();
    if !previous_chars.is_empty() {
        file.write_fmt(format_args!(
            "self.push_all(\"{}\".as_bytes());\n",
            previous_chars
        ))
        .unwrap();
    }
    file.write_fmt(format_args!(
        r#"self.push(c);
self.take_until_whitespace()?;
let rem = self.take_str();
return Err(LexError::Unknown{}(rem, self.offset));"#,
        token_type
    ))
    .unwrap();
    file.write_all("\n}\n}\n".as_bytes()).unwrap();
}

static DIRECTIVES: &[LexerStringMatch] = &[
    LexerStringMatch {
        match_string: "class",
        token_type: "Directive::Class",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "super",
        token_type: "Directive::Super",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "implements",
        token_type: "Directive::Implements",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "source",
        token_type: "Directive::Source",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "field",
        token_type: "Directive::Field",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "end field",
        token_type: "Directive::EndField",
        can_eof: true,
    },
    LexerStringMatch {
        match_string: "subannotation",
        token_type: "Directive::Subannotation",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "end subannotation",
        token_type: "Directive::EndSubannotation",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "annotation",
        token_type: "Directive::Annotation",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "end annotation",
        token_type: "Directive::EndAnnotation",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "enum",
        token_type: "Directive::Enum",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "method",
        token_type: "Directive::Method",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "end method",
        token_type: "Directive::EndMethod",
        can_eof: true,
    },
    LexerStringMatch {
        match_string: "registers",
        token_type: "Directive::Registers",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "locals",
        token_type: "Directive::Locals",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "array-data",
        token_type: "Directive::ArrayData",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "end array-data",
        token_type: "Directive::EndArrayData",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "packed-switch",
        token_type: "Directive::PackedSwitch",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "end packed-switch",
        token_type: "Directive::EndPackedSwitch",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sparse-switch",
        token_type: "Directive::SparseSwitch",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "end sparse-switch",
        token_type: "Directive::EndSparseSwitch",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "catch",
        token_type: "Directive::Catch",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "catchall",
        token_type: "Directive::CatchAll",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "line",
        token_type: "Directive::Line",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "param",
        token_type: "Directive::Param",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "end param",
        token_type: "Directive::EndParam",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "local",
        token_type: "Directive::Local",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "end local",
        token_type: "Directive::EndLocal",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "restart local",
        token_type: "Directive::RestartLocal",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "prologue",
        token_type: "Directive::Prologue",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "epilogue",
        token_type: "Directive::Epilogue",
        can_eof: false,
    },
];

static INSTRUCTIONS: &[LexerStringMatch] = &[
    LexerStringMatch {
        match_string: "goto",
        token_type: "INS_GOTO",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "goto/16",
        token_type: "INS_GOTO_16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "goto/32",
        token_type: "INS_GOTO_32",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "return",
        token_type: "INS_RETURN",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "return-void",
        token_type: "INS_RETURN_VOID",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "return-wide",
        token_type: "INS_RETURN_WIDE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "return-object",
        token_type: "INS_RETURN_OBJECT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "return-void-barrier",
        token_type: "INS_RETURN_VOID_BARRIER",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "return-void-no-barrier",
        token_type: "INS_RETURN_VOID_NO_BARRIER",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "nop",
        token_type: "INS_NOP",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "const",
        token_type: "INS_CONST",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "const/4",
        token_type: "INS_CONST_4",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "const/16",
        token_type: "INS_CONST_16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "const-wide",
        token_type: "INS_CONST_WIDE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "const-wide/16",
        token_type: "INS_CONST_WIDE_16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "const-wide/32",
        token_type: "INS_CONST_WIDE_32",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "const/high16",
        token_type: "INS_CONST_HIGH16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "const-wide/high16",
        token_type: "INS_CONST_WIDE_HIGH16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "const-string",
        token_type: "INS_CONST_STRING",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "const-string/jumbo",
        token_type: "INS_CONST_STRING_JUMBO",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "const-method-handle",
        token_type: "INS_CONST_METHOD_HANDLE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "const-method-type",
        token_type: "INS_CONST_METHOD_TYPE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "if-eqz",
        token_type: "INS_IF_EQZ",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "if-nez",
        token_type: "INS_IF_NEZ",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "if-ltz",
        token_type: "INS_IF_LTZ",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "if-gez",
        token_type: "INS_IF_GEZ",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "if-gtz",
        token_type: "INS_IF_GTZ",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "if-lez",
        token_type: "INS_IF_LEZ",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "if-eq",
        token_type: "INS_IF_EQ",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "if-ne",
        token_type: "INS_IF_NE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "if-lt",
        token_type: "INS_IF_LT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "if-ge",
        token_type: "INS_IF_GE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "if-gt",
        token_type: "INS_IF_GT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "if-le",
        token_type: "INS_IF_LE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "move-result",
        token_type: "INS_MOVE_RESULT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "move-result-wide",
        token_type: "INS_MOVE_RESULT_WIDE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "move-result-object",
        token_type: "INS_MOVE_RESULT_OBJECT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "move-exception",
        token_type: "INS_MOVE_EXCEPTION",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "monitor-enter",
        token_type: "INS_MONITOR_ENTER",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "monitor-exit",
        token_type: "INS_MONITOR_EXIT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "throw",
        token_type: "INS_THROW",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "move",
        token_type: "INS_MOVE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "move-wide",
        token_type: "INS_MOVE_WIDE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "move-object",
        token_type: "INS_MOVE_OBJECT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "array-length",
        token_type: "INS_ARRAY_LENGTH",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "neg-int",
        token_type: "INS_NEG_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "not-int",
        token_type: "INS_NOT_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "neg-long",
        token_type: "INS_NEG_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "not-long",
        token_type: "INS_NOT_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "neg-float",
        token_type: "INS_NEG_FLOAT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "neg-double",
        token_type: "INS_NEG_DOUBLE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "int-to-long",
        token_type: "INS_INT_TO_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "int-to-float",
        token_type: "INS_INT_TO_FLOAT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "int-to-double",
        token_type: "INS_INT_TO_DOUBLE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "long-to-int",
        token_type: "INS_LONG_TO_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "long-to-float",
        token_type: "INS_LONG_TO_FLOAT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "long-to-double",
        token_type: "INS_LONG_TO_DOUBLE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "float-to-int",
        token_type: "INS_FLOAT_TO_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "float-to-long",
        token_type: "INS_FLOAT_TO_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "float-to-double",
        token_type: "INS_FLOAT_TO_DOUBLE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "double-to-int",
        token_type: "INS_DOUBLE_TO_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "double-to-long",
        token_type: "INS_DOUBLE_TO_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "double-to-float",
        token_type: "INS_DOUBLE_TO_FLOAT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "int-to-byte",
        token_type: "INS_INT_TO_BYTE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "int-to-char",
        token_type: "INS_INT_TO_CHAR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "int-to-short",
        token_type: "INS_INT_TO_SHORT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "add-int/2addr",
        token_type: "INS_ADD_INT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sub-int/2addr",
        token_type: "INS_SUB_INT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "mul-int/2addr",
        token_type: "INS_MUL_INT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "div-int/2addr",
        token_type: "INS_DIV_INT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "rem-int/2addr",
        token_type: "INS_REM_INT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "and-int/2addr",
        token_type: "INS_AND_INT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "or-int/2addr",
        token_type: "INS_OR_INT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "xor-int/2addr",
        token_type: "INS_XOR_INT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "shl-int/2addr",
        token_type: "INS_SHL_INT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "shr-int/2addr",
        token_type: "INS_SHR_INT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "ushr-int/2addr",
        token_type: "INS_USHR_INT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "add-long/2addr",
        token_type: "INS_ADD_LONG_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sub-long/2addr",
        token_type: "INS_SUB_LONG_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "mul-long/2addr",
        token_type: "INS_MUL_LONG_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "div-long/2addr",
        token_type: "INS_DIV_LONG_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "rem-long/2addr",
        token_type: "INS_REM_LONG_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "and-long/2addr",
        token_type: "INS_AND_LONG_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "or-long/2addr",
        token_type: "INS_OR_LONG_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "xor-long/2addr",
        token_type: "INS_XOR_LONG_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "shl-long/2addr",
        token_type: "INS_SHL_LONG_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "shr-long/2addr",
        token_type: "INS_SHR_LONG_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "ushr-long/2addr",
        token_type: "INS_USHR_LONG_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "add-float/2addr",
        token_type: "INS_ADD_FLOAT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sub-float/2addr",
        token_type: "INS_SUB_FLOAT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "mul-float/2addr",
        token_type: "INS_MUL_FLOAT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "div-float/2addr",
        token_type: "INS_DIV_FLOAT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "rem-float/2addr",
        token_type: "INS_REM_FLOAT_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "add-double/2addr",
        token_type: "INS_ADD_DOUBLE_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sub-double/2addr",
        token_type: "INS_SUB_DOUBLE_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "mul-double/2addr",
        token_type: "INS_MUL_DOUBLE_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "div-double/2addr",
        token_type: "INS_DIV_DOUBLE_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "rem-double/2addr",
        token_type: "INS_REM_DOUBLE_2ADDR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sget",
        token_type: "INS_SGET",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sget-wide",
        token_type: "INS_SGET_WIDE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sget-object",
        token_type: "INS_SGET_OBJECT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sget-boolean",
        token_type: "INS_SGET_BOOLEAN",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sget-byte",
        token_type: "INS_SGET_BYTE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sget-char",
        token_type: "INS_SGET_CHAR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sget-short",
        token_type: "INS_SGET_SHORT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sput",
        token_type: "INS_SPUT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sput-wide",
        token_type: "INS_SPUT_WIDE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sput-object",
        token_type: "INS_SPUT_OBJECT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sput-boolean",
        token_type: "INS_SPUT_BOOLEAN",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sput-byte",
        token_type: "INS_SPUT_BYTE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sput-char",
        token_type: "INS_SPUT_CHAR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sput-short",
        token_type: "INS_SPUT_SHORT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sget-volatile",
        token_type: "INS_SGET_VOLATILE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sget-wide-volatile",
        token_type: "INS_SGET_WIDE_VOLATILE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sget-object-volatile",
        token_type: "INS_SGET_OBJECT_VOLATILE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sput-volatile",
        token_type: "INS_SPUT_VOLATILE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sput-wide-volatile",
        token_type: "INS_SPUT_WIDE_VOLATILE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sput-object-volatile",
        token_type: "INS_SPUT_OBJECT_VOLATILE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "check-cast",
        token_type: "INS_CHECK_CAST",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "new-instance",
        token_type: "INS_NEW_INSTANCE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "const-class",
        token_type: "INS_CONST_CLASS",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "add-int/lit8",
        token_type: "INS_ADD_INT_LIT8",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "rsub-int/lit8",
        token_type: "INS_RSUB_INT_LIT8",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "mul-int/lit8",
        token_type: "INS_MUL_INT_LIT8",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "div-int/lit8",
        token_type: "INS_DIV_INT_LIT8",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "rem-int/lit8",
        token_type: "INS_REM_INT_LIT8",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "and-int/lit8",
        token_type: "INS_AND_INT_LIT8",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "or-int/lit8",
        token_type: "INS_OR_INT_LIT8",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "xor-int/lit8",
        token_type: "INS_XOR_INT_LIT8",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "shl-int/lit8",
        token_type: "INS_SHL_INT_LIT8",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "shr-int/lit8",
        token_type: "INS_SHR_INT_LIT8",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "ushr-int/lit8",
        token_type: "INS_USHR_INT_LIT8",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iget",
        token_type: "INS_IGET",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iget-wide",
        token_type: "INS_IGET_WIDE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iget-object",
        token_type: "INS_IGET_OBJECT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iget-boolean",
        token_type: "INS_IGET_BOOLEAN",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iget-byte",
        token_type: "INS_IGET_BYTE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iget-char",
        token_type: "INS_IGET_CHAR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iget-short",
        token_type: "INS_IGET_SHORT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput",
        token_type: "INS_IPUT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-wide",
        token_type: "INS_IPUT_WIDE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-object",
        token_type: "INS_IPUT_OBJECT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-boolean",
        token_type: "INS_IPUT_BOOLEAN",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-byte",
        token_type: "INS_IPUT_BYTE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-char",
        token_type: "INS_IPUT_CHAR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-short",
        token_type: "INS_IPUT_SHORT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iget-volatile",
        token_type: "INS_IGET_VOLATILE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iget-wide-volatile",
        token_type: "INS_IGET_WIDE_VOLATILE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iget-object-volatile",
        token_type: "INS_IGET_OBJECT_VOLATILE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-volatile",
        token_type: "INS_IPUT_VOLATILE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-wide-volatile",
        token_type: "INS_IPUT_WIDE_VOLATILE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-object-volatile",
        token_type: "INS_IPUT_OBJECT_VOLATILE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "instance-of",
        token_type: "INS_INSTANCE_OF",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "new-array",
        token_type: "INS_NEW_ARRAY",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iget-quick",
        token_type: "INS_IGET_QUICK",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iget-wide-quick",
        token_type: "INS_IGET_WIDE_QUICK",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iget-object-quick",
        token_type: "INS_IGET_OBJECT_QUICK",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-quick",
        token_type: "INS_IPUT_QUICK",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-wide-quick",
        token_type: "INS_IPUT_WIDE_QUICK",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-object-quick",
        token_type: "INS_IPUT_OBJECT_QUICK",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-boolean-quick",
        token_type: "INS_IPUT_BOOLEAN_QUICK",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-byte-quick",
        token_type: "INS_IPUT_BYTE_QUICK",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-char-quick",
        token_type: "INS_IPUT_CHAR_QUICK",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "iput-short-quick",
        token_type: "INS_IPUT_SHORT_QUICK",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "rsub-int",
        token_type: "INS_RSUB_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "add-int/lit16",
        token_type: "INS_ADD_INT_LIT16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "mul-int/lit16",
        token_type: "INS_MUL_INT_LIT16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "div-int/lit16",
        token_type: "INS_DIV_INT_LIT16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "rem-int/lit16",
        token_type: "INS_REM_INT_LIT16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "and-int/lit16",
        token_type: "INS_AND_INT_LIT16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "or-int/lit16",
        token_type: "INS_OR_INT_LIT16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "xor-int/lit16",
        token_type: "INS_XOR_INT_LIT16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "move/from16",
        token_type: "INS_MOVE_FROM16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "move-wide/from16",
        token_type: "INS_MOVE_WIDE_FROM16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "move-object/from16",
        token_type: "INS_MOVE_OBJECT_FROM16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "cmpl-float",
        token_type: "INS_CMPL_FLOAT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "cmpg-float",
        token_type: "INS_CMPG_FLOAT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "cmpl-double",
        token_type: "INS_CMPL_DOUBLE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "cmpg-double",
        token_type: "INS_CMPG_DOUBLE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "cmp-long",
        token_type: "INS_CMP_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "aget",
        token_type: "INS_AGET",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "aget-wide",
        token_type: "INS_AGET_WIDE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "aget-object",
        token_type: "INS_AGET_OBJECT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "aget-boolean",
        token_type: "INS_AGET_BOOLEAN",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "aget-byte",
        token_type: "INS_AGET_BYTE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "aget-char",
        token_type: "INS_AGET_CHAR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "aget-short",
        token_type: "INS_AGET_SHORT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "aput",
        token_type: "INS_APUT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "aput-wide",
        token_type: "INS_APUT_WIDE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "aput-object",
        token_type: "INS_APUT_OBJECT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "aput-boolean",
        token_type: "INS_APUT_BOOLEAN",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "aput-byte",
        token_type: "INS_APUT_BYTE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "aput-char",
        token_type: "INS_APUT_CHAR",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "aput-short",
        token_type: "INS_APUT_SHORT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "add-int",
        token_type: "INS_ADD_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sub-int",
        token_type: "INS_SUB_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "mul-int",
        token_type: "INS_MUL_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "div-int",
        token_type: "INS_DIV_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "rem-int",
        token_type: "INS_REM_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "and-int",
        token_type: "INS_AND_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "or-int",
        token_type: "INS_OR_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "xor-int",
        token_type: "INS_XOR_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "shl-int",
        token_type: "INS_SHL_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "shr-int",
        token_type: "INS_SHR_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "ushr-int",
        token_type: "INS_USHR_INT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "add-long",
        token_type: "INS_ADD_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sub-long",
        token_type: "INS_SUB_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "mul-long",
        token_type: "INS_MUL_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "div-long",
        token_type: "INS_DIV_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "rem-long",
        token_type: "INS_REM_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "and-long",
        token_type: "INS_AND_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "or-long",
        token_type: "INS_OR_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "xor-long",
        token_type: "INS_XOR_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "shl-long",
        token_type: "INS_SHL_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "shr-long",
        token_type: "INS_SHR_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "ushr-long",
        token_type: "INS_USHR_LONG",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "add-float",
        token_type: "INS_ADD_FLOAT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sub-float",
        token_type: "INS_SUB_FLOAT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "mul-float",
        token_type: "INS_MUL_FLOAT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "div-float",
        token_type: "INS_DIV_FLOAT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "rem-float",
        token_type: "INS_REM_FLOAT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "add-double",
        token_type: "INS_ADD_DOUBLE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sub-double",
        token_type: "INS_SUB_DOUBLE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "mul-double",
        token_type: "INS_MUL_DOUBLE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "div-double",
        token_type: "INS_DIV_DOUBLE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "rem-double",
        token_type: "INS_REM_DOUBLE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "fill-array-data",
        token_type: "INS_FILL_ARRAY_DATA",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "packed-switch",
        token_type: "INS_PACKED_SWITCH",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "sparse-switch",
        token_type: "INS_SPARSE_SWITCH",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "move/16",
        token_type: "INS_MOVE_16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "move-wide/16",
        token_type: "INS_MOVE_WIDE_16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "move-object/16",
        token_type: "INS_MOVE_OBJECT_16",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "filled-new-array",
        token_type: "INS_FILLED_NEW_ARRAY",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "filled-new-array/range",
        token_type: "INS_FILLED_NEW_ARRAY_RANGE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "execute-inline",
        token_type: "INS_EXECUTE_INLINE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "execute-inline/range",
        token_type: "INS_EXECUTE_INLINE_RANGE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-custom",
        token_type: "INS_INVOKE_CUSTOM",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-virtual",
        token_type: "INS_INVOKE_VIRTUAL",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-super",
        token_type: "INS_INVOKE_SUPER",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-direct",
        token_type: "INS_INVOKE_DIRECT",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-static",
        token_type: "INS_INVOKE_STATIC",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-interface",
        token_type: "INS_INVOKE_INTERFACE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-direct-empty",
        token_type: "INS_INVOKE_DIRECT_EMPTY",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-virtual-quick",
        token_type: "INS_INVOKE_VIRTUAL_QUICK",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-super-quick",
        token_type: "INS_INVOKE_SUPER_QUICK",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-polymorphic",
        token_type: "INS_INVOKE_POLYMORPHIC",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-custom/range",
        token_type: "INS_INVOKE_CUSTOM_RANGE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-virtual/range",
        token_type: "INS_INVOKE_VIRTUAL_RANGE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-super/range",
        token_type: "INS_INVOKE_SUPER_RANGE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-direct/range",
        token_type: "INS_INVOKE_DIRECT_RANGE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-static/range",
        token_type: "INS_INVOKE_STATIC_RANGE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-interface/range",
        token_type: "INS_INVOKE_INTERFACE_RANGE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-object-init/range",
        token_type: "INS_INVOKE_OBJECT_INIT_RANGE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-virtual-quick/range",
        token_type: "INS_INVOKE_VIRTUAL_QUICK_RANGE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-super-quick/range",
        token_type: "INS_INVOKE_SUPER_QUICK_RANGE",
        can_eof: false,
    },
    LexerStringMatch {
        match_string: "invoke-polymorphic/range",
        token_type: "INS_INVOKE_POLYMORPHIC_RANGE",
        can_eof: false,
    },
];
