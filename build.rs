//! Embeds every file under templates/ into the binary, as TEMPLATES: (path, text).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir).expect("templates/").flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=templates");
    let root = Path::new("templates");
    let mut files = Vec::new();
    walk(root, &mut files);
    let mut code = String::from("pub static TEMPLATES: &[(&str, &str)] = &[\n");
    for file in files {
        let rel = file.strip_prefix(root).expect("under templates").to_string_lossy().replace('\\', "/");
        let _ = writeln!(code, "    ({rel:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/templates/{rel}\"))),");
    }
    code.push_str("];\n");
    fs::write(Path::new(&env::var("OUT_DIR").expect("OUT_DIR")).join("templates.rs"), code).expect("write templates.rs");
}
