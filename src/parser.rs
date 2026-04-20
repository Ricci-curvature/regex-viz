//! Recursive-descent regex parser.
//!
//! Grammar (PEG-style):
//!   alt    := concat ('|' concat)*
//!   concat := repeat+
//!   repeat := atom ('*' | '+' | '?')?
//!   atom   := literal | '(' alt ')'
//!
//! Stage 1 scope: literal chars, '|', '*', '+', '?', '(' ')'. No classes, no
//! escapes, no '.'. Extended in later stages.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ast {
    Literal(char),
    Concat(Vec<Ast>),
    Alt(Vec<Ast>),
    Star(Box<Ast>),
    Plus(Box<Ast>),
    Question(Box<Ast>),
}

pub fn parse(src: &str) -> Result<Ast, String> {
    let mut p = Parser {
        src: src.chars().collect(),
        pos: 0,
    };
    let ast = p.parse_alt()?;
    if p.pos != p.src.len() {
        return Err(format!("unexpected '{}' at pos {}", p.src[p.pos], p.pos));
    }
    Ok(ast)
}

struct Parser {
    src: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn parse_alt(&mut self) -> Result<Ast, String> {
        let mut branches = vec![self.parse_concat()?];
        while self.peek() == Some('|') {
            self.bump();
            branches.push(self.parse_concat()?);
        }
        Ok(if branches.len() == 1 {
            branches.pop().unwrap()
        } else {
            Ast::Alt(branches)
        })
    }

    fn parse_concat(&mut self) -> Result<Ast, String> {
        let mut items = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            items.push(self.parse_repeat()?);
        }
        if items.is_empty() {
            return Err(format!("empty expression at pos {}", self.pos));
        }
        Ok(if items.len() == 1 {
            items.pop().unwrap()
        } else {
            Ast::Concat(items)
        })
    }

    fn parse_repeat(&mut self) -> Result<Ast, String> {
        let atom = self.parse_atom()?;
        Ok(match self.peek() {
            Some('*') => {
                self.bump();
                Ast::Star(Box::new(atom))
            }
            Some('+') => {
                self.bump();
                Ast::Plus(Box::new(atom))
            }
            Some('?') => {
                self.bump();
                Ast::Question(Box::new(atom))
            }
            _ => atom,
        })
    }

    fn parse_atom(&mut self) -> Result<Ast, String> {
        match self.peek() {
            Some('(') => {
                self.bump();
                let inner = self.parse_alt()?;
                if self.peek() != Some(')') {
                    return Err(format!("expected ')' at pos {}", self.pos));
                }
                self.bump();
                Ok(inner)
            }
            Some(c) if is_literal_char(c) => {
                self.bump();
                Ok(Ast::Literal(c))
            }
            Some(c) => Err(format!("unexpected '{}' at pos {}", c, self.pos)),
            None => Err("unexpected end of input".into()),
        }
    }
}

fn is_literal_char(c: char) -> bool {
    !matches!(
        c,
        '|' | '*' | '+' | '?' | '(' | ')' | '.' | '[' | ']' | '\\'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(c: char) -> Ast {
        Ast::Literal(c)
    }

    #[test]
    fn single_literal() {
        assert_eq!(parse("a").unwrap(), lit('a'));
    }

    #[test]
    fn concat_two() {
        assert_eq!(parse("ab").unwrap(), Ast::Concat(vec![lit('a'), lit('b')]));
    }

    #[test]
    fn alternation() {
        assert_eq!(parse("a|b").unwrap(), Ast::Alt(vec![lit('a'), lit('b')]));
    }

    #[test]
    fn star_plus_question() {
        assert_eq!(parse("a*").unwrap(), Ast::Star(Box::new(lit('a'))));
        assert_eq!(parse("a+").unwrap(), Ast::Plus(Box::new(lit('a'))));
        assert_eq!(parse("a?").unwrap(), Ast::Question(Box::new(lit('a'))));
    }

    #[test]
    fn group_then_star() {
        // (a|b)* → Star(Alt[a, b])
        assert_eq!(
            parse("(a|b)*").unwrap(),
            Ast::Star(Box::new(Ast::Alt(vec![lit('a'), lit('b')])))
        );
    }

    #[test]
    fn precedence_alt_lower_than_concat_and_star() {
        // a|b*c → Alt[a, Concat[Star(b), c]]
        assert_eq!(
            parse("a|b*c").unwrap(),
            Ast::Alt(vec![
                lit('a'),
                Ast::Concat(vec![Ast::Star(Box::new(lit('b'))), lit('c')])
            ])
        );
    }

    #[test]
    fn unbalanced_paren_errors() {
        assert!(parse("(a|b").is_err());
        assert!(parse("a)").is_err());
    }
}
