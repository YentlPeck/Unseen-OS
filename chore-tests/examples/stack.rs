use chore_tests::chore;
fn main() {
    // measure: smallest thread stack (KB) each program runs in
    let nest = format!("{}say 1\n{}", "when yes\n".repeat(16), "done\n".repeat(16));
    let tree = format!("say {}1{} & 1{}", "(".repeat(12), ")".repeat(12), " + 1".repeat(44));
    let both = format!("{}{tree}\n{}", "when yes\n".repeat(16), "done\n".repeat(16));
    let args: Vec<String> = std::env::args().collect();
    let want = args[1].clone();
    let kb: usize = args[2].parse().unwrap();
    for (name, src) in [("nest", nest), ("tree", tree), ("both", both)] {
        if name != want { continue; }
        {
            let s = src.clone();
            let ok = std::thread::Builder::new().stack_size(kb * 1024).spawn(move || {
                struct W;
                impl chore::World for W {
                    fn page(&self, _: &str) -> Option<String> { None }
                    fn is_drawer(&self, _: &str) -> bool { false }
                    fn pages_in(&self, _: &str) -> Vec<String> { vec![] }
                    fn check_write(&self, _: &str) -> Result<(), String> { Ok(()) }
                    fn check_toss(&self, _: &str) -> Result<(), String> { Ok(()) }
                    fn check_send(&self, _: &str, _: &str, _: &str) -> Result<(), String> { Ok(()) }
                    fn check_event(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), String> { Ok(()) }
                    fn today(&self) -> String { String::new() }
                }
                chore::run(&s, &W).error.is_none()
            }).unwrap().join();
            assert!(matches!(ok, Ok(true)));
        }
    }
}
