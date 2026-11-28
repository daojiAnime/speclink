#[path = "src/git_rev.rs"]
mod git_rev;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/git_rev.rs");

    let rev = std::env::var_os("CARGO_MANIFEST_DIR")
        .and_then(|dir| {
            git_rev::resolve_for_manifest(std::path::Path::new(&dir), &mut |path| {
                if let Some(display) = path.to_str() {
                    println!("cargo:rerun-if-changed={display}");
                }
            })
        })
        .unwrap_or_default();
    println!("cargo:rustc-env=SPECLINK_GIT_REV={rev}");
}
