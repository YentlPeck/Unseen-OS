//! Every example in docs/script-reference.md is a test (design §10):
//! - a `script` block must give exactly the `output` block after it, or a mistake containing
//!   the `mistake` block's text;
//! - a `tests` block is run as a test page and must give exactly the `report` block after it;
//! - a `page NAME` block puts that page on the examples' desk (for `use`).
extern crate alloc;

#[path = "../../kernel/src/fs.rs"]
#[allow(dead_code)]
mod fs;
mod common;

use common::TestWorld;

/// (kind, text) of every fenced block, in order.
fn blocks(md: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut lines = md.lines();
    while let Some(l) = lines.next() {
        if let Some(kind) = l.strip_prefix("```") {
            if kind.is_empty() {
                continue;
            }
            let mut body = Vec::new();
            for b in lines.by_ref() {
                if b == "```" {
                    break;
                }
                body.push(b);
            }
            out.push((kind.to_string(), body.join("\n")));
        }
    }
    out
}

#[test]
fn every_reference_example_gives_its_result() {
    let md = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/script-reference.md")).unwrap();
    let b = blocks(&md);
    let mut start = String::from(common::START);
    for (kind, text) in &b {
        if let Some(name) = kind.strip_prefix("page ") {
            start.push_str(&format!("#page {name}\n{text}\n#end\n"));
        }
    }
    let world = TestWorld::from(&start);
    let mut checked = 0;
    for (i, (kind, src)) in b.iter().enumerate() {
        if kind == "lab" {
            // typed lines start with the lab's prompt (">   ", or "... " inside a block); the
            // other lines are what the lab shows. The whole screen must come out the same.
            let mut lab = script::Lab::new();
            let mut screen = Vec::new();
            for l in src.lines() {
                let Some(typed) = l.strip_prefix(">   ").or_else(|| l.strip_prefix("... ")) else { continue };
                let prompt = if lab.in_block() { "... " } else { ">   " };
                screen.push(format!("{prompt}{typed}"));
                screen.extend(lab.line(typed, &world));
            }
            assert_eq!(screen.join("\n"), *src, "the lab shows something else");
            checked += 1;
            continue;
        }
        if kind != "script" && kind != "tests" {
            continue;
        }
        let (want_kind, want) = b.get(i + 1).expect("an example needs an output, mistake or report block after it");
        if kind == "tests" {
            assert_eq!(want_kind, "report", "a tests block needs a report block after it");
            let r = script::run_tests("tests", src, &world);
            assert_eq!(r.lines().join("\n"), *want, "wrong report for:\n{src}");
            checked += 1;
            continue;
        }
        let o = script::run("main", src, &world);
        match want_kind.as_str() {
            "output" => {
                assert!(o.error.is_none(), "example failed: {:?}\n{src}", o.error.map(|e| e.to_string()));
                assert_eq!(o.output.join("\n"), *want, "wrong output for:\n{src}");
            }
            "mistake" => {
                let e = o.error.unwrap_or_else(|| panic!("example should fail:\n{src}")).to_string();
                assert!(e.contains(want.as_str()), "mistake {e:?} should contain {want:?} for:\n{src}");
            }
            k => panic!("unknown block kind {k} after a script block"),
        }
        checked += 1;
    }
    assert!(checked >= 30, "only {checked} examples");
}
