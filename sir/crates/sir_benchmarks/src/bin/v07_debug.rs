use sir_lower::lower_function;
use sir_verify::Verifier;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ll_text = std::fs::read_to_string(&args[1]).unwrap();
    let fname = &args[2];
    match lower_function(&ll_text, fname) {
        Ok(func) => {
            let mut v = Verifier::new(&func);
            let ok = v.verify();
            println!("verified: {}", ok);
            for e in v.errors() {
                println!("  {:?}", e);
            }
 // dump node kinds
            for node in func.arena.iter() {
                let meta = format!("{:?}", node.metadata);
            let meta_s = if meta.contains("llvm") { format!(" META={}", meta) } else { String::new() };
            println!("  %{} {:?}{}", node.id.0, node.kind, meta_s);
            }
        }
        Err(e) => println!("lower failed: {}", e),
    }
}
