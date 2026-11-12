use std::{env, io};

fn main() -> io::Result<()> {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../icons/audexis.ico");
    winresource::WindowsResource::new()
        .set_icon("../../icons/audexis.ico")
        .set("ProductName", "Audexis")
        .set("FileDescription", "Audexis")
        .compile()
}
