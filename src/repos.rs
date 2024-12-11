use mpsc::channel;
use std::fmt::{Display, Formatter};
use std::os::windows::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::{self, fs, thread};

use path_absolutize::Absolutize;

use crate::dialogue::dialogue_ui::Dialogue;
use crate::dialogue::dialogue_ui::DialogueMessage;
use crate::dialogue::dialogue_ui::DialogueMessage::{Finish, ForceShutdown};
use crate::dialogue::dialogue_ui::{DialogueMessage::ItemsFound, DialogueMessage::ProgressUpdate};

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

pub fn find_project(path: String, new_tab: bool) -> Result<()> {
    let (tx, rx) = channel::<DialogueMessage<ProjectInfo>>();
    let search_sender = tx.clone();
    thread::spawn(move || {
        let path = Path::new(&path);

        let mut updater = Updater::new(&search_sender);
        let repos = get_project_paths(path, &mut updater);
        search_sender.send(Finish).unwrap();
        return repos;
    });

    let ctrlc_sender = tx.clone();
    ctrlc::set_handler(move || ctrlc_sender.send(ForceShutdown).unwrap())?;

    let mut selection = Dialogue::new(rx).prompt("Select repository").interact();

    if let Ok(Some(selected_repo)) = selection {
        let selected = &selected_repo.path;
        open_tab(selected, new_tab);
    }

    return Ok(());
}

fn path_to_project(p: &PathBuf) -> ProjectInfo {
    let details = get_repo_info(&p);
    let full_path = to_full_path(&p);
    return ProjectInfo {
        path: full_path,
        detailed_repo_info: details,
    };
}

fn open_tab(directory: &String, new_tab: bool) {
    if new_tab {
        print!("<#Execute#>wt -w 0 nt -d {:}", directory);
    } else {
        print!("<#Execute#>cd {:}", directory);
    }
    // std::process::Command::new("wt").args(["-w", "0", "nt", "-d", directory]).output().expect("failed to open new tab");
}

fn to_full_path(path: &PathBuf) -> String {
    let expanded = shellexpand::full(path.to_str().unwrap())
        .unwrap()
        .into_owned();
    let expanded_path = Path::new(&expanded);
    let canonical = expanded_path.absolutize();
    let full_path = canonical
        .unwrap()
        .into_owned()
        .into_os_string()
        .into_string()
        .unwrap();

    return full_path;
}

struct ProjectMarker {
    is_dir: bool,
    name: String,
    path: PathBuf,
}

fn is_valid_repository_marker(dir_entry: fs::DirEntry) -> Option<ProjectMarker> {
    match dir_entry.metadata() {
        Ok(metadata) => {
            const BANNED_ATTRS: u32 = 4 | 1024; // System | ReparsePoint
            let has_banned_attrs = metadata.file_attributes() & BANNED_ATTRS > 0;

            if has_banned_attrs {
                return None;
            }

            let name = dir_entry.file_name().to_string_lossy().to_string();
            let path = dir_entry.path();
            return Some(ProjectMarker {
                is_dir: metadata.is_dir(),
                name,
                path,
            });
        }
        Err(_) => None,
    }
}

fn get_project_paths(path: &std::path::Path, updater: &mut Updater) -> Vec<ProjectInfo> {
    let mut result = Vec::new();
    let mut traverse_queue = Vec::new();
    traverse_queue.push(PathBuf::from(path));

    while let Some(popped) = traverse_queue.pop() {
        updater.update_current(&popped);
        let read_dir_result = fs::read_dir(&popped);

        match read_dir_result {
            Ok(read_dir) => {
                let children_dirs = read_dir
                    .map(|d| d.unwrap())
                    .filter_map(|d| is_valid_repository_marker(d))
                    .collect::<Vec<_>>();

                let mut is_project = false;
                for child in &children_dirs {
                    if child.name == ".git" && child.is_dir {
                        is_project = true;
                        break;
                    }
                }

                if !is_project {
                    for child in &children_dirs {
                        if child.name.ends_with(".sln") {
                            is_project = true;
                            break;
                        }

                        if child.name.ends_with(".csproj") {
                            is_project = true;
                            break;
                        }
                    }
                }

                if is_project {
                    let repo = path_to_project(&popped);
                    updater.on_new_project(&repo);
                    result.push(repo);
                    continue;
                }

                for child in &children_dirs {
                    if child.name == "node_modules" && child.is_dir {
                        continue;
                    }

                    traverse_queue.push(child.path.clone());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => continue, // Its ok to skip directories we cant look at
            Err(e) => { 
                // TODO: log this thing
                // println!("Encoutnered unknown error: {}", e); 
                continue; 
            },
        }
    }

    result
}

#[derive(Clone)]
struct ProjectInfo {
    path: String,
    detailed_repo_info: Vec<DetailedRepoInfo>,
}

impl PartialEq<Self> for ProjectInfo {
    fn eq(&self, other: &Self) -> bool {
        self.path.eq(&other.path)
    }
}

impl Eq for ProjectInfo {}

impl Display for ProjectInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let emoji = match self.detailed_repo_info.first() {
            None => "",
            Some(DetailedRepoInfo::NpmProject) => " [js]",
            Some(DetailedRepoInfo::CsharpProject) => " [csharp]",
            Some(DetailedRepoInfo::GoProject) => " [go]",
            Some(DetailedRepoInfo::RustProject) => " [rust]",
            Some(DetailedRepoInfo::LuaProject) => " [lua]",
        };

        let display = self.path.clone() + emoji;
        return f.write_str(&display);
    }
}

#[derive(Clone)]
enum DetailedRepoInfo {
    CsharpProject,
    NpmProject,
    GoProject,
    RustProject,
    LuaProject,
}

fn get_repo_info(path: &PathBuf) -> Vec<DetailedRepoInfo> {
    let mut repos = Vec::new();

    let inner_items = path.read_dir().unwrap().filter(|f| f.is_ok());
    for item in inner_items {
        let unwrapped = item.unwrap();
        let os_file_name = unwrapped.file_name();
        let file_name = os_file_name.to_string_lossy();
        if file_name.ends_with(".sln") || file_name.ends_with(".csproj") {
            repos.push(DetailedRepoInfo::CsharpProject);
        }

        if file_name == "package.json" {
            repos.push(DetailedRepoInfo::NpmProject);
        }

        if file_name == "go.mod" {
            repos.push(DetailedRepoInfo::GoProject);
        }

        if file_name == "Cargo.toml" {
            repos.push(DetailedRepoInfo::RustProject);
        }

        if file_name.ends_with(".lua") || file_name == "lua" {
            repos.push(DetailedRepoInfo::LuaProject);
        }
    }



    return repos;
}

struct Updater<'a> {
    sender: &'a Sender<DialogueMessage<ProjectInfo>>,
    last_updated: Option<std::time::Instant>,
}

impl<'a> Updater<'a> {
    pub(crate) fn on_new_project(&self, repo: &ProjectInfo) {
        self.sender.send(ItemsFound(vec![repo.clone()])).unwrap()
    }

    fn update_current(&mut self, folder: &PathBuf) {
        if let Some(last_updated) = self.last_updated {
            let now = std::time::Instant::now();
            let delta = now - last_updated;
            if delta < std::time::Duration::from_millis(200) {
                return;
            }
        }

        let display_path = folder.to_str().unwrap();
        self.sender
            .send(ProgressUpdate(
                format!("Last found directory:{display_path}").into_boxed_str(),
            ))
            .unwrap();
        self.last_updated = Some(std::time::Instant::now());
    }

    fn new(spinner: &Sender<DialogueMessage<ProjectInfo>>) -> Updater {
        Updater {
            sender: spinner,
            last_updated: None,
        }
    }
}
