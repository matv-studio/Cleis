//! Formula columns: small, safe arithmetic over the other columns of the same row.
//!
//! Grammar: `expr := term (('+'|'-') term)*`, `term := factor (('*'|'/') factor)*`,
//! `factor := number | '[' column name ']' | ident '(' args ')' | '(' expr ')' | '-' factor`.
//! Functions: `yearly(x)`, `monthly(x)`, `default(x, d)`, `round(x)`, `min(a,b)`, `max(a,b)`.
//! A price keeps its currency through `+ - *` and `/` by a plain number.

use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum FormulaError {
    #[error("syntax error at {0}")]
    Syntax(usize),
    #[error("unknown column [{0}]")]
    UnknownColumn(String),
    #[error("unknown function {0}")]
    UnknownFunction(String),
    #[error("currencies differ")]
    Currency,
    #[error("division by zero")]
    DivZero,
    #[error("formula nested too deeply")]
    TooDeep,
}

/// Nesting allowed in a formula: far above any real one, low enough that a pasted or synced
/// `((((…` cannot overflow the stack.
const MAX_DEPTH: usize = 64;

/// A number, possibly a price (`currency`), possibly per period (`per_year` multiplier).
#[derive(Clone, Debug, PartialEq)]
pub struct Num {
    pub value: f64,
    pub currency: Option<String>,
    /// For a price: how many times a year it is paid (12 for monthly). `None` otherwise.
    pub per_year: Option<f64>,
}

impl Num {
    pub fn plain(value: f64) -> Self {
        Self {
            value,
            currency: None,
            per_year: None,
        }
    }
}

/// Supplies values for `[Column]` references. `Ok(None)` means the cell is empty.
pub trait Row {
    fn get(&self, column: &str) -> Result<Option<Num>, FormulaError>;
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Col(String),
    Ident(String),
    Op(char),
}

fn lex(src: &str) -> Result<Vec<(usize, Tok)>, FormulaError> {
    let b: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let st = i;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == '.') {
                i += 1;
            }
            let s: String = b[st..i].iter().collect();
            out.push((
                st,
                Tok::Num(s.parse().map_err(|_| FormulaError::Syntax(st))?),
            ));
        } else if c == '[' {
            let st = i;
            i += 1;
            let s0 = i;
            while i < b.len() && b[i] != ']' {
                i += 1;
            }
            if i == b.len() {
                return Err(FormulaError::Syntax(st));
            }
            out.push((
                st,
                Tok::Col(b[s0..i].iter().collect::<String>().trim().to_owned()),
            ));
            i += 1;
        } else if c.is_alphabetic() {
            let st = i;
            while i < b.len() && (b[i].is_alphanumeric() || b[i] == '_') {
                i += 1;
            }
            out.push((
                st,
                Tok::Ident(b[st..i].iter().collect::<String>().to_lowercase()),
            ));
        } else if "+-*/(),".contains(c) {
            out.push((i, Tok::Op(c)));
            i += 1;
        } else {
            return Err(FormulaError::Syntax(i));
        }
    }
    Ok(out)
}

struct Parser<'a, R: Row> {
    toks: Vec<(usize, Tok)>,
    pos: usize,
    row: &'a R,
    len: usize,
    depth: usize,
}

type Out = Result<Option<Num>, FormulaError>;

fn combine(a: Option<Num>, b: Option<Num>, op: char) -> Out {
    let (Some(a), Some(b)) = (a, b) else {
        return Ok(None);
    };
    let currency = match (&a.currency, &b.currency) {
        (Some(x), Some(y)) if x != y => return Err(FormulaError::Currency),
        (Some(x), _) | (_, Some(x)) => Some(x.clone()),
        _ => None,
    };
    let per_year = a.per_year.or(b.per_year);
    let value = match op {
        '+' => a.value + b.value,
        '-' => a.value - b.value,
        '*' => a.value * b.value,
        '/' => {
            if b.value == 0.0 {
                return Err(FormulaError::DivZero);
            }
            a.value / b.value
        }
        _ => unreachable!(),
    };
    Ok(Some(Num {
        value,
        currency,
        per_year,
    }))
}

impl<R: Row> Parser<'_, R> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.1)
    }
    fn at(&self) -> usize {
        self.toks.get(self.pos).map_or(self.len, |t| t.0)
    }
    fn expect(&mut self, c: char) -> Result<(), FormulaError> {
        if self.peek() == Some(&Tok::Op(c)) {
            self.pos += 1;
            Ok(())
        } else {
            Err(FormulaError::Syntax(self.at()))
        }
    }
    fn expr(&mut self) -> Out {
        let mut v = self.term()?;
        while let Some(Tok::Op(c @ ('+' | '-'))) = self.peek().cloned() {
            self.pos += 1;
            let r = self.term()?;
            v = combine(v, r, c)?;
        }
        Ok(v)
    }
    fn term(&mut self) -> Out {
        let mut v = self.factor()?;
        while let Some(Tok::Op(c @ ('*' | '/'))) = self.peek().cloned() {
            self.pos += 1;
            let r = self.factor()?;
            v = combine(v, r, c)?;
        }
        Ok(v)
    }
    fn args(&mut self) -> Result<Vec<Option<Num>>, FormulaError> {
        self.expect('(')?;
        let mut a = vec![self.expr()?];
        while self.peek() == Some(&Tok::Op(',')) {
            self.pos += 1;
            a.push(self.expr()?);
        }
        self.expect(')')?;
        Ok(a)
    }
    fn factor(&mut self) -> Out {
        self.depth += 1;
        let v = if self.depth > MAX_DEPTH {
            Err(FormulaError::TooDeep)
        } else {
            self.factor_inner()
        };
        self.depth -= 1;
        v
    }
    fn factor_inner(&mut self) -> Out {
        let at = self.at();
        match self.toks.get(self.pos).map(|t| t.1.clone()) {
            Some(Tok::Num(n)) => {
                self.pos += 1;
                Ok(Some(Num::plain(n)))
            }
            Some(Tok::Col(c)) => {
                self.pos += 1;
                self.row.get(&c)
            }
            Some(Tok::Op('(')) => {
                self.pos += 1;
                let v = self.expr()?;
                self.expect(')')?;
                Ok(v)
            }
            Some(Tok::Op('-')) => {
                self.pos += 1;
                Ok(self.factor()?.map(|mut n| {
                    n.value = -n.value;
                    n
                }))
            }
            Some(Tok::Ident(f)) => {
                self.pos += 1;
                let a = self.args()?;
                let arity = |n: usize| {
                    if a.len() == n {
                        Ok(())
                    } else {
                        Err(FormulaError::Syntax(at))
                    }
                };
                match f.as_str() {
                    "yearly" | "monthly" => {
                        arity(1)?;
                        Ok(a[0].clone().map(|n| {
                            let py = n.per_year.unwrap_or(1.0);
                            let yearly = n.value * py;
                            let value = if f == "yearly" { yearly } else { yearly / 12.0 };
                            Num {
                                value,
                                currency: n.currency,
                                per_year: None,
                            }
                        }))
                    }
                    "default" => {
                        arity(2)?;
                        Ok(a[0].clone().or_else(|| a[1].clone()))
                    }
                    "round" => {
                        arity(1)?;
                        Ok(a[0].clone().map(|mut n| {
                            // Nudge by 1e-9 so 3.015 (stored as 3.01499…) rounds like a person would.
                            n.value = (n.value * 100.0 + n.value.signum() * 1e-9).round() / 100.0;
                            n
                        }))
                    }
                    "min" | "max" => {
                        arity(2)?;
                        match (&a[0], &a[1]) {
                            (Some(x), Some(y)) => {
                                let pick = if (f == "min") == (x.value <= y.value) {
                                    x
                                } else {
                                    y
                                };
                                Ok(Some(pick.clone()))
                            }
                            _ => Ok(None),
                        }
                    }
                    _ => Err(FormulaError::UnknownFunction(f)),
                }
            }
            _ => Err(FormulaError::Syntax(at)),
        }
    }
}

/// Evaluates `expr` against a row. `Ok(None)` when an input cell is empty.
pub fn eval<R: Row>(expr: &str, row: &R) -> Out {
    let toks = lex(expr)?;
    let mut p = Parser {
        toks,
        pos: 0,
        row,
        len: expr.len(),
        depth: 0,
    };
    let v = p.expr()?;
    if p.pos != p.toks.len() {
        return Err(FormulaError::Syntax(p.at()));
    }
    Ok(v)
}

/// Checks syntax and column names without values (for the column editor).
pub fn validate(expr: &str, known_columns: &[&str]) -> Result<(), FormulaError> {
    struct Probe<'a>(&'a [&'a str]);
    impl Row for Probe<'_> {
        fn get(&self, c: &str) -> Result<Option<Num>, FormulaError> {
            if self
                .0
                .iter()
                .any(|k| crate::text::fold(k) == crate::text::fold(c))
            {
                Ok(Some(Num::plain(1.0)))
            } else {
                Err(FormulaError::UnknownColumn(c.to_owned()))
            }
        }
    }
    eval(expr, &Probe(known_columns)).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn deep_nesting_is_refused_not_a_crash() {
        let row = M(HashMap::new());
        let deep = format!("{}1{}", "(".repeat(100_000), ")".repeat(100_000));
        assert!(matches!(eval(&deep, &row), Err(FormulaError::TooDeep)));
        assert!(matches!(
            eval(&format!("{}1", "-".repeat(100_000)), &row),
            Err(FormulaError::TooDeep)
        ));
        let ok = format!("{}1{}", "(".repeat(20), ")".repeat(20));
        assert_eq!(eval(&ok, &row).unwrap().unwrap().value, 1.0);
    }

    struct M(HashMap<&'static str, Num>);
    impl Row for M {
        fn get(&self, c: &str) -> Result<Option<Num>, FormulaError> {
            match c {
                "Missing" => Ok(None),
                _ => self
                    .0
                    .get(c)
                    .cloned()
                    .map(Some)
                    .ok_or_else(|| FormulaError::UnknownColumn(c.into())),
            }
        }
    }
    fn price(v: f64, py: f64) -> Num {
        Num {
            value: v,
            currency: Some("EUR".into()),
            per_year: Some(py),
        }
    }

    #[test]
    fn yearly_cost() {
        let r = M(HashMap::from([
            ("Price", price(15.0, 12.0)),
            ("Licenses", Num::plain(4.0)),
        ]));
        let v = eval("yearly([Price]) * default([Licenses], 1)", &r)
            .unwrap()
            .unwrap();
        assert_eq!((v.value, v.currency.as_deref()), (720.0, Some("EUR")));
        let r2 = M(HashMap::from([("Price", price(110.0, 1.0))]));
        assert_eq!(
            eval("yearly([Price]) * default([Missing], 1)", &r2)
                .unwrap()
                .unwrap()
                .value,
            110.0
        );
    }

    #[test]
    fn arithmetic_and_errors() {
        let r = M(HashMap::new());
        assert_eq!(eval("1 + 2 * (3 - 1) / 4", &r).unwrap().unwrap().value, 2.0);
        assert_eq!(eval("-2 * 3", &r).unwrap().unwrap().value, -6.0);
        assert_eq!(eval("[Missing] + 1", &r).unwrap(), None);
        assert_eq!(eval("1 / 0", &r), Err(FormulaError::DivZero));
        assert!(matches!(eval("1 +", &r), Err(FormulaError::Syntax(_))));
        assert!(matches!(
            eval("foo(1)", &r),
            Err(FormulaError::UnknownFunction(_))
        ));
        assert!(matches!(
            eval("[Nope]", &r),
            Err(FormulaError::UnknownColumn(_))
        ));
        assert_eq!(eval("max(2, 5)", &r).unwrap().unwrap().value, 5.0);
        assert_eq!(eval("round(1.005 * 3)", &r).unwrap().unwrap().value, 3.02);
        let usd = Num {
            value: 1.0,
            currency: Some("USD".into()),
            per_year: None,
        };
        let r3 = M(HashMap::from([("A", price(1.0, 1.0)), ("B", usd)]));
        assert_eq!(eval("[A] + [B]", &r3), Err(FormulaError::Currency));
    }

    #[test]
    fn validation() {
        assert!(validate("yearly([Price]) * [licenses]", &["Price", "Licenses"]).is_ok());
        assert!(validate("[Prix]", &["Price"]).is_err());
    }
}
