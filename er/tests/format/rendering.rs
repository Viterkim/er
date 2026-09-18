use core::{error::Error, fmt};
use er::*;
use fmt::Write as _;
use std::io;

#[derive(Er)]
pub struct ParsePortErr(pub String);
pub fn parse_port(input: &str) -> Er<u16, ParsePortErr> {
    input.parse::<u16>().er(|| ParsePortErr::new(input))
}

#[derive(Er)]
pub struct LoadConfigErr;
pub fn load_config(port: &str) -> Er<u16, LoadConfigErr> {
    parse_port(port).er(())
}

#[test]
pub fn causal_chain() {
    let error = load_config("aint_even_a_number_cmon_man").unwrap_err();

    #[cfg(feature = "src_locations")]
    let expected = format!(
        "LoadConfigErr @ {}\n`- ParsePortErr(\"aint_even_a_number_cmon_man\") @ {}\n   `- invalid digit found in string @ {}",
        error.src_location, error.nodes[0].src_location, error.nodes[0].nodes[0].src_location,
    );
    #[cfg(not(feature = "src_locations"))]
    let expected = "LoadConfigErr\n`- ParsePortErr(\"aint_even_a_number_cmon_man\")\n   `- invalid digit found in string";

    assert_eq!(error.er_report().to_string(), expected);
}

#[test]
pub fn panic_output() {
    assert_eq!(parse_port("85").er_report().unwrap(), 85);
    assert_eq!(parse_port("80").er_top().expect("usable port"), 80);

    for top in [false, true] {
        let tree = ErTree::new(Message("context"), [Message("cause")]);
        let expected = if top {
            tree.er_top().to_string()
        } else {
            tree.er_report().to_string()
        };
        let result: Er<(), Message> = Err(tree);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if top {
                result.er_top().expect("operation failed");
            } else {
                result.er_report().expect("operation failed");
            }
        }))
        .unwrap_err();
        let message = panic.downcast_ref::<String>().expect("expect panic text");

        assert_eq!(message, &format!("operation failed: {expected}"));
    }
}

pub struct Message(pub &'static str);
impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl fmt::Debug for Message {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!("unexpected Debug")
    }
}
impl Error for Message {}

#[derive(Er)]
pub struct WrapErr;
#[test]
pub fn multiline() {
    for (message, expected) in [
        ("first\nsecond\nthird", "first\n   second\n   third"),
        ("alpha\n\nbeta", "alpha\n   \n   beta"),
        ("alpha\nbeta\n", "alpha\n   beta"),
    ] {
        let tree = ErTree::new(WrapErr, [Message(message)]);
        #[cfg(feature = "src_locations")]
        let expected = format!(
            "WrapErr @ {}\n`- {expected} @ {}",
            tree.src_location, tree.nodes[0].src_location,
        );
        #[cfg(not(feature = "src_locations"))]
        let expected = format!("WrapErr\n`- {expected}");

        assert_eq!(tree.er_report().to_string(), expected);
    }

    let tree = ErTree::new(Message("root one\nroot two"), [Message("child").er()]);
    #[cfg(feature = "src_locations")]
    let expected = format!(
        "root one\nroot two @ {}\n`- child @ {}",
        tree.src_location, tree.nodes[0].src_location,
    );
    #[cfg(not(feature = "src_locations"))]
    let expected = "root one\nroot two\n`- child";

    assert_eq!(tree.er_report().to_string(), expected);
}

#[derive(Er)]
pub struct MidErr;
#[test]
pub fn branches() {
    let branch = ErTree::new(MidErr, [Message("alpha\nbeta")]);
    let tree = ErTree::new(
        WrapErr,
        [branch.into_er_node(), Message("gamma").er().into_er_node()],
    );
    #[cfg(feature = "src_locations")]
    let expected = format!(
        "WrapErr @ {}\n|- MidErr @ {}\n|  `- alpha\n|     beta @ {}\n`- gamma @ {}",
        tree.src_location,
        tree.nodes[0].src_location,
        tree.nodes[0].nodes[0].src_location,
        tree.nodes[1].src_location,
    );
    #[cfg(not(feature = "src_locations"))]
    let expected = "WrapErr\n|- MidErr\n|  `- alpha\n|     beta\n`- gamma";

    assert_eq!(tree.er_report().to_string(), expected);

    #[cfg(feature = "src_locations")]
    let expected = format!(
        "WrapErr @ {} [MidErr @ {} [alpha beta @ {}] | gamma @ {}]",
        tree.src_location,
        tree.nodes[0].src_location,
        tree.nodes[0].nodes[0].src_location,
        tree.nodes[1].src_location,
    );
    #[cfg(not(feature = "src_locations"))]
    let expected = "WrapErr [MidErr [alpha beta] | gamma]";

    assert_eq!(tree.er_report().single_line().to_string(), expected);
}

#[test]
pub fn message_text() {
    for (message, indented, flat) in [
        (
            "request @ backend",
            "request @ backend",
            "request @ backend",
        ),
        (
            "OuterErr @ other.rs:1:1\n`- InnerErr @ other.rs:2:2 [retry | later]",
            "OuterErr @ other.rs:1:1\n   `- InnerErr @ other.rs:2:2 [retry | later]",
            "OuterErr @ other.rs:1:1 `- InnerErr @ other.rs:2:2 [retry | later]",
        ),
    ] {
        let tree = ErTree::new(Message(message), [Message(message)]);
        #[cfg(feature = "src_locations")]
        let expected = format!(
            "{message} @ {}\n`- {indented} @ {}",
            tree.src_location, tree.nodes[0].src_location,
        );
        #[cfg(not(feature = "src_locations"))]
        let expected = format!("{message}\n`- {indented}");

        assert_eq!(tree.er_report().to_string(), expected);

        #[cfg(feature = "src_locations")]
        let expected = format!(
            "{flat} @ {} [{flat} @ {}]",
            tree.src_location, tree.nodes[0].src_location,
        );
        #[cfg(not(feature = "src_locations"))]
        let expected = format!("{flat} [{flat}]");

        assert_eq!(tree.er_report().single_line().to_string(), expected);
    }
}

pub struct Chunked(pub &'static [&'static str]);
impl fmt::Display for Chunked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chunk in self.0 {
            f.write_str(chunk)?;
        }

        Ok(())
    }
}
impl fmt::Debug for Chunked {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!("unexpected Debug")
    }
}
impl Error for Chunked {}
#[test]
pub fn chunks() {
    for chunks in [
        &["alpha\nbeta"][..],
        &["alp", "ha", "\n", "be", "ta"],
        &["alpha\r", "\nbeta"],
    ] {
        let tree = Chunked(chunks).er();
        #[cfg(feature = "src_locations")]
        let expected = format!("alpha\nbeta @ {}", tree.src_location);
        #[cfg(not(feature = "src_locations"))]
        let expected = "alpha\nbeta";

        assert_eq!(tree.er_report().to_string(), expected);
        assert_eq!(tree.er_top().single_line().to_string(), "alpha beta");

        let mut text = String::new();
        let writer: &mut dyn fmt::Write = &mut text;
        render::write_report(writer, &tree.top, &[], Layout::Multiline, None).unwrap();

        assert_eq!(text, "alpha\nbeta");
        text.clear();
        let value: &dyn fmt::Display = &tree.top;
        render::write_top(&mut text as &mut dyn fmt::Write, value, Layout::SingleLine).unwrap();

        assert_eq!(text, "alpha beta");

        text.clear();
        let mut writer = render::LineWriter::indent(&mut text as &mut dyn fmt::Write, "  ");
        writer.wrap = render::Wrap::Flatten;
        fmt::write(&mut writer, format_args!("{value}")).unwrap();

        assert_eq!(text, "alpha beta");
    }
}

#[test]
pub fn partitions() -> fmt::Result {
    let input = "a\r\nb\rc\0d\n\n";

    for wrap in [render::Wrap::Indent("  "), render::Wrap::Flatten] {
        let render = |parts: &[&str]| -> Result<String, fmt::Error> {
            let mut text = String::new();
            let mut writer = render::LineWriter::indent(&mut text, "  ");
            writer.wrap = wrap;

            for part in parts {
                writer.write_str("")?;
                writer.write_str(part)?;
            }
            writer.write_str("")?;
            writer.write_char('!')?;
            Ok(text)
        };
        let expected = render(&[input])?;

        let mut characters = String::new();
        let mut writer = render::LineWriter::indent(&mut characters, "  ");
        writer.wrap = wrap;
        for ch in input.chars().chain(['!']) {
            writer.write_char(ch)?;
        }
        assert_eq!(characters, expected);

        // Each bit chooses a cut between two characters.
        for cuts in 0..1 << (input.len() - 1) {
            let mut parts = Vec::new();
            let mut start = 0;
            for end in 1..input.len() {
                if cuts & (1 << (end - 1)) != 0 {
                    parts.push(&input[start..end]);
                    start = end;
                }
            }
            parts.push(&input[start..]);

            assert_eq!(render(&parts)?, expected, "{parts:?}");
        }
    }

    Ok(())
}

#[test]
pub fn single_line() {
    for (chunks, expected) in [
        (&["a\nb\r\nc\rd\0e"][..], "a b c d e"),
        (&["a\r", "\nb"], "a b"),
        (&["a\n", "\r\0"], "a"),
        (&["kølig 😀 værdi\ttab"], "kølig 😀 værdi\ttab"),
    ] {
        let tree = Chunked(chunks).er();
        assert_eq!(tree.er_top().single_line().to_string(), expected);

        let report = tree.into_er_report().single_line();
        #[cfg(feature = "src_locations")]
        let expected_report = format!("{expected} @ {}", report.tree.src_location);
        #[cfg(not(feature = "src_locations"))]
        let expected_report = expected;

        assert_eq!(report.to_string(), expected_report);
        assert_eq!(report.as_ref().to_string(), report.to_string());

        let top = report.tree.into_er_top().single_line();
        assert_eq!(top.as_ref().to_string(), expected);
        assert_eq!(format!("{top:#?}"), expected);
    }

    assert_eq!(Message("a\nb").er().er_top().to_string(), "a\nb");
}

#[derive(Debug)]
pub struct LegacyErr(pub io::Error);
impl fmt::Display for LegacyErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "legacy failed: {}", self.0)
    }
}
impl Error for LegacyErr {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

#[test]
pub fn display_includes_source() {
    let cause = io::Error::new(io::ErrorKind::TimedOut, "connect timeout");
    let tree = ErTree::new(WrapErr, [LegacyErr(cause)]);

    for layout in [Layout::Multiline, Layout::SingleLine] {
        assert_eq!(
            tree.er_report()
                .layout(layout)
                .to_string()
                .matches("connect timeout")
                .count(),
            2
        );
    }
}

pub struct Budget {
    pub remaining: usize,
    pub written: String,
}
impl fmt::Write for Budget {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if text.len() > self.remaining {
            return Err(fmt::Error);
        }
        self.remaining -= text.len();
        self.written.push_str(text);

        Ok(())
    }
}

#[test]
pub fn writer_failure() {
    use fmt::Write as _;
    let tree = ErTree::new(WrapErr, [Message("a fairly long child message")]);

    for layout in [Layout::Multiline, Layout::SingleLine] {
        let mut sink = Budget {
            remaining: 12,
            written: String::new(),
        };

        assert!(write!(sink, "{}", tree.er_report().layout(layout)).is_err());
        assert!(!sink.written.contains("fairly long"));
    }
}
