//! Loading a program (design §4 "Modules"): the main page and every page it uses, each once,
//! the used pages first in the order of the `use` lines. A loop of pages is a mistake. A used
//! page may only hold `use`, `fixed` and `task` lines (its tests, if any, are not run).

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::limits::*;
use crate::parse::{parse, Program};
use crate::run::World;
use crate::util::{display, path_of};

/// A mistake found while loading: (page index into `pages`, line, message), with the pages
/// known so far (so the page can be named).
pub struct LoadErr {
    pub page: String,
    pub line: usize,
    pub message: String,
}

pub fn load<W: World>(main_page: &str, source: &str, world: &W) -> Result<Program, LoadErr> {
    let mut l = Loader { world, out: Program::default(), stack: Vec::new(), cost: 0 };
    l.out.pages.push(String::from(main_page));
    l.visit(0, source)?;
    Ok(l.out)
}

struct Loader<'a, W: World> {
    world: &'a W,
    out: Program,
    /// The pages being loaded, outermost first (to find loops).
    stack: Vec<usize>,
    cost: usize,
}

impl<'a, W: World> Loader<'a, W> {
    fn err(&self, page: usize, line: usize, message: String) -> LoadErr {
        LoadErr { page: self.out.pages[page].clone(), line, message }
    }

    fn visit(&mut self, page: usize, source: &str) -> Result<(), LoadErr> {
        let mut p = parse(source).map_err(|(line, m)| self.err(page, line, m))?;
        self.cost += p.cost;
        if self.cost > PROGRAM_MEMORY {
            return Err(self.err(page, 0, format!("the program is too big (at most {} KB in all)", PROGRAM_MEMORY / 1024)));
        }
        if page != 0 {
            if let Some(first) = p.main.first() {
                return Err(self.err(
                    page,
                    first.n,
                    "a page that is used may only hold use, fixed and task lines (nothing that runs by itself)".into(),
                ));
            }
        }
        self.stack.push(page);
        for (written, line) in core::mem::take(&mut p.uses) {
            let path = path_of(&written).map_err(|m| self.err(page, line, m))?;
            if let Some(&i) = self.stack.iter().find(|&&i| self.out.pages[i] == path) {
                let what = if i == page { "itself".into() } else { display(&self.out.pages[i]) };
                return Err(self.err(page, line, format!("{} uses {what}, which uses it: pages can't use each other in a loop", display(&path))));
            }
            if self.out.pages.contains(&path) {
                continue; // loaded already (two pages may use the same page)
            }
            if self.out.pages.len() >= MAX_PAGES {
                return Err(self.err(page, line, format!("a program can use at most {MAX_PAGES} pages")));
            }
            let Some(text) = self.world.page(&path) else {
                let m = if self.world.is_drawer(&path) {
                    format!("{} is a drawer, not a page", display(&path))
                } else {
                    format!("there is no page called {}", display(&path))
                };
                return Err(self.err(page, line, m));
            };
            self.out.pages.push(path);
            let i = self.out.pages.len() - 1;
            self.visit(i, &text)?;
        }
        self.stack.pop();
        for mut t in p.tasks {
            t.page = page;
            self.out.tasks.push(t);
        }
        for mut f in p.fixed {
            f.page = page;
            self.out.fixed.push(f);
        }
        if page == 0 {
            for mut t in p.tests {
                t.page = page;
                self.out.tests.push(t);
            }
            self.out.main = p.main;
        }
        self.out.cost = self.cost;
        Ok(())
    }
}
