fn main() {
    // This will help us see the expanded macros
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/**/*.rs");
}
