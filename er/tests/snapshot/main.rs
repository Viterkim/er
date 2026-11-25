pub mod capture;
pub mod presentation;
pub mod sources;

use core::{error::Error, fmt};

pub struct Message {
    pub text: &'static str,
    pub source: Option<Box<Message>>,
}
impl Message {
    pub fn new(text: &'static str) -> Self {
        Self { text, source: None }
    }
}
impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.text == "broken" {
            f.write_str("partial message")?;
            return Err(fmt::Error);
        }

        f.write_str(self.text)
    }
}
impl fmt::Debug for Message {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!("snapshot must use Display")
    }
}
impl Error for Message {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref().map(|source| source as &dyn Error)
    }
}
