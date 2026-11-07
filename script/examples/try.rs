// Run a program from a file on an empty world: cargo run --example try -- file
struct W;
impl script::World for W {
    fn page(&self, _: &str) -> Option<String> { None }
    fn is_drawer(&self, p: &str) -> bool { p.is_empty() }
    fn pages_in(&self, _: &str) -> Vec<String> { vec![] }
    fn check_write(&self, _: &str) -> Result<(), String> { Ok(()) }
    fn check_toss(&self, _: &str) -> Result<(), String> { Ok(()) }
    fn check_send(&self, _: &str, _: &str, _: &str) -> Result<(), String> { Ok(()) }
    fn check_event(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), String> { Ok(()) }
    fn today(&self) -> String { "2026-11-14".into() }
}
fn main() {
    let src = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let o = script::run("main", &src, &W);
    for l in &o.output { println!("{l}"); }
    println!("-- steps {} peak {} error {:?}", o.steps, o.peak_memory, o.error.map(|e| e.to_string()));
}
