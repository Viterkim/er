use crate::{ErLineError, lines::Lines};
use core::{error::Error, fmt};

impl<E: fmt::Display> fmt::Display for ErLineError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(e) => fmt::Display::fmt(e, formatter),
            Self::Callback(e) => fmt::Display::fmt(e, formatter),
        }
    }
}
impl<E: Error + 'static> Error for ErLineError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Format(e) => Some(e),
            Self::Callback(e) => Some(e),
        }
    }
}

impl<F: FnMut(&str) -> Result<(), E>, E> fmt::Write for Lines<F, E> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if self.error.is_some() {
            return Err(fmt::Error);
        }

        for part in text.split_inclusive('\n') {
            if let Some(line) = part.strip_suffix('\n') {
                self.buffer.push_str(line);
                let line = self.buffer.strip_suffix('\r').unwrap_or(&self.buffer);
                if let Err(error) = (self.emit)(line) {
                    self.error = Some(error);
                    return Err(fmt::Error);
                }
                self.buffer.clear();
            } else {
                self.buffer.push_str(part);
            }
        }

        Ok(())
    }
}
