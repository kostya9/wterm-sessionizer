use std::{io, mem};
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::fmt::Display;
use std::io::Write;
use std::ops::Deref;
use std::sync::mpsc::Receiver;
use std::thread::sleep;
use std::time::Duration;

use dialoguer::console::{Key, style, StyledObject};
use dialoguer::console::Term;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

mod windows_input;

pub struct Dialogue<T> {
    items: Vec<T>,
    additional_items_receiver: Receiver<Box<T>>,
}

impl<T> Dialogue<T> where T: Display, T: Eq, T: Clone {
    pub fn new(receiver: Receiver<Box<T>>) -> Dialogue<T> {
        Dialogue { items: vec![], additional_items_receiver: receiver }
    }

    pub fn interact(&mut self) -> io::Result<Option<T>> {
        let mut renderer = Renderer::new();
        let mut full_input = CurrentInput {
            cursor: 0,
            input: "".to_string(),
            predictions: vec![],
            max_predictions: 20,
            matcher: SkimMatcherV2::default().ignore_case(),
            selected: None,
        };

        'outer: loop {
            self.fill_predictions(&mut full_input);
            renderer.clear()?;

            let choose_prompt = "Choose: ";
            renderer.write_prompt(choose_prompt)?;
            let position = renderer.get_position();

            renderer.write_line(&full_input.input)?;

            for (idx, item) in full_input.predictions.iter().enumerate() {
                let is_selected = match &full_input.selected {
                    Some(s) => s.idx == idx,
                    None => false
                };
                renderer.write_selection_item(&item.to_string(), is_selected)?;
            }

            let end_position = renderer.get_position();
            renderer.move_cursor_to(&position.with_x(position.x + full_input.input.len()))?;

            renderer.term.show_cursor()?;
            let key = loop {
                if let Some(key) = windows_input::try_read_single_key()? {
                    break key;
                }

                if self.handle_received_items() {
                    if self.fill_predictions(&mut full_input) {
                        renderer.term.hide_cursor()?;
                        renderer.move_cursor_to(&end_position)?;
                        continue 'outer;
                    }
                }

                sleep(Duration::from_millis(10));
            };
            match key
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
                    renderer.move_cursor_to(&end_position)?;
                    return Ok(None);
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
                            item: full_input.predictions.get(next_idx).unwrap().clone(),
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
                            item: full_input.predictions.get(next_idx).unwrap().clone(),
                        })
                    }
                }
                Key::Enter => {
                    match full_input.selected {
                        Some(selection) => {
                            renderer.move_cursor_to(&end_position)?;
                            renderer.clear()?;
                            renderer.write_successful("Choose: ", &selection.item)?;
                            return Ok(Some(selection.item.clone()));
                        }
                        None => {}
                    }
                }

                _ => {}
            }

            renderer.term.hide_cursor()?;
            renderer.move_cursor_to(&end_position)?;
        }
    }

    pub fn add_items(&mut self, items: Vec<T>) {
        for item in items {
            self.items.push(item);
        }
    }

    /// We want to rerender stuff as little as possible, so this returns whether the predictions actually changed
    fn fill_predictions(&self, input: &mut CurrentInput<T>) -> bool {
        let predictions = &mut input.predictions;

        // TODO: do we *really* need to allocate a heap here?
        // This should be a min-heap cause we want the top scores here
        let mut binary_heap = BinaryHeap::<Reverse<Prediction<T>>>::with_capacity(input.max_predictions);

        let items = self.items.iter().map(|i| (i, input.matcher.fuzzy_match(&format!("{}", i), &input.input)));
        for (item, score) in items {
            match score {
                Some(score) =>
                    if binary_heap.len() < input.max_predictions {
                        binary_heap.push(Reverse(Prediction { score, item }));
                    } else {
                        if let Some(min_element) = binary_heap.peek() {
                            if score > min_element.0.score {
                                binary_heap.pop();
                                binary_heap.push(Reverse(Prediction { score, item }));
                            }
                        }
                    },
                _ => {}
            }
        }

        let same_size = binary_heap.len() == predictions.len();
        if !same_size {
            predictions.clear();
        }
        let mut changed = false;
        for (idx, reverse_prediction) in binary_heap.iter().rev().enumerate() {
            let prediction = &reverse_prediction.0;
            if let Some(cur) = predictions.get(idx) {
                if cur == prediction.item {
                    continue;
                }
            }

            let len = predictions.len();
            if idx < len {
                let _ = mem::replace(&mut predictions[idx], prediction.item.clone());
            } else {
                predictions.push(prediction.item.clone());
            }
            changed = true;
        }

        if !changed {
            return false;
        }

        input.selected = self.get_new_selected(input);
        return true;
    }

    fn get_new_selected(&self, input: &CurrentInput<T>) -> Option<Selected<T>> {
        match &input.selected {
            Some(selected) => {
                if let Some(position) = input.predictions.iter().position(|x| *x == selected.item) {
                    // If the same item is there, we preserve the selection of the item
                    return Some(Selected {
                        idx: position,
                        item: selected.item.clone(),
                    });
                }
                if selected.idx < input.predictions.len() {
                    // If the same position is there, we preserve the selection of the position
                    return Some(Selected {
                        idx: selected.idx,
                        item: input.predictions.get(selected.idx)?.clone(),
                    });
                }
            }
            None => {}
        };

        if input.predictions.len() > 0 {
            // Select the first thing if nothing is selected
            return Some(Selected {
                idx: 0,
                item: input.predictions.first()?.clone(),
            });
        }
        return None;
    }

    fn handle_received_items(&mut self) -> bool {
        let mut added = false;
        let events = self.additional_items_receiver.try_iter();
        for event in events {
            self.items.push(event.deref().clone());
            added = true;
        }
        return added;
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

struct CurrentInput<T> {
    input: String,
    cursor: usize,
    predictions: Vec<T>,
    max_predictions: usize,
    matcher: SkimMatcherV2,
    selected: Option<Selected<T>>,
}

struct Renderer {
    lines_number: usize,
    term: Term,
    cursor_position: RendererPosition,
}

struct Selected<T> {
    idx: usize,
    item: T,
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

    fn move_cursor_to(&mut self, position: &RendererPosition) -> io::Result<()> {
        if self.cursor_position.y > position.y {
            self.term.move_cursor_up(self.cursor_position.y - position.y)?;
        } else {
            self.term.move_cursor_down(position.y - self.cursor_position.y)?;
        }

        self.term.move_cursor_right(position.x)?;
        self.cursor_position = position.clone();
        Ok(())
    }

    fn clear(&mut self) -> io::Result<()> {
        self.term.clear_last_lines(self.lines_number)?;
        self.lines_number = 0;
        self.cursor_position = RendererPosition::zero();
        Ok(())
    }

    fn write_line(&mut self, message: &str) -> io::Result<()> {
        self.term.write_line(message)?;
        self.lines_number += 1;
        self.cursor_position.x = 0;
        self.cursor_position.y += 1;

        Ok(())
    }

    fn write_line_formatted<T: Display>(&mut self, styled_object: StyledObject<T>) -> io::Result<()> {
        self.write_formatted(styled_object)?;
        self.term.write_line("")?;
        self.lines_number += 1;
        self.cursor_position.x = 0;
        self.cursor_position.y += 1;

        Ok(())
    }

    fn write(&mut self, message: &str) -> io::Result<()> {
        self.term.write(message.as_bytes())?;
        Ok(())
    }

    fn write_formatted<T: Display>(&mut self, styled_object: StyledObject<T>) -> io::Result<()> {
        self.write(styled_object.to_string().as_str())
    }

    fn write_selection_item(&mut self, item: &str, selected: bool) -> io::Result<()> {
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
        self.write_line_formatted(styled_message)
    }

    fn write_prompt(&mut self, prompt: &str) -> io::Result<()> {
        let padding_left = 3;
        let prefix = style("?").yellow().to_string() + (0..padding_left - 1).map(|_| " ").collect::<String>().as_str();
        self.cursor_position.x = self.cursor_position.x + padding_left + prompt.len();
        self.write_formatted(style(prefix + prompt).bold())
    }

    pub fn write_successful<T>(&mut self, successful_message: &str, item: &T) -> io::Result<()> where T: Display {
        let padding_left = 3;
        let prefix = style("✔").yellow().to_string() + (0..padding_left - 1).map(|_| " ").collect::<String>().as_str();
        self.cursor_position.x = self.cursor_position.x + padding_left + successful_message.len() + item.to_string().len();
        self.write_formatted(style(prefix + successful_message + item.to_string().as_str()).bold())
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
