fn main() {
    println!("cargo:rerun-if-changed=grammar");
    // Process any .lalrpop files found under the crate root.
    lalrpop::process_root().unwrap();
}
