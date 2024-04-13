use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt::Display;
use std::io::Write;

use dialoguer::{console::Term, theme::SimpleTheme};
use dialoguer::console::Key;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

pub struct Dialogue<T> {
    items: Vec<T>,
}

impl<T> Dialogue<T> where T: Display {
    pub fn new() -> Dialogue<T> {
        Dialogue { items: vec![] }
    }

    pub fn interact(&self) {
        let mut renderer = Renderer::new();
        let mut full_input = CurrentInput {
            cursor: 0,
            input: "".to_string(),
            predictions: vec![],
            max_predictions: 5,
            matcher: SkimMatcherV2::default(),
        };


        loop {
            self.fill_predictions(&mut full_input);
            renderer.clear();

            renderer.write_line("Choose:");
            renderer.write(&"> ".to_string());

            renderer.write_line(&full_input.input);

            for item in &full_input.predictions {
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
                        full_input.input.remove(full_input.cursor - 1);
                        full_input.cursor -= 1;
                    }
                }
                Key::Escape => break,
                Key::ArrowLeft => {
                    if full_input.cursor > 0 {
                        full_input.cursor -= 1;
                    }
                }
                Key::ArrowRight => {
                    if full_input.cursor < full_input.input.len() {
                        full_input.cursor += 1;
                    }
                }
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

    fn fill_predictions<'a>(&'a self, input: &mut CurrentInput<'a, T>) {
        let predictions = &mut input.predictions;

        // TODO: do we *really* need to allocate a heap here?
        let mut binary_heap = BinaryHeap::with_capacity(input.max_predictions);

        let items = self.items.iter().map(|i| (i, input.matcher.fuzzy_match(&format!("{}", i), &input.input)));
        for (item, score) in items {
            match score {
                Some(score) =>
                    binary_heap.push(Prediction { score, item }),
                _ => {}
            }

            if binary_heap.len() > input.max_predictions { binary_heap.pop(); }
        }


        predictions.clear();
        for prediction in binary_heap {
            predictions.push(prediction.item);
        }
    }
}

struct Prediction<'a, T: ?Sized> {
    item: &'a T,
    score: i64,
}

impl<'a, T> Eq for Prediction<'a, T> {}

impl<'a, T> PartialEq<Self> for Prediction<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        return self.score.eq(&other.score);
    }
}

impl<'a, T> PartialOrd<Self> for Prediction<'a, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        return self.score.partial_cmp(&other.score);
    }
}

impl<'a, T> Ord for Prediction<'a, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        return self.score.cmp(&other.score);
    }
}

struct CurrentInput<'a, T> {
    input: String,
    cursor: usize,
    predictions: Vec<&'a T>,
    max_predictions: usize,
    matcher: SkimMatcherV2,
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
