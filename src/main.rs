use mpsc::channel;
use std::{self, fs, thread};
use std::fmt::{Display, Formatter};
use std::os::windows::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::mpsc::Sender;

use clap::{Parser, Subcommand};
use path_absolutize::Absolutize;

use crate::dialogue::dialogue_ui::Dialogue;
use crate::dialogue::dialogue_ui::DialogueMessage;
use crate::dialogue::dialogue_ui::{DialogueMessage::ItemsFound, DialogueMessage::ProgressUpdate};
use crate::dialogue::dialogue_ui::DialogueMessage::{Finish, ForceShutdown};

mod dialogue;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    // Make this command default
    #[clap(flatten)]
    find_project: FindProjectArgs,
}

#[derive(Subcommand, Debug)]
enum Commands {
    FindProject(FindProjectArgs),
    OnChangedDirectory { path: String },
    Init {},
}

#[derive(Debug, clap::Args)]
struct FindProjectArgs {
    #[arg(default_value = ".")]
    path: String,

    #[arg(short, long)]
    new_tab: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    return match cli
        .command
        .unwrap_or(Commands::FindProject(cli.find_project))
    {
        Commands::FindProject(FindProjectArgs { path, new_tab }) => {
            return find_project(path, new_tab);
        }
        Commands::Init {} => {
            let content = include_str!("init.ps1");
            print!("{:}", content);
            return Ok(());
        }

        _ => Ok(()),
    };
}

fn find_project(path: String, new_tab: bool) -> Result<()> {
    let (tx, rx) = channel::<DialogueMessage<RepoInfo>>();
    let search_sender = tx.clone();
    thread::spawn(move || {
        let path = Path::new(&path);

        let mut updater = Updater::new(&search_sender);
        let repos = get_repository_paths(path, &mut updater);
        search_sender.send(Finish).unwrap();
        return repos;
    });

    let ctrlc_sender = tx.clone();
    ctrlc::set_handler(move || ctrlc_sender.send(ForceShutdown).unwrap())?;

    let mut selection = Dialogue::new(rx)
        .prompt("Select repository")
        .interact();

    if let Ok(Some(selected_repo)) = selection {
        let selected = &selected_repo.path;
        open_tab(selected, new_tab);
    }

    return Ok(());
}

fn path_to_repo(p: &PathBuf) -> RepoInfo {
    let details = get_repo_info(&p);
    let full_path = to_full_path(&p);
    return RepoInfo {
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

fn is_valid_repository_candidate(dir_entry: &fs::DirEntry) -> bool {
    match dir_entry.metadata() {
        Ok(metadata) => {
            const BANNED_ATTRS: u32 = 4 | 1024; // System | ReparsePoint
            let has_banned_attrs = metadata.file_attributes() & BANNED_ATTRS > 0;
            return !has_banned_attrs && metadata.is_dir();
        }
        Err(_) => false,
    }
}

fn get_repository_paths(path: &std::path::Path, updater: &mut Updater) -> Vec<RepoInfo> {
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
                    .filter(is_valid_repository_candidate)
                    .collect::<Vec<_>>();

                let mut is_repo = false;
                for child in &children_dirs {
                    if child.file_name() == ".git" {
                        is_repo = true;
                        break;
                    }
                }

                if is_repo {
                    let repo = path_to_repo(&popped);
                    updater.on_new_repo(&repo);
                    result.push(repo);
                    continue;
                }

                for child in &children_dirs {
                    if child.file_name() == "node_modules" {
                        continue;
                    }

                    traverse_queue.push(child.path());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => continue, // Its ok to skip directories we cant look at
            Err(e) => panic!("Encoutnered unknown error: {}", e),
        }
    }

    result
}


#[derive(Clone)]
struct RepoInfo {
    path: String,
    detailed_repo_info: Vec<DetailedRepoInfo>,
}

impl PartialEq<Self> for RepoInfo {
    fn eq(&self, other: &Self) -> bool {
        self.path.eq(&other.path)
    }
}

impl Eq for RepoInfo {}

impl Display for RepoInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let emoji = match self.detailed_repo_info.first() {
            None => "",
            Some(DetailedRepoInfo::NpmProject) => " [js]",
            Some(DetailedRepoInfo::CsharpProject) => " [csharp]",
            Some(DetailedRepoInfo::GoProject) => " [go]",
            Some(DetailedRepoInfo::RustProject) => " [rust]",
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
}

fn get_repo_info(path: &PathBuf) -> Vec<DetailedRepoInfo> {
    let mut repos = Vec::new();

    let inner_items = path.read_dir().unwrap().filter(|f| f.is_ok());
    for item in inner_items {
        let unwrapped = item.unwrap();
        let os_file_name = unwrapped.file_name();
        let file_name = os_file_name.to_string_lossy();
        if file_name.ends_with(".sln") {
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
    }

    return repos;
}

struct Updater<'a> {
    sender: &'a Sender<DialogueMessage<RepoInfo>>,
    last_updated: Option<std::time::Instant>,
}

impl<'a> Updater<'a> {
    pub(crate) fn on_new_repo(&self, repo: &RepoInfo) {
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

        let displayPath = folder.to_str().unwrap();
        self.sender.send(ProgressUpdate(format!("Last found directory:{displayPath}").into_boxed_str())).unwrap();
        self.last_updated = Some(std::time::Instant::now());
    }

    fn new(spinner: &Sender<DialogueMessage<RepoInfo>>) -> Updater {
        Updater {
            sender: spinner,
            last_updated: None,
        }
    }
}
