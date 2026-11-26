//! Windows: puts the Cleis icon in the executable as resource 1, the one GPUI loads for its
//! windows (taskbar, Alt+Tab) and Explorer shows for the file.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let ico = dir.join("../../packaging/windows/cleis.ico");
    println!("cargo:rerun-if-changed={}", ico.display());
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let rc = out.join("cleis.rc");
    let path = ico.display().to_string().replace('\\', "\\\\");
    std::fs::write(&rc, format!("1 ICON \"{path}\"\n")).unwrap();
    embed_resource::compile(&rc, embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
