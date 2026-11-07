// Does this program run in KB of stack? cargo run --release --example stackfile -- file kb
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
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let kb: usize = a[2].parse().unwrap();
    let ok = std::thread::Builder::new().stack_size(kb * 1024).spawn(move || {
        let o = script::run("main", &src, &W);
        if let Some(e) = o.error { eprintln!("mistake: {}", e.message); }
    }).unwrap().join().is_ok();
    if ok { println!("fits"); }
}
