use sir_lower::lower_function;
use sir_verify::Verifier;
use std::fs;

fn main() {
    let ll = fs::read_to_string("../corpus/kernels.ll").unwrap();
    let funcs = sir_lower::list_functions(&ll);
    for name in &funcs {
        match lower_function(&ll, name) {
            Ok(func) => {
                let mut v = Verifier::new(&func);
                if !v.verify() {
                    println!("=== {} ===", name);
                    for e in v.errors() {
                        println!("  {:?}", e);
                    }
                }
            }
            Err(_) => {}
        }
    }
}
