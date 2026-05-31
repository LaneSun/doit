use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_base = Path::new(&out_dir);
    let locales_dir = out_base.join("locales");
    fs::create_dir_all(&locales_dir).unwrap();

    let source_base = Path::new("src/locales");
    let commands_dir = Path::new("src/commands");

    for entry in fs::read_dir(source_base).unwrap() {
        let entry = entry.unwrap();
        let lang_file = entry.file_name();
        let lang = lang_file.to_str().unwrap();

        let mut content = fs::read_to_string(source_base.join(lang)).unwrap();

        // Collect per-command locale files
        if let Ok(cmd_dirs) = fs::read_dir(commands_dir) {
            for cmd_entry in cmd_dirs {
                let cmd_entry = cmd_entry.unwrap();
                let locales_path = cmd_entry.path().join("locales").join(lang);
                if locales_path.exists() {
                    let cmd_content = fs::read_to_string(&locales_path).unwrap();
                    content.push('\n');
                    content.push_str(&cmd_content);
                }
            }
        }

        let out_path = locales_dir.join(lang);
        let mut out_file = fs::File::create(&out_path).unwrap();
        out_file.write_all(content.as_bytes()).unwrap();
    }

    // Generate i18n loader that points to OUT_DIR locales
    let loader_code = format!("rust_i18n::i18n!(\"{}/locales\");\n", out_base.display());
    fs::write(out_base.join("i18n_loader.rs"), loader_code).unwrap();

    // Rerun if any locale source changes
    println!("cargo:rerun-if-changed=src/locales");
    println!("cargo:rerun-if-changed=src/commands");
}
