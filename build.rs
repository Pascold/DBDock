//! Embeds the application icon and version information into the Windows
//! executable.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon/dbdock.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("assets/icon/dbdock.ico")
        .set("ProductName", "DBDock")
        .set("FileDescription", "DBDock - test databases in Docker")
        .set("LegalCopyright", "MIT OR Apache-2.0");
    if let Err(e) = resource.compile() {
        // A missing resource compiler must not go unnoticed in release builds.
        panic!("cannot embed the Windows icon: {e}");
    }
}
