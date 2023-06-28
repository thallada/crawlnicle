use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=.frontend-built");

    let root_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root_dir = Path::new(&root_dir);
    let dir = root_dir.join("static/js");

    let entries = fs::read_dir(&dir).unwrap();

    let js_bundles: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".js"))
        .map(|entry| {
            Path::new("/")
                .join(entry.path().strip_prefix(root_dir).unwrap())
                .display()
                .to_string()
        })
        .collect();

    fs::write(dir.join("js_bundles.txt"), js_bundles.join("\n")).unwrap();
}
