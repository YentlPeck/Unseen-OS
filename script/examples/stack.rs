// Smallest stack (KB) a deep task program needs: cargo run --release --example stack -- whens brackets depth
struct W;
impl script::World for W {
    fn page(&self, _: &str) -> Option<String> { None }
    fn is_drawer(&self, _: &str) -> bool { false }
    fn pages_in(&self, _: &str) -> Vec<String> { vec![] }
    fn check_write(&self, _: &str) -> Result<(), String> { Ok(()) }
    fn check_toss(&self, _: &str) -> Result<(), String> { Ok(()) }
    fn check_send(&self, _: &str, _: &str, _: &str) -> Result<(), String> { Ok(()) }
    fn check_event(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), String> { Ok(()) }
    fn today(&self) -> String { String::new() }
}
fn fits(src: &str, kb: usize) -> bool {
    let s = src.to_string();
    std::thread::Builder::new().stack_size(kb * 1024).spawn(move || {
        let o = script::run("main", &s, &W);
        if let Some(e) = o.error { eprintln!("mistake: {e}"); }
    }).unwrap().join().is_ok()
}
fn main() {
    let a: Vec<usize> = std::env::args().skip(1).map(|x| x.parse().unwrap()).collect();
    let (whens, brackets, depth, kb) = (a[0], a[1], a[2], a[3]);
    let src = format!("task down(n)
{}when n is 0
give 0
done
give {}down(n - 1){}
{}done
say down({depth})",
        "when yes
".repeat(whens), "(1 + ".repeat(brackets), ")".repeat(brackets), "done
".repeat(whens));
    if fits(&src, kb) { println!("fits"); }
}
