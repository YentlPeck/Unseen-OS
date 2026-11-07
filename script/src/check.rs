//! Checks made before anything runs (design §4 "Names", "Tasks", "Fixed values"): every task
//! that is called exists and gets the right number of inputs; no two tasks or fixed values have
//! the same name, on any page; none has the name of a built-in; nothing changes a fixed value;
//! fixed values only use what never looks at the world; and the test-only functions are only
//! used in tests.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::parse::{builtin, builtin_name, is_instruction_word, is_pure, is_test_only, Expr, Line, Pretend, Program, Stmt, Target};

/// A mistake found by the checker: (page index, line, message).
pub type CErr = (usize, usize, String);

pub fn check(p: &Program) -> Result<(), CErr> {
    let mut task_names: Vec<&str> = Vec::new();
    for t in &p.tasks {
        let at = |m: String| (t.page, t.line, m);
        if builtin(&t.name).is_some() {
            return Err(at(format!("a task can't be called {}: that is a built-in function", t.name)));
        }
        if is_instruction_word(&t.name) {
            return Err(at(format!("a task can't be called {}: that word starts an instruction", t.name)));
        }
        if task_names.contains(&t.name.as_str()) {
            return Err(at(format!("there are two tasks called {}", t.name)));
        }
        task_names.push(&t.name);
    }
    let mut fixed: Vec<&str> = Vec::new();
    for f in &p.fixed {
        let at = |m: String| (f.page, f.line, m);
        if builtin(&f.name).is_some() {
            return Err(at(format!("a fixed value can't be called {}: that is a built-in function", f.name)));
        }
        if fixed.contains(&f.name.as_str()) {
            return Err(at(format!("there are two fixed values called {}", f.name)));
        }
        if task_names.contains(&f.name.as_str()) {
            return Err(at(format!("{} is the name of a task, so it can't be a fixed value too", f.name)));
        }
        fixed_expr(&f.value, &fixed).map_err(at)?;
        fixed.push(&f.name);
    }
    for t in &p.tasks {
        for x in &t.params {
            if fixed.contains(&x.as_str()) {
                return Err((t.page, t.line, format!("the input {x} has the name of a fixed value")));
            }
        }
        Cx { p, fixed: &fixed, page: t.page, in_test: false }.lines(&t.body)?;
    }
    for t in &p.tests {
        Cx { p, fixed: &fixed, page: t.page, in_test: true }.lines(&t.body)?;
    }
    Cx { p, fixed: &fixed, page: 0, in_test: false }.lines(&p.main)
}

/// A fixed value may use numbers, texts, yes/no, lists, records, operators, earlier fixed
/// values and the built-ins that never look at the world.
fn fixed_expr(e: &Expr, earlier: &[&str]) -> Result<(), String> {
    match e {
        Expr::Var(n) if earlier.contains(&n.as_str()) => Ok(()),
        Expr::Var(n) => Err(format!(
            "a fixed value can only use earlier fixed values, and {n} is not one (fixed values are worked out before the program runs)"
        )),
        Expr::Task(n, _) => Err(format!("a fixed value can't call a task ({n})")),
        Expr::Call(b, args) => {
            if !is_pure(*b) {
                return Err(format!("a fixed value can't use {}(...): it looks at the pages", builtin_name(*b)));
            }
            args.iter().try_for_each(|a| fixed_expr(a, earlier))
        }
        Expr::Neg(x) | Expr::Not(x) | Expr::Field(x, _) => fixed_expr(x, earlier),
        Expr::Bin(_, l, r) => {
            fixed_expr(l, earlier)?;
            fixed_expr(r, earlier)
        }
        Expr::List(items) => items.iter().try_for_each(|a| fixed_expr(a, earlier)),
        Expr::Record(fields) => fields.iter().try_for_each(|(_, a)| fixed_expr(a, earlier)),
        Expr::Num(_) | Expr::Text(_) | Expr::Bool(_) => Ok(()),
    }
}

struct Cx<'a> {
    p: &'a Program,
    fixed: &'a [&'a str],
    page: usize,
    in_test: bool,
}

impl<'a> Cx<'a> {
    fn lines(&self, lines: &[Line]) -> Result<(), CErr> {
        for l in lines {
            let at = |e: String| (self.page, l.n, e);
            match &l.stmt {
                Stmt::Set(t, e) => {
                    self.target(t).map_err(at)?;
                    self.expr(e).map_err(at)?;
                }
                Stmt::AddTo(e, t) => {
                    self.expr(e).map_err(at)?;
                    self.target(t).map_err(at)?;
                }
                Stmt::Say(e) | Stmt::Give(e) | Stmt::Call(e) | Stmt::Fail(e) | Stmt::Check(e) | Stmt::Launch(e) | Stmt::Press(e) => {
                    self.expr(e).map_err(at)?
                }
                Stmt::Pretend(Pretend::Page(a, b)) => {
                    self.expr(a).map_err(at)?;
                    self.expr(b).map_err(at)?;
                }
                Stmt::Pretend(Pretend::Today(e)) => self.expr(e).map_err(at)?,
                Stmt::Stop | Stmt::Skip | Stmt::Quit => {}
                Stmt::When(branches, otherwise) => {
                    for (c, body) in branches {
                        self.expr(c).map_err(at)?;
                        self.lines(body)?;
                    }
                    self.lines(otherwise)?;
                }
                Stmt::ForEach(name, e, body) => {
                    self.not_fixed(name).map_err(at)?;
                    self.expr(e).map_err(at)?;
                    self.lines(body)?;
                }
                Stmt::Repeat(e, body) | Stmt::While(e, body) => {
                    self.expr(e).map_err(at)?;
                    self.lines(body)?;
                }
                Stmt::Try(body, name, handler) => {
                    self.not_fixed(name).map_err(at)?;
                    self.lines(body)?;
                    self.lines(handler)?;
                }
                Stmt::Do(_, args) => args.iter().try_for_each(|a| self.expr(a)).map_err(at)?,
            }
        }
        Ok(())
    }

    fn not_fixed(&self, name: &str) -> Result<(), String> {
        if self.fixed.contains(&name) {
            Err(format!("{name} is a fixed value: it can't be changed"))
        } else {
            Ok(())
        }
    }

    fn target(&self, t: &Target) -> Result<(), String> {
        self.not_fixed(&t.name)?;
        for p in &t.parts {
            if let crate::parse::Part::At(e) = p {
                self.expr(e)?;
            }
        }
        Ok(())
    }

    fn expr(&self, e: &Expr) -> Result<(), String> {
        match e {
            Expr::Task(name, args) => {
                let Some(t) = self.p.tasks.iter().find(|t| t.name == *name) else {
                    return Err(format!("there is no task or built-in called {name}(...)"));
                };
                if t.params.len() != args.len() {
                    let s = if t.params.len() == 1 { "" } else { "s" };
                    return Err(format!(
                        "{name}(...) needs {} value{s} in its brackets, got {}",
                        t.params.len(),
                        args.len()
                    ));
                }
                args.iter().try_for_each(|a| self.expr(a))
            }
            Expr::Call(b, args) => {
                if is_test_only(*b) && !self.in_test {
                    return Err(format!("{}(...) only works inside a test", builtin_name(*b)));
                }
                args.iter().try_for_each(|a| self.expr(a))
            }
            Expr::List(args) => args.iter().try_for_each(|a| self.expr(a)),
            Expr::Record(fields) => fields.iter().try_for_each(|(_, a)| self.expr(a)),
            Expr::Neg(x) | Expr::Not(x) | Expr::Field(x, _) => self.expr(x),
            Expr::Bin(_, l, r) => {
                self.expr(l)?;
                self.expr(r)
            }
            Expr::Num(_) | Expr::Text(_) | Expr::Bool(_) | Expr::Var(_) => Ok(()),
        }
    }
}
