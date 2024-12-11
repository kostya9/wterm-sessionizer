use std::{io::Seek, sync::mpsc::channel};

use dirs;
use serde::{Deserialize, Serialize};
use serde_json;

use crate::dialogue::dialogue_ui::{Dialogue, DialogueMessage};

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

pub fn expand(path: &str) -> Result<()> {
    let default_cd_location = path.to_string();

    let target_path = std::env::current_dir()?.join(path);
    if target_path.exists() {
        execute_cd(&default_cd_location);
        return Ok(());
    }

    if let Ok(folders) = find_expanded_folder(path) {
        if folders.is_empty() {
            execute_cd(&default_cd_location);
            return Ok(());
        }

        if folders.len() == 1 {
            let folder = folders[0].clone();
            execute_cd(&folder);
            return Ok(());
        }

        let (tx, rx) = channel::<DialogueMessage<String>>();
        tx.send(DialogueMessage::ItemsFound(folders)).unwrap();
        let selection = Dialogue::new(rx).prompt("Select folder").interact();

        if let Ok(Some(selection)) = selection {
            execute_cd(&selection);
            return Ok(());
        }
        
    }

    execute_cd(&default_cd_location);

    return Ok(());
}

fn execute_cd(path: &str) {
    let cmd = format!("<#Execute#>cd \"{:}\"", path);
    println!("{}", cmd.to_string());
}

pub fn find_expanded_folder(path: &str) -> Result<Vec<String>> {
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

    let similar = dirs
        .iter()
        .filter(|d| d.dir.contains(&path))
        .map(|d| d.dir.clone());
    return Ok(similar.into_iter().collect());
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
