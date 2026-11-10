#[allow(dead_code)]
mod app;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::export_designs(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ReCreation/skins/redox"),
    )
}
