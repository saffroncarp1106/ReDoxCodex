fn main() {
    println!("cargo:rerun-if-changed=icon.ico");
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        winresource::WindowsResource::new()
            .set_icon("icon.ico")
            .compile()
            .expect("could not embed icon.ico in the Windows executable");
    }
}
