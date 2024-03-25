use bracket_lib::color::RGB;
use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;


pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let mut state = serializer.serialize_struct("Color", 3)?;
        state.serialize_field("r", &self.r)?;
        state.serialize_field("g", &self.g)?;
        state.serialize_field("b", &self.b)?;
        state.end()
    }
}