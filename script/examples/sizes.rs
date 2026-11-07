fn main() {
    println!("Tok {} Expr {} Stmt {} Line {} Value {}", std::mem::size_of::<script::lex::Tok>(),
        std::mem::size_of::<script::parse::Expr>(), std::mem::size_of::<script::parse::Stmt>(),
        std::mem::size_of::<script::parse::Line>(), std::mem::size_of::<script::value::Value>());
}
