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
    let repos = get_repositories(path, &mut updater);

    spinner.finish();

    if repos.len() == 0 {
        println!("Couldnt find repos in the current directory");
        return Ok(());
    }

    let full_paths_repos = repos.iter().map(|a| {
        let expanded = shellexpand::full(a.to_str().unwrap()).unwrap().into_owned();
        let expanded_path = Path::new(&expanded);
        let canonical = expanded_path.absolutize();
        canonical.unwrap().into_owned().into_os_string().into_string().unwrap()
    })
    .collect::<Vec<_>>();
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .items(&full_paths_repos)
        .default(0)
        .with_prompt("Select repository")
        .interact_on_opt(&Term::stderr())?;

    if let Some(selected_idx) = selection {
        let selected = &full_paths_repos[selected_idx];
        println!("{selected:?}");
        std::process::Command::new("wt").args(["-w", "0", "nt", "-d", selected]).output().expect("failed to open new tab");

    }

    Ok(())
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

fn get_repositories(path: &std::path::Path, updater: &mut Updater) -> Vec<PathBuf> {
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

struct Updater<'a> { bar: &'a ProgressBar, last_updated: Option<std::time::Instant> }

impl<'a> Updater<'a> {
    fn update_current(&mut self, string: &std::path::PathBuf) {
        match self.last_updated {
            Some(last_updated) => {
                let now = std::time::Instant::now();
                let delta = now - last_updated;
                if delta < std::time::Duration::from_millis(200) {
                    return;
                }
            }
            _ => (),
        }

        self.bar.tick();
        self.bar.set_message(string.clone().into_os_string().into_string().unwrap());
        self.last_updated = Some(std::time::Instant::now());
    }

    fn new(spinner: &ProgressBar) -> Updater {
        Updater { bar: spinner, last_updated: None }
    }
}