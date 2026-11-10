// SPDX-License-Identifier: AGPL-3.0-only
//! Mesin ekspresi indikator sederhana.
//! Tidak ada eval/exec — parser dan evaluator ditulis sendiri.
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Num(f64),
    Var(String),
    BinOp { op: char, lhs: Box<Expr>, rhs: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
}

#[derive(Debug, PartialEq)]
pub enum EvalError {
    UnknownVar(String),
    UnknownFn(String),
    WrongArgCount { func: String, expected: usize, got: usize },
    ParseError(String),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EvalError::UnknownVar(v) => write!(f, "Variabel tidak dikenal: {v}"),
            EvalError::UnknownFn(n) => write!(f, "Fungsi tidak dikenal: {n}"),
            EvalError::WrongArgCount { func, expected, got } =>
                write!(f, "Fungsi {func} butuh {expected} argumen, dapat {got}"),
            EvalError::ParseError(s) => write!(f, "Kesalahan parse: {s}"),
        }
    }
}

pub type Context = HashMap<String, f64>;

pub fn eval(expr: &Expr, ctx: &Context) -> Result<Option<f64>, EvalError> {
    match expr {
        Expr::Num(n) => Ok(Some(*n)),
        Expr::Var(name) => ctx.get(name).copied().map(Some)
            .ok_or_else(|| EvalError::UnknownVar(name.clone())),
        Expr::BinOp { op, lhs, rhs } => {
            let l = eval(lhs, ctx)?;
            let r = eval(rhs, ctx)?;
            match (l, r) {
                (Some(a), Some(b)) => Ok(Some(match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    '/' => if b == 0.0 { return Ok(None); } else { a / b },
                    _ => return Err(EvalError::ParseError(format!("Operator tidak dikenal: {op}"))),
                })),
                _ => Ok(None),
            }
        }
        Expr::Call { name, args } => eval_call(name, args, ctx),
    }
}

fn eval_args(args: &[Expr], ctx: &Context) -> Result<Vec<Option<f64>>, EvalError> {
    args.iter().map(|a| eval(a, ctx)).collect()
}

fn eval_call(name: &str, args: &[Expr], ctx: &Context) -> Result<Option<f64>, EvalError> {
    match name {
        "sum" => {
            let vals: Result<Vec<_>, _> = args.iter().map(|a| eval(a, ctx)).collect();
            Ok(Some(vals?.iter().filter_map(|v| *v).sum()))
        }
        "avg" => {
            let vals: Result<Vec<_>, _> = args.iter().map(|a| eval(a, ctx)).collect();
            let nums: Vec<f64> = vals?.iter().filter_map(|v| *v).collect();
            if nums.is_empty() { Ok(None) } else { Ok(Some(nums.iter().sum::<f64>() / nums.len() as f64)) }
        }
        "count" => {
            let vals = eval_args(args, ctx)?;
            Ok(Some(vals.iter().filter(|v| v.is_some()).count() as f64))
        }
        "ratio" => {
            if args.len() != 2 {
                return Err(EvalError::WrongArgCount { func: "ratio".into(), expected: 2, got: args.len() });
            }
            let p = eval(&args[0], ctx)?;
            let q = eval(&args[1], ctx)?;
            match (p, q) {
                (Some(a), Some(b)) => Ok(if b == 0.0 { None } else { Some(a / b) }),
                _ => Ok(None),
            }
        }
        "if" => {
            if args.len() != 3 {
                return Err(EvalError::WrongArgCount { func: "if".into(), expected: 3, got: args.len() });
            }
            let cond = eval(&args[0], ctx)?;
            match cond {
                Some(v) if v != 0.0 => eval(&args[1], ctx),
                Some(_) => eval(&args[2], ctx),
                None => Ok(None),
            }
        }
        _ => Err(EvalError::UnknownFn(name.into())),
    }
}

struct Parser { chars: Vec<char>, pos: usize }

impl Parser {
    fn new(input: &str) -> Self { Self { chars: input.chars().collect(), pos: 0 } }
    fn peek(&self) -> Option<char> { self.chars.get(self.pos).copied() }
    fn next(&mut self) -> Option<char> { let c = self.peek(); self.pos += 1; c }
    fn skip_ws(&mut self) { while matches!(self.peek(), Some(c) if c.is_whitespace()) { self.next(); } }

    fn parse_expr(&mut self) -> Result<Expr, EvalError> { self.parse_add() }

    fn parse_add(&mut self) -> Result<Expr, EvalError> {
        let mut lhs = self.parse_mul()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('+') | Some('-') => {
                    let op = self.next().unwrap();
                    let rhs = self.parse_mul()?;
                    lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<Expr, EvalError> {
        let mut lhs = self.parse_atom()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('*') | Some('/') => {
                    let op = self.next().unwrap();
                    let rhs = self.parse_atom()?;
                    lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_atom(&mut self) -> Result<Expr, EvalError> {
        self.skip_ws();
        match self.peek() {
            Some('(') => { self.next(); let e = self.parse_expr()?; self.skip_ws(); self.next(); Ok(e) }
            Some(c) if c.is_ascii_digit() || c == '.' => self.parse_num(),
            Some(c) if c.is_alphabetic() || c == '_' => self.parse_name(),
            other => Err(EvalError::ParseError(format!("Token tidak terduga: {other:?}"))),
        }
    }

    fn parse_num(&mut self) -> Result<Expr, EvalError> {
        let mut s = String::new();
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.') {
            s.push(self.next().unwrap());
        }
        s.parse::<f64>().map(Expr::Num)
            .map_err(|_| EvalError::ParseError(format!("Angka tidak valid: {s}")))
    }

    fn parse_name(&mut self) -> Result<Expr, EvalError> {
        let mut name = String::new();
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
            name.push(self.next().unwrap());
        }
        self.skip_ws();
        if self.peek() == Some('(') {
            self.next();
            let mut args = vec![];
            loop {
                self.skip_ws();
                if self.peek() == Some(')') { self.next(); break; }
                args.push(self.parse_expr()?);
                self.skip_ws();
                if self.peek() == Some(',') { self.next(); }
            }
            Ok(Expr::Call { name, args })
        } else {
            Ok(Expr::Var(name))
        }
    }
}

pub fn parse(input: &str) -> Result<Expr, EvalError> {
    let mut p = Parser::new(input);
    let e = p.parse_expr()?;
    p.skip_ws();
    if p.pos < p.chars.len() {
        Err(EvalError::ParseError(format!("Karakter tersisa: {}", &input[p.pos..])))
    } else {
        Ok(e)
    }
}

pub fn evaluate(formula: &str, ctx: &Context) -> Result<Option<f64>, EvalError> {
    let expr = parse(formula)?;
    eval(&expr, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ctx(pairs: &[(&str, f64)]) -> Context {
        pairs.iter().map(|(k,v)| (k.to_string(), *v)).collect()
    }
    #[test] fn test_literal()   { assert_eq!(evaluate("42", &ctx(&[])), Ok(Some(42.0))); }
    #[test] fn test_add()       { assert_eq!(evaluate("a + b", &ctx(&[("a",3.0),("b",4.0)])), Ok(Some(7.0))); }
    #[test] fn test_div_zero()  { assert_eq!(evaluate("a / b", &ctx(&[("a",5.0),("b",0.0)])), Ok(None)); }
    #[test] fn test_ratio_pct() { assert_eq!(evaluate("ratio(a, b) * 100", &ctx(&[("a",3.0),("b",4.0)])), Ok(Some(75.0))); }
    #[test] fn test_sum()       { assert_eq!(evaluate("sum(a, b, c)", &ctx(&[("a",1.0),("b",2.0),("c",3.0)])), Ok(Some(6.0))); }
    #[test] fn test_avg()       { assert_eq!(evaluate("avg(a, b)", &ctx(&[("a",10.0),("b",20.0)])), Ok(Some(15.0))); }
    #[test] fn test_if_true()   { assert_eq!(evaluate("if(a, b, c)", &ctx(&[("a",1.0),("b",99.0),("c",0.0)])), Ok(Some(99.0))); }
    #[test] fn test_unknown_var() { assert!(matches!(evaluate("x", &ctx(&[])), Err(EvalError::UnknownVar(_)))); }
    #[test] fn test_pkk_formula() {
        let c = ctx(&[("jumlah_kelompok_binaan", 15.0),("jumlah_kelompok_pkk", 20.0)]);
        let result = evaluate("ratio(jumlah_kelompok_binaan, jumlah_kelompok_pkk) * 100", &c);
        assert_eq!(result, Ok(Some(75.0)));
    }
}
