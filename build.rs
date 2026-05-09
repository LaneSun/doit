use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    let out_dir = Path::new("locales");
    fs::create_dir_all(out_dir).unwrap();

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

        let out_path = out_dir.join(lang);
        let mut out_file = fs::File::create(&out_path).unwrap();
        out_file.write_all(content.as_bytes()).unwrap();
    }

    // Rerun if any locale source changes
    println!("cargo:rerun-if-changed=src/locales");
    println!("cargo:rerun-if-changed=src/commands");
}
