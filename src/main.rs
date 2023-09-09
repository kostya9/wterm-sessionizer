use std::path::{Path, PathBuf};
use std::{self, fs};
use std::os::windows::prelude::*;
use dialoguer::{console::Term, theme::ColorfulTheme, FuzzySelect};
use indicatif::{ProgressBar, ProgressStyle};
use path_absolutize::Absolutize;


fn main() -> std::io::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let default_dir = ".".to_string();
    let dir = args.get(1).unwrap_or(&default_dir);
    let path = std::path::Path::new(dir);

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::with_template("[{elapsed_precise}] {spinner}\n{msg}").unwrap().tick_strings(&["Searching", "Searching.","Searching..", "Searching...", ""]));
    let mut updater = Updater::new(&spinner);
    let repo_paths = get_repository_paths(path, &mut updater);
    let repos = repo_paths.into_iter().map(|p| {
        let details = get_repo_info(&p);
        return RepoInfo {
            path: p,
            detailed_repo_info: details
        }
    }).collect::<Vec<_>>();


    spinner.finish();

    if repos.len() == 0 {
        println!("Couldnt find repos in the current directory");
        return Ok(());
    }

    let full_paths_repos = repos.iter().map(|a| {
        let expanded = shellexpand::full(a.path.to_str().unwrap()).unwrap().into_owned();
        let expanded_path = Path::new(&expanded);
        let canonical = expanded_path.absolutize();
        let display_path = canonical.unwrap().into_owned().into_os_string().into_string().unwrap();
        let emoji = match a.detailed_repo_info.first() {
            None => "",
            Some(DetailedRepoInfo::NpmProject) => " [js]",
            Some(DetailedRepoInfo::CsharpProject) => " [csharp]",
            Some(DetailedRepoInfo::GoProject) => " [go]"
        };

        return display_path + emoji;
    }).collect::<Vec<_>>();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .items(&full_paths_repos)
        .default(0)
        .with_prompt("Select repository")
        .interact_on_opt(&Term::stderr())?;

    if let Some(selected_idx) = selection {
        let selected = &full_paths_repos[selected_idx];
        println!("{selected:?}");
        open_tab(selected);
    }

    Ok(())
}

fn open_tab(directory: &String){
    std::process::Command::new("wt").args(["-w", "0", "nt", "-d", directory]).output().expect("failed to open new tab");
}

fn is_valid_repository_candidate(dir_entry: &fs::DirEntry) -> bool {
    match dir_entry.metadata() {
        Ok(metadata) => {
            const BANNED_ATTRS: u32 = 4 | 1024; // System | ReparsePoint
            let has_banned_attrs = metadata.file_attributes() & BANNED_ATTRS > 0;
            return !has_banned_attrs && metadata.is_dir();
        },
        Err(_) => false
    }
}

fn get_repository_paths(path: &std::path::Path, updater: &mut Updater) -> Vec<PathBuf> {
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
                    result.push(popped);
                    continue;
                }

                for child in &children_dirs {
                    if child.file_name() == "node_modules" {
                        continue;
                    }

                    traverse_queue.push(child.path());
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => continue, // Its ok to skip directories we cant look at
            Err(e) => panic!("Encoutnered unknown error: {}", e)
        }
    }

    result
}

struct RepoInfo {
    path: PathBuf,
    detailed_repo_info: Vec<DetailedRepoInfo>
}

enum DetailedRepoInfo {
    CsharpProject,
    NpmProject,
    GoProject,
}

fn get_repo_info(path: &std::path::PathBuf) -> Vec<DetailedRepoInfo> {
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
    }

    return repos;

}

struct Updater<'a> { bar: &'a ProgressBar, last_updated: Option<std::time::Instant> }

impl<'a> Updater<'a> {
    fn update_current(&mut self, string: &std::path::PathBuf) {
        if let Some(last_updated) = self.last_updated {
            let now = std::time::Instant::now();
            let delta = now - last_updated;
            if delta < std::time::Duration::from_millis(200) {
                return;
            }
        }

        self.bar.tick();
        self.bar.set_message(string.clone().into_os_string().into_string().unwrap());
        self.last_updated = Some(std::time::Instant::now());
    }

    fn new(spinner: &ProgressBar) -> Updater {
        Updater { bar: spinner, last_updated: None }
    }
}
