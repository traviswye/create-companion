//! Embed a Windows application manifest: Common Controls v6 (needed by the
//! tray menu / `muda`, which imports `TaskDialogIndirect`), per-monitor DPI
//! awareness, and UTF-8 as the active code page.

fn main() {
    #[cfg(windows)]
    {
        use embed_manifest::{embed_manifest, manifest::ActiveCodePage, new_manifest};
        if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
            let manifest =
                new_manifest("CreateCompanion.Engine").active_code_page(ActiveCodePage::Utf8);
            embed_manifest(manifest).expect("embedding the Windows manifest");
            // Product name / description shown by Task Manager and file properties.
            let mut res = winresource::WindowsResource::new();
            res.set_icon("../../assets/icon.ico");
            res.set("ProductName", "Create Companion")
                .set("FileDescription", "Create Companion engine")
                .set("CompanyName", "Create Companion")
                .set("LegalCopyright", "MIT");
            if let Err(e) = res.compile() {
                println!("cargo:warning=version resource not embedded: {e}");
            }
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/icon.ico");
    println!("cargo:rerun-if-changed=../../assets/tray-32.rgba");
}
