use alloc::string::String;
use core::{convert::Infallible, fmt};

/// Formatting or callback failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineError<E> {
    Format(fmt::Error),
    Callback(E),
}

pub struct Lines<F, E> {
    pub buffer: String,
    pub emit: F,
    pub error: Option<E>,
}

/// Only formats once, no line endings.
pub fn for_each_line(
    value: &(impl fmt::Display + ?Sized),
    mut emit: impl FnMut(&str),
) -> fmt::Result {
    let result: Result<(), LineError<Infallible>> = try_for_each_line(value, |line| {
        emit(line);
        Ok(())
    });

    match result {
        Ok(()) => Ok(()),
        Err(LineError::Format(e)) => Err(e),
        Err(LineError::Callback(e)) => match e {},
    }
}

pub fn try_for_each_line<E>(
    value: &(impl fmt::Display + ?Sized),
    emit: impl FnMut(&str) -> Result<(), E>,
) -> Result<(), LineError<E>> {
    let mut writer = Lines {
        buffer: String::new(),
        emit,
        error: None,
    };

    let rendered = fmt::write(&mut writer, format_args!("{value}"));
    if let Some(e) = writer.error {
        return Err(LineError::Callback(e));
    }

    rendered.map_err(LineError::Format)?;
    if !writer.buffer.is_empty() {
        (writer.emit)(&writer.buffer).map_err(LineError::Callback)?;
    }

    Ok(())
}
