use crate::dialogue::Dialogue;

#[path = "../dialogue.rs"] mod dialogue;

fn main() {
    let mut d = Dialogue::<String>::new("hehe".to_string());
    
    let items = vec!["hehe".to_string(), "hoohoo".to_string()];
    
    d.add_items(items);
    d.interact();
}