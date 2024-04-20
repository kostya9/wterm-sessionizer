use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::dialogue::{Dialogue, DialogueMessage};
use crate::dialogue::DialogueMessage::{Finish, ItemsFound, ProgressUpdate};

#[path = "../dialogue.rs"]
mod dialogue;

fn main() {
    let (tx, rx) = mpsc::channel::<DialogueMessage<String>>();
    thread::spawn(move || {
        let mut i = 0;
        loop {
            let str = i.to_string();
            tx.send(ItemsFound(vec![str]));
            tx.send(ProgressUpdate(format!("Found {i}...").into_boxed_str()));

            thread::sleep(Duration::from_secs(1));
            i += 1;

            if i > 20 {
                tx.send(Finish);
                break;
            }
        }
    });


    let mut d = Dialogue::<String>::new(rx);

    let items = vec!["hehe",
                     "hoohoo",
                     "oo",
                     "ee",
                     "hhhh",
                     "gg",
                     "wp"];

    d.add_items(items.iter().map(|x| x.to_string()).collect());
    d.interact().unwrap();
}