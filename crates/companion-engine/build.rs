//! Embed a Windows application manifest: Common Controls v6 (needed by the
//! tray menu / `muda`, which imports `TaskDialogIndirect`), per-monitor DPI
//! awareness, and UTF-8 as the active code page.

fn main() {
    #[cfg(windows)]
    {
        use embed_manifest::{embed_manifest, manifest::ActiveCodePage, new_manifest};
        if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
            let manifest =
                new_manifest("NayaOS.NayaCompanion").active_code_page(ActiveCodePage::Utf8);
            embed_manifest(manifest).expect("embedding the Windows manifest");
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}
