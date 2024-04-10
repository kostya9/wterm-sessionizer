use std::fmt::Display;
use std::io::Write;

use dialoguer::{console::Term, theme::SimpleTheme};
use dialoguer::console::Key;

pub struct Dialogue<T> {
    items: Vec<T>,
    message: String,
}

impl<T> Dialogue<T> where T: Display {
    pub fn new(message: String) -> Dialogue<T> {
        Dialogue { message, items: vec![] }
    }

    pub fn interact(&self) {
        let mut renderer = Renderer::new();
        let mut full_input = CurrentInput {
            cursor: 0,
            input: "".to_string(),
        };

        loop {
            renderer.clear();

            renderer.write_line("Choose:");
            renderer.write(&"> ".to_string());

            renderer.write_line(&full_input.input);

            for item in &self.items {
                renderer.write_line(&item.to_string());
            }

            let cursor_dy = renderer.lines_number - 1;
            let cursor_dx = "> ".len() + full_input.cursor;
            renderer.term.move_cursor_up(cursor_dy).unwrap();
            renderer.term.move_cursor_right(cursor_dx).unwrap();

            match renderer.term.read_key().unwrap()
            {
                Key::Char(char) => {
                    full_input.input.insert(full_input.cursor, char);
                    full_input.cursor += 1;
                }
                Key::Backspace => {
                    if full_input.input.len() > 0 {
                        full_input.input.remove(full_input.cursor);
                        full_input.cursor -= 1;
                    }
                }
                Key::Escape => break,
                Key::ArrowLeft => {
                    if full_input.cursor > 0 {
                        full_input.cursor -= 1;
                    }
                },
                Key::ArrowRight => {
                    if full_input.cursor < full_input.input.len() {
                        full_input.cursor += 1;
                    }
                },
                _ => {}
            }

            renderer.term.move_cursor_left(cursor_dx).unwrap();
            renderer.term.move_cursor_down(cursor_dy).unwrap();
        }
    }

    pub fn add_items(&mut self, items: Vec<T>) {
        for item in items {
            self.items.push(item);
        }
    }
}

struct CurrentInput {
    input: String,
    cursor: usize,
}

struct Renderer {
    theme: SimpleTheme,
    lines_number: usize,
    term: Term,
}

impl Renderer {
    fn new() -> Renderer {
        Renderer {
            theme: SimpleTheme {},
            lines_number: 0,
            term: Term::stderr(),
        }
    }

    fn clear(&mut self) {
        self.term.clear_last_lines(self.lines_number).unwrap();
        self.lines_number = 0;
    }

    fn write_line(&mut self, message: &str) {
        self.term.write_line(message).unwrap();
        self.lines_number += 1;
    }

    fn write(&mut self, message: &String) {
        self.term.write(message.as_bytes()).unwrap();
    }
}
