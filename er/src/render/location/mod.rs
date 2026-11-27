mod impls;
mod path;

use alloc::vec::Vec;

#[derive(Default)]
pub struct Locations<'a> {
    root: Option<EntryLocation<'a>>,
    previous: Option<EntryLocation<'a>>,
    ancestors: Vec<Option<EntryLocation<'a>>>,
    last_path: Option<(&'a str, RenderedPath<'a>)>,
}

#[derive(Clone, Copy, Eq)]
struct EntryLocation<'a> {
    line: u32,
    column: u32,
    file: &'a str,
}

#[derive(Clone, Copy)]
enum RenderedPath<'a> {
    Plain(&'a str),
    Git { repo: &'a str, tail: &'a str },
}
