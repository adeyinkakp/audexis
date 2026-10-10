use serde::Serialize;
use tauri::Manager;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFile {
    name: String,
    content: String,
}

#[tauri::command]
pub fn get_logs(app: tauri::AppHandle) -> Result<Vec<LogFile>, String> {
    let directory = app
        .path()
        .app_log_dir()
        .map_err(|error| error.to_string())?;
    let mut paths = std::fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "log"))
        .collect::<Vec<_>>();
    paths.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
    paths
        .into_iter()
        .map(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown.log")
                .to_string();
            let content = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
            Ok(LogFile { name, content })
        })
        .collect()
}
