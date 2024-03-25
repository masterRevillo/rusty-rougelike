use bracket_lib::color::RGB;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Messages {
    messages: Vec<(String, RGB)>
}

impl Messages {
    pub fn new() -> Self {
        Self {messages: vec![]}
    }

    pub fn add<T: Into<String>>(&mut self, message: T, color: RGB) {
        self.messages.push((message.into(), color))
    }

    // returns an `impl Trait`. basically, it allows you to specify a return type without explicitly describing the type
    // The actual return type just needs to implement the trait specified.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &(String, RGB)> {
        self.messages.iter()
    }
}