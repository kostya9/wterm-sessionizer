use std::io::Seek;

use dirs;
use serde::{Deserialize, Serialize};
use serde_json;

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

pub fn expand(path: &str) -> Result<()> {
    let default_cmd = format!("<#Execute#>cd {:}", path);

    let target_path = std::env::current_dir()?.join(path);
    if target_path.exists() {
        println!("{}", default_cmd.to_string());
        return Ok(());
    }

    if let Ok(Some(folder)) = find_expanded_folder(path) {
        let cmd = format!("<#Execute#>cd {:}", folder);
        println!("{}", cmd.to_string());
    } else {
        println!("{}", default_cmd.to_string());
    }

    return Ok(());
}

pub fn find_expanded_folder(path: &str) -> Result<Option<String>> {
    let file_name = "directory_history.json";
    let app_name = "wterm-sessionizer";
    let app_folder = dirs::data_dir().unwrap().join(app_name);

    let settings = if !app_folder.exists() {
        DirectoryHistory {
            visited_dirs: Vec::new(),
        }
    } else {
        let settings_path = app_folder.join(file_name);

        let file = std::fs::OpenOptions::new().read(true).open(settings_path)?;
        let settings = serde_json::from_reader::<_, DirectoryHistory>(&file);
        settings.unwrap_or(DirectoryHistory {
            visited_dirs: Vec::new(),
        })
    };

    let mut dirs = settings.visited_dirs;
    dirs.sort_by(|a, b| b.times.partial_cmp(&a.times).unwrap());

    // Try to give priority to the last directory in the path
    // E.g. if the target path is Cloud, and the two directories are 
    // C:\CloudRepos\Hehe and c:\CloudRepos, we should prioritize the second one
    let mut similar_dir = dirs.iter().find(|d| {
        let last_part = std::path::Path::new(&d.dir).file_name();

        if let Some(last_part) = last_part {
            return last_part.to_string_lossy().contains(&path);
        }

        return false;
    });

    if similar_dir.is_none() {
        similar_dir = dirs.iter().find(|d| d.dir.contains(&path));
    }

    if let Some(dir) = similar_dir {
        return Ok(Some(dir.dir.clone()));
    }

    return Ok(None);
}

pub fn on_changed_directory(new_dir: &str) -> Result<()> {
    // append current dir name
    let full_path = std::env::current_dir()?;
    let new_full_path = full_path.join(new_dir);

    if !new_full_path.exists() {
        return Ok(());
    }

    let new_full_path = new_full_path.to_str().unwrap().to_string();

    let file_name = "directory_history.json";
    let app_name = "wterm-sessionizer";
    let app_folder = dirs::data_dir().unwrap().join(app_name);

    if !app_folder.exists() {
        std::fs::create_dir_all(&app_folder)?;
    }

    let settings_path = app_folder.join(file_name);

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(settings_path)?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let mut existing_data =
        serde_json::from_reader::<_, DirectoryHistory>(&file).unwrap_or(DirectoryHistory {
            visited_dirs: Vec::new(),
        });

    let existing_dir = existing_data
        .visited_dirs
        .iter_mut()
        .find(|d| d.dir == new_full_path);

    if let Some(dir) = existing_dir {
        dir.last_accessed_timestamp = now;
        dir.times += 1;
    } else {
        existing_data.visited_dirs.push(VisitedDir {
            dir: new_full_path,
            last_accessed_timestamp: now,
            times: 1,
        });
    }

    let max_dirs = 100;
    if existing_data.visited_dirs.len() > max_dirs {
        existing_data
            .visited_dirs
            .sort_by(|a, b| b.times.partial_cmp(&a.times).unwrap());
        existing_data.visited_dirs.pop();
    }

    // overwrite the file with the new data
    file.seek(std::io::SeekFrom::Start(0))?;
    serde_json::to_writer(&file, &existing_data)?;
    let end = file.seek(std::io::SeekFrom::End(0))?;
    file.set_len(end)?;
    return Ok(());
}

#[derive(Serialize, Deserialize)]
struct DirectoryHistory {
    visited_dirs: Vec<VisitedDir>,
}

#[derive(Serialize, Deserialize)]
struct VisitedDir {
    dir: String,
    last_accessed_timestamp: u64,
    times: u32,
}
