use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::dialogue::Dialogue;

#[path = "../dialogue.rs"]
mod dialogue;

fn main() {
    let (tx, rx) = mpsc::channel::<Box<String>>();
    thread::spawn(move || {
        let mut i = 0;
        loop {
            let str = i.to_string();
            tx.send(Box::new(str)).unwrap();

            thread::sleep(Duration::from_secs(1));
            i += 1;
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