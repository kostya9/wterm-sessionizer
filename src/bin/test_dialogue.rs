use crate::dialogue::Dialogue;

#[path = "../dialogue.rs"]
mod dialogue;

fn main() {
    let mut d = Dialogue::<&str>::new();

    let items = vec!["hehe",
                     "hoohoo",
                     "oo",
                     "ee",
                     "hhhh",
                     "gg",
                     "wp"];

    d.add_items(items);
    d.interact();
}