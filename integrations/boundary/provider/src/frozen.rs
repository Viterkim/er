use super::ReadErr;

// Keep these conversions fixed. A current derive on both sides would hide a protocol change.
pub struct Report {
    pub diagnostics: String,
}
impl From<((String,), [(); 4])> for Report {
    fn from(((diagnostics,), _): ((String,), [(); 4])) -> Self {
        Self { diagnostics }
    }
}

pub struct Top<T = ()> {
    pub kind: ReadErr,
    pub diagnostics: String,
    pub data: T,
}
impl<T: Default> From<((ReadErr, String), [(); 5])> for Top<T> {
    fn from(((kind, diagnostics), _): ((ReadErr, String), [(); 5])) -> Self {
        Self {
            kind,
            diagnostics,
            data: T::default(),
        }
    }
}
impl<T> From<(((ReadErr, String), T), [(); 5])> for Top<T> {
    fn from((((kind, diagnostics), data), _): (((ReadErr, String), T), [(); 5])) -> Self {
        Self {
            kind,
            diagnostics,
            data,
        }
    }
}

pub struct SingleLine {
    pub diagnostics: String,
}
impl From<((String,), [(); 6])> for SingleLine {
    fn from(((diagnostics,), _): ((String,), [(); 6])) -> Self {
        Self { diagnostics }
    }
}

pub struct SingleLineTop {
    pub kind: ReadErr,
    pub diagnostics: String,
}
impl From<((ReadErr, String), [(); 7])> for SingleLineTop {
    fn from(((kind, diagnostics), _): ((ReadErr, String), [(); 7])) -> Self {
        Self { kind, diagnostics }
    }
}

pub struct Layouts {
    pub diagnostics: String,
    pub single_line: String,
}
impl From<((String, String), [(); 8])> for Layouts {
    fn from(((diagnostics, single_line), _): ((String, String), [(); 8])) -> Self {
        Self {
            diagnostics,
            single_line,
        }
    }
}

pub struct LayoutsTop {
    pub kind: ReadErr,
    pub diagnostics: String,
    pub single_line: String,
}
impl From<((ReadErr, String, String), [(); 9])> for LayoutsTop {
    fn from(((kind, diagnostics, single_line), _): ((ReadErr, String, String), [(); 9])) -> Self {
        Self {
            kind,
            diagnostics,
            single_line,
        }
    }
}
