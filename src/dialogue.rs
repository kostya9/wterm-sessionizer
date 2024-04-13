use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt::Display;
use std::io::Write;

use dialoguer::console::{Key, style, StyledObject};
use dialoguer::console::Term;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

pub struct Dialogue<T> {
    items: Vec<T>,
}

impl<T> Dialogue<T> where T: Display, T: Eq {
    pub fn new() -> Dialogue<T> {
        Dialogue { items: vec![] }
    }

    pub fn interact(&self) -> Option<&T> {
        let mut renderer = Renderer::new();
        let mut full_input = CurrentInput {
            cursor: 0,
            input: "".to_string(),
            predictions: vec![],
            max_predictions: 5,
            matcher: SkimMatcherV2::default().ignore_case(),
            selected: None,
        };

        loop {
            self.fill_predictions(&mut full_input);
            full_input.selected = self.get_new_selected(&full_input);
            renderer.clear();

            let choose_prompt = "Choose: ";
            renderer.write_prompt(choose_prompt);
            let position = renderer.get_position();

            renderer.write_line(&full_input.input);

            for (idx, &item) in full_input.predictions.iter().enumerate() {
                let is_selected = match &full_input.selected {
                    Some(s) => s.idx == idx,
                    None => false
                };
                renderer.write_selection_item(&item.to_string(), is_selected);
            }

            let end_position = renderer.get_position();
            renderer.move_cursor_to(&position.with_x(position.x + full_input.input.len()));

            renderer.term.show_cursor().unwrap();
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
                Key::Escape => {
                    renderer.move_cursor_to(&end_position);
                    return None;
                }
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
                Key::ArrowUp => {
                    if full_input.predictions.len() == 0 {
                        full_input.selected = None;
                    } else {
                        let last_idx = full_input.predictions.len() - 1;
                        let next_idx = match &full_input.selected {
                            Some(s) => if s.idx == 0 { last_idx } else { s.idx - 1 },
                            None => last_idx
                        };

                        full_input.selected = Some(Selected {
                            idx: next_idx,
                            item: full_input.predictions.get(next_idx).unwrap(),
                        })
                    }
                }
                Key::ArrowDown => {
                    if full_input.predictions.len() == 0 {
                        full_input.selected = None;
                    } else {
                        let last_idx = full_input.predictions.len() - 1;
                        let next_idx = match &full_input.selected {
                            Some(s) => if s.idx == last_idx { 0 } else { s.idx + 1 },
                            None => 0
                        };

                        full_input.selected = Some(Selected {
                            idx: next_idx,
                            item: full_input.predictions.get(next_idx).unwrap(),
                        })
                    }
                }
                Key::Enter => {
                    match full_input.selected {
                        Some(selection) => {
                            renderer.move_cursor_to(&end_position);
                            renderer.clear();
                            renderer.write_successful("Choose: ", selection.item);
                            return Some(selection.item);
                        }
                        None => {}
                    }
                }

                _ => {}
            }

            renderer.term.hide_cursor().unwrap();
            renderer.move_cursor_to(&end_position);
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

    fn get_new_selected<'a>(&self, input: &CurrentInput<'a, T>) -> Option<Selected<'a, T>> {
        match &input.selected {
            Some(selected) => {
                if let Some(position) = input.predictions.iter().position(|x| **x == *selected.item) {
                    // If the same item is there, we preserve the selection of the item
                    return Some(Selected {
                        idx: position,
                        item: selected.item,
                    });
                }
                if selected.idx < input.predictions.len() {
                    // If the same position is there, we preserve the selection of the position
                    return Some(Selected {
                        idx: selected.idx,
                        item: input.predictions.get(selected.idx).unwrap(),
                    });
                }
            }
            None => {}
        };

        if input.predictions.len() > 0 {
            // Select the first thing if nothing is selected
            return Some(Selected {
                idx: 0,
                item: input.predictions.first().unwrap(),
            });
        }
        return None;
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
    selected: Option<Selected<'a, T>>,
}

struct Renderer {
    lines_number: usize,
    term: Term,
    cursor_position: RendererPosition,
}

struct Selected<'a, T> {
    idx: usize,
    item: &'a T,
}

impl Renderer {
    fn new() -> Renderer {
        Renderer {
            lines_number: 0,
            term: Term::stderr(),
            cursor_position: RendererPosition::zero(),
        }
    }

    fn get_position(&self) -> RendererPosition {
        return self.cursor_position.clone();
    }

    fn move_cursor_to(&mut self, position: &RendererPosition) {
        if self.cursor_position.y > position.y {
            self.term.move_cursor_up(self.cursor_position.y - position.y).unwrap();
        } else {
            self.term.move_cursor_down(position.y - self.cursor_position.y).unwrap();
        }

        self.term.move_cursor_right(position.x).unwrap();
        self.cursor_position = position.clone();
    }

    fn clear(&mut self) {
        self.term.clear_last_lines(self.lines_number).unwrap();
        self.lines_number = 0;
        self.cursor_position = RendererPosition::zero();
    }

    fn write_line(&mut self, message: &str) {
        self.term.write_line(message).unwrap();
        self.lines_number += 1;
        self.cursor_position.x = 0;
        self.cursor_position.y += 1;
    }

    fn write_line_formatted<T: Display>(&mut self, styled_object: StyledObject<T>) {
        self.write_formatted(styled_object);
        self.term.write_line("").unwrap();
        self.lines_number += 1;
        self.cursor_position.x = 0;
        self.cursor_position.y += 1;
    }

    fn write(&mut self, message: &str) {
        self.term.write(message.as_bytes()).unwrap();
    }

    fn write_formatted<T: Display>(&mut self, styled_object: StyledObject<T>) {
        self.write(styled_object.to_string().as_str());
    }

    fn write_selection_item(&mut self, item: &str, selected: bool) {
        let padding_left = 3;
        let styled_message = if selected {
            let prefix = style("❯").green().to_string() + (0..padding_left - 1).map(|_| " ").collect::<String>().as_str();
            let item = prefix + style(item).blue().to_string().as_str();
            style(item).bold()
        } else {
            let prefix = (0..padding_left).map(|_| " ").collect::<String>();
            let item = prefix + item;
            style(item)
        };

        self.cursor_position.x = self.cursor_position.x + padding_left + item.len();
        self.write_line_formatted(styled_message);
    }

    fn write_prompt(&mut self, prompt: &str) {
        let padding_left = 3;
        let prefix = style("?").yellow().to_string() + (0..padding_left - 1).map(|_| " ").collect::<String>().as_str();
        self.cursor_position.x = self.cursor_position.x + padding_left + prompt.len();
        self.write_formatted(style(prefix + prompt).bold());
    }

    pub fn write_successful<T>(&mut self, successful_message: &str, item: &T) where T: Display {
        let padding_left = 3;
        let prefix = style("✔").yellow().to_string() + (0..padding_left - 1).map(|_| " ").collect::<String>().as_str();
        self.cursor_position.x = self.cursor_position.x + padding_left + successful_message.len() + item.to_string().len();
        self.write_formatted(style(prefix + successful_message + item.to_string().as_str()).bold());
    }
}

#[derive(Clone)]
struct RendererPosition {
    x: usize,
    y: usize,
}

impl RendererPosition {
    fn zero() -> RendererPosition {
        return RendererPosition {
            x: 0,
            y: 0,
        };
    }

    fn with_x(&self, x: usize) -> RendererPosition {
        return RendererPosition {
            x,
            y: self.y,
        };
    }

    fn with_y(&self, y: usize) -> RendererPosition {
        return RendererPosition {
            x: self.x,
            y,
        };
    }
}
