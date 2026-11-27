use super::RenderedPath;

pub const MIN_CACHE_PATH: usize = "registry/src/-0000000000000000/a/b".len();

#[inline]
pub fn render_path(file: &str) -> RenderedPath<'_> {
    let path = dependency_path(file);
    #[cfg(feature = "small_path_src")]
    let path = small_path(path);
    path
}

// rustc can remap the cache to a relative path.
fn dependency_path(file: &str) -> RenderedPath<'_> {
    if file.len() < MIN_CACHE_PATH {
        return RenderedPath::Plain(file);
    }

    let bytes = file.as_bytes();
    for (dash, _) in file.match_indices('-') {
        let end = dash + 17;
        if !bytes.get(end).is_some_and(|&byte| is_sep(byte)) {
            continue;
        }

        let mut dirs = bytes[..dash].rsplit(|&byte| is_sep(byte));
        let name = dirs.next().unwrap_or_default();
        let rest = &file[end + 1..];
        match (dirs.next(), dirs.next()) {
            (Some(b"src"), Some(b"registry")) => {
                if is_hex(&bytes[dash + 1..end])
                    && rest
                        .as_bytes()
                        .iter()
                        .position(|&byte| is_sep(byte))
                        .is_some_and(|sep| sep > 0 && sep + 1 < rest.len())
                {
                    return RenderedPath::Plain(rest);
                }
            }
            (Some(b"checkouts"), Some(b"git")) if !name.is_empty() => {
                let Some((rev, path)) = rest.split_once(['/', '\\']) else {
                    continue;
                };
                if path.is_empty()
                    || !(7..=40).contains(&rev.len())
                    || !is_hex(rev.as_bytes())
                    || !is_hex(&bytes[dash + 1..end])
                {
                    continue;
                }
                let repo = &file[dash - name.len()..dash];
                if path
                    .strip_prefix(repo)
                    .is_some_and(|tail| tail.starts_with(['/', '\\']))
                {
                    return RenderedPath::Plain(path);
                }

                return RenderedPath::Git {
                    repo,
                    tail: &rest[rev.len()..],
                };
            }
            _ => {}
        }
    }

    RenderedPath::Plain(file)
}

fn is_sep(byte: u8) -> bool {
    matches!(byte, b'/' | b'\\')
}

fn is_hex(bytes: &[u8]) -> bool {
    // Checking the whole hash lets LLVM compare several bytes at once.
    bytes
        .iter()
        .fold(true, |valid, byte| valid & byte.is_ascii_hexdigit())
}

#[cfg(feature = "small_path_src")]
fn small_path(path: RenderedPath<'_>) -> RenderedPath<'_> {
    let file = match path {
        RenderedPath::Plain(file) => file,
        RenderedPath::Git { tail, .. } => &tail[1..],
    };
    let mut dirs = file.as_bytes().rsplit(|&byte| is_sep(byte));
    let mut start = file.len() - dirs.next().unwrap_or_default().len();

    while let Some(dir) = dirs.next() {
        start -= dir.len() + 1;
        if dir != b"src" {
            continue;
        }

        let src = &file[start..];
        for parent in dirs {
            start -= parent.len() + 1;
            if parent.is_empty() {
                continue;
            }
            if matches!(parent, [drive, b':'] if drive.is_ascii_alphabetic()) {
                break;
            }
            return RenderedPath::Plain(&file[start..]);
        }

        return match path {
            RenderedPath::Plain(_) => RenderedPath::Plain(src),
            RenderedPath::Git { .. } => path,
        };
    }

    path
}
