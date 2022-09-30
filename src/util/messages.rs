use serde::{Deserialize, Serialize};
use tcod::colors::*;

#[derive(Serialize, Deserialize)]
pub struct Messages {
    messages: Vec<(String, Color)>
}

impl Messages {
    pub fn new() -> Self {
        Self {messages: vec![]}
    }

    pub fn add<T: Into<String>>(&mut self, message: T, color: Color) {
        self.messages.push((message.into(), color))
    }

    // returns an `impl Trait`. basically, it allows you to specify a return type without explicitly describing the type
    // The actual return type just needs to implement the trait specified.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &(String, Color)> {
        self.messages.iter()
    }
}