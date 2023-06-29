use std::env;
use std::fs;
use std::path::Path;

fn write_bundle_manifest(asset_type: &str) {
    let root_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root_dir = Path::new(&root_dir);
    let dir = root_dir.join(format!("static/{}", asset_type));

    let entries = fs::read_dir(&dir).unwrap();

    let bundles: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .ends_with(&format!(".{}", asset_type))
        })
        .map(|entry| {
            Path::new("/")
                .join(entry.path().strip_prefix(root_dir).unwrap())
                .display()
                .to_string()
        })
        .collect();

    fs::write(dir.join("manifest.txt"), bundles.join("\n")).unwrap();
}

fn main() {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=.frontend-built");

    write_bundle_manifest("js");
    write_bundle_manifest("css");
}
