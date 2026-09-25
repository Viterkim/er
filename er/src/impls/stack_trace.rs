use crate::{ErErrorId, ErStackTrace, ErTraceExt, ErTree};
use alloc::{boxed::Box, vec::Vec};
use core::{any::type_name, fmt, panic::Location};
use std::backtrace::Backtrace;

impl fmt::Display for ErStackTrace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} @ {}\n{}",
            self.error_name, self.trace_location, self.capture
        )
    }
}
impl fmt::Debug for ErStackTrace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl<E> ErTraceExt for ErTree<E> {
    #[track_caller]
    fn er_trace(mut self) -> Self {
        self.stack_traces.push(ErStackTrace {
            error_name: type_name::<E>(),
            trace_location: Location::caller(),
            error_id: ErErrorId(0),
            capture: Box::new(Backtrace::force_capture()),
        });
        self
    }
}
impl<T, E> ErTraceExt for Result<T, ErTree<E>> {
    #[track_caller]
    fn er_trace(self) -> Self {
        match self {
            Ok(value) => Ok(value),
            Err(tree) => Err(tree.er_trace()),
        }
    }
}

pub fn append_traces(traces: &mut Vec<ErStackTrace>, mut incoming: Vec<ErStackTrace>) {
    if traces.is_empty() {
        *traces = incoming;
    } else {
        traces.append(&mut incoming);
    }
}
