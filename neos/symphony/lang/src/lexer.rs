//! Tokeniser for the Symphony DSL.
//!
//! # A2 is enforced here, not downstream
//!
//! Axiom A2 deprecates Boolean truth values. The strongest way to honour that
//! in a *language* is to make Boolean constructs **unlexable** — `true`,
//! `false`, `&&`, `||`, `!`, `==`, `if`, `else` are not merely unused
//! identifiers, they are rejected with an error naming the axiom.
//!
//! A convention ("please don't write `if`") would survive exactly as long as
//! nobody was in a hurry. A lexer error survives everything.

use crate::LangError;

/// A source token.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // keywords
    Task,
    At,
    Hz,
    Phase,
    Scale,
    When,
    Aligns,
    Opposes,
    Resonates,
    Detunes,
    Fork,
    Invert,
    Emit,
    // the instruction-executing state machine (`_mkb/instruction_set.md`)
    Store,
    Load,
    Cell,
    Path,
    Acquire,
    Release,
    Halt,
    // phase literals - A2's permitted pair, written as signs
    Positive,
    Negative,
    // structure
    OpenBrace,
    CloseBrace,
    // leaves
    Ident(String),
    Number(f64),
}

/// Constructs that would reintroduce Boolean logic, with the reason each is
/// refused. Rejected at lex time so A2 cannot be violated even by accident.
const FORBIDDEN: &[(&str, &str)] = &[
    ("true", "A2 deprecates Boolean truth values; use a phase orientation"),
    ("false", "A2 deprecates Boolean truth values; use a phase orientation"),
    ("if", "conditionals are phase alignment: use `when X aligns Y`"),
    ("else", "there is no Boolean complement; use `when X opposes Y`"),
    ("&&", "no Boolean conjunction; combine phases by interference"),
    ("||", "no Boolean disjunction; combine phases by interference"),
    ("!", "no Boolean negation; the opposite of a phase is its inversion"),
    ("==", "no equality test; alignment is measured, not compared"),
    ("!=", "no inequality test; use `when X opposes Y`"),
    ("bool", "there is no Boolean type in this language"),
    ("and", "no Boolean conjunction; combine phases by interference"),
    ("or", "no Boolean disjunction; combine phases by interference"),
    ("not", "no Boolean negation; the opposite of a phase is its inversion"),
];

/// A token with the line it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned {
    pub token: Token,
    pub line: usize,
}

/// Tokenise Symphony source.
///
/// # Errors
/// [`LangError::ForbiddenBooleanConstruct`] for anything that would reintroduce
/// Boolean logic, and [`LangError::UnexpectedCharacter`] for stray input.
pub fn lex(source: &str) -> Result<Vec<Spanned>, LangError> {
    let mut out = Vec::new();

    for (line_no, line) in source.lines().enumerate() {
        let line_no = line_no + 1;
        let line = line.split('#').next().unwrap_or(""); // `#` starts a comment

        // Reject Boolean operator punctuation before it can be split away.
        for (bad, why) in FORBIDDEN {
            if !bad.chars().next().unwrap().is_alphabetic() && line.contains(bad) {
                return Err(LangError::ForbiddenBooleanConstruct {
                    construct: (*bad).to_string(),
                    reason: (*why).to_string(),
                    line: line_no,
                });
            }
        }

        for raw in line.split_whitespace() {
            // Split trailing/leading braces off a word so `{` need not be spaced.
            let mut pieces: Vec<&str> = Vec::new();
            let mut rest = raw;
            while let Some(stripped) = rest.strip_prefix('{') {
                pieces.push("{");
                rest = stripped;
            }
            let mut tail = Vec::new();
            while let Some(stripped) = rest.strip_suffix('}') {
                tail.push("}");
                rest = stripped;
            }
            if !rest.is_empty() {
                pieces.push(rest);
            }
            pieces.extend(tail);

            for piece in pieces {
                if piece.is_empty() {
                    continue;
                }
                let lowered = piece.to_ascii_lowercase();
                if let Some((bad, why)) = FORBIDDEN.iter().find(|(b, _)| *b == lowered) {
                    return Err(LangError::ForbiddenBooleanConstruct {
                        construct: (*bad).to_string(),
                        reason: (*why).to_string(),
                        line: line_no,
                    });
                }

                let token = match lowered.as_str() {
                    "task" => Token::Task,
                    "at" => Token::At,
                    "hz" => Token::Hz,
                    "phase" => Token::Phase,
                    "scale" => Token::Scale,
                    "when" => Token::When,
                    "aligns" => Token::Aligns,
                    "opposes" => Token::Opposes,
                    "resonates" => Token::Resonates,
                    "detunes" => Token::Detunes,
                    "fork" => Token::Fork,
                    "invert" => Token::Invert,
                    "emit" => Token::Emit,
                    "store" => Token::Store,
                    "load" => Token::Load,
                    "cell" => Token::Cell,
                    "path" => Token::Path,
                    "acquire" => Token::Acquire,
                    "release" => Token::Release,
                    "halt" => Token::Halt,
                    "+" => Token::Positive,
                    "-" => Token::Negative,
                    "{" => Token::OpenBrace,
                    "}" => Token::CloseBrace,
                    _ => {
                        if let Ok(n) = piece.parse::<f64>() {
                            Token::Number(n)
                        } else if piece
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_')
                            && piece.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                        {
                            Token::Ident(piece.to_string())
                        } else {
                            return Err(LangError::UnexpectedCharacter {
                                text: piece.to_string(),
                                line: line_no,
                            });
                        }
                    }
                };
                out.push(Spanned {
                    token,
                    line: line_no,
                });
            }
        }
    }

    Ok(out)
}
