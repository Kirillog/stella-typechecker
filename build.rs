fn main() {
    println!("cargo:rerun-if-changed=grammar/parser.lalrpop");
    // Process the lalrpop grammar file in the grammar directory.
    let out_dir = std::env::var("OUT_DIR").unwrap();
    lalrpop::Configuration::new()
        .set_in_dir("grammar")
        .set_out_dir(&out_dir)
        .process()
        .unwrap();
}
