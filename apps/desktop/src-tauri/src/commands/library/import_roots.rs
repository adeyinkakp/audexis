use std::env;

use rfd::FileDialog;

#[tauri::command]
pub fn import_roots() -> Vec<String> {
    let home_dir: String = match env::home_dir() {
        Some(fp) => String::from(fp.to_string_lossy()),
        None => ".".to_owned(),
    };

    FileDialog::new()
        .set_title("Select files or folders")
        .set_directory(home_dir)
        .pick_folders()
        .unwrap_or_default()
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}
