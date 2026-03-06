fn main() {
    println!("cargo:rerun-if-changed=src/stella.lalrpop");
    // Process any .lalrpop files found under the crate root.
    lalrpop::process_root().unwrap();
}
