# smalisa 

smalisa is a rust crate intended to facilitate static analysis of smali files. Note that the input smali files are expected to be syntactically valid. This speeds up the lexing and parsing phases of the analysis and is a hard requirement.

Note that input files must be UTF8 encoded: this crate will use `String::from_utf8_lossy` on input bytes.

## Usage

## Details

All comments and certain directives from the files are ignored. The following directives are ignored:

- `.source`
- `.line`
- `.registers`
- `.local`
- `.end local`
- `.restart local`

Since we assume valid input smali, some tokens are matched with the shortest possible amount of characters. For example, the `new-array` instruction is matched after only the `new-a` as there are no other instructions that start with that prefix. Many things are matched this way.
