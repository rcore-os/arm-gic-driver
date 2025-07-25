use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-search={}", out_dir().display());

    println!("cargo::rustc-link-arg=-Tlink_test.x");
    println!("cargo::rustc-link-arg-tests=-no-pie");
    println!("cargo::rustc-link-arg-tests=-znostart-stop-gc");
    println!("cargo::rustc-link-arg-tests=-Map=target/kernel.map");
}

fn out_dir() -> PathBuf {
    PathBuf::from(std::env::var("OUT_DIR").unwrap())
}
