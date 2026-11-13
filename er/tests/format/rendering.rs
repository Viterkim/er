use core::{error::Error, fmt};
use er::*;
use fmt::Write as _;
use std::io;

#[derive(Er)]
pub struct ParsePortErr(pub String);
pub fn parse_port(input: &str) -> ErResult<u16, ParsePortErr> {
    input.parse::<u16>().er(|_| input)
}

#[derive(Er)]
pub struct LoadConfigErr;
pub fn load_config(port: &str) -> ErResult<u16, LoadConfigErr> {
    parse_port(port).er(())
}

#[test]
pub fn causal_chain() {
    let error = load_config("aint_even_a_number_cmon_man").unwrap_err();

    #[cfg(feature = "src_locations")]
    let expected = format!(
        "LoadConfigErr @ {}\n`- ParsePortErr(\"aint_even_a_number_cmon_man\") @ {}\n   `- invalid digit found in string",
        error.src_location, error.nodes[0].src_location,
    );

    #[cfg(not(feature = "src_locations"))]
    let expected = "LoadConfigErr\n`- ParsePortErr(\"aint_even_a_number_cmon_man\")\n   `- invalid digit found in string";

    assert_eq!(error.er_report().to_string(), expected);
}

#[test]
pub fn panic_output() {
    let tree = ErTree::new(Message("context"), [Message("cause")]);
    let expected = tree.er_report();
    let result: ErResult<(), Message> = Err(tree);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        result.er_expect_report("operation failed");
    }))
    .unwrap_err();
    let message = panic.downcast_ref::<String>().expect("expect panic text");

    assert_eq!(message, &format!("operation failed: {expected}"));
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
        ("alpha\n\nbeta", "alpha\n\n   beta"),
        ("alpha\nbeta\n", "alpha\n   beta"),
    ] {
        let tree = ErTree::new(WrapErr, [Message(message)]);

        #[cfg(feature = "src_locations")]
        let expected = format!("WrapErr @ {}\n`- {expected}", tree.src_location,);

        #[cfg(not(feature = "src_locations"))]
        let expected = format!("WrapErr\n`- {expected}");

        assert_eq!(tree.er_report().to_string(), expected);
    }

    let tree = ErTree::new(
        Message("root one\nroot two"),
        [ErTree::from(Message("sub"))],
    );

    #[cfg(feature = "src_locations")]
    let expected = format!(
        "root one\nroot two @ {}\n`- sub @ {}",
        tree.src_location, tree.nodes[0].src_location,
    );

    #[cfg(not(feature = "src_locations"))]
    let expected = "root one\nroot two\n`- sub";

    assert_eq!(tree.er_report().to_string(), expected);
}

#[derive(Er)]
pub struct MidErr;
#[test]
pub fn branches() {
    let branch = ErTree::new(MidErr, [Message("alpha\nbeta")]);
    let tree = ErTree::new(
        WrapErr,
        [
            branch.into_er_part(),
            ErTree::from(Message("gamma")).into_er_part(),
        ],
    );

    #[cfg(feature = "src_locations")]
    let expected = format!(
        "WrapErr @ {}\n|- MidErr @ {}\n|  `- alpha\n|     beta\n`- gamma @ {}",
        tree.src_location, tree.nodes[0].src_location, tree.nodes[1].src_location,
    );

    #[cfg(not(feature = "src_locations"))]
    let expected = "WrapErr\n|- MidErr\n|  `- alpha\n|     beta\n`- gamma";

    assert_eq!(tree.er_report().to_string(), expected);
}

#[test]
pub fn message_text() {
    for (message, indented) in [
        ("request @ backend", "request @ backend"),
        (
            "OuterErr @ other.rs:1:1\n`- InnerErr @ other.rs:2:2 [retry | later]",
            "OuterErr @ other.rs:1:1\n   `- InnerErr @ other.rs:2:2 [retry | later]",
        ),
    ] {
        let tree = ErTree::new(Message(message), [Message(message)]);

        #[cfg(feature = "src_locations")]
        let expected = format!("{message} @ {}\n`- {indented}", tree.src_location,);

        #[cfg(not(feature = "src_locations"))]
        let expected = format!("{message}\n`- {indented}");

        assert_eq!(tree.er_report().to_string(), expected);
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
        let tree = ErTree::from(Chunked(chunks));

        #[cfg(feature = "src_locations")]
        let expected = format!("alpha\nbeta @ {}", tree.src_location);

        #[cfg(not(feature = "src_locations"))]
        let expected = "alpha\nbeta";

        assert_eq!(tree.er_report().to_string(), expected);

        let mut text = String::new();
        let writer: &mut dyn fmt::Write = &mut text;
        tree.er_write_report(writer).unwrap();

        assert_eq!(text, expected);
    }
}

#[test]
pub fn partitions() -> fmt::Result {
    let input = "æ\r\n日本\rc\0🦀\n\n";
    let boundaries: Vec<_> = input
        .char_indices()
        .skip(1)
        .map(|(index, _)| index)
        .collect();

    let render = |parts: &[&str]| -> Result<String, fmt::Error> {
        let mut text = String::new();
        let mut writer = render::LineWriter::indent(&mut text, "  ");

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
    for ch in input.chars().chain(['!']) {
        writer.write_char(ch)?;
    }
    assert_eq!(characters, expected);

    // Each bit chooses a cut between two characters.
    for cuts in 0..1 << boundaries.len() {
        let mut parts = Vec::new();
        let mut start = 0;
        for (bit, end) in boundaries.iter().copied().enumerate() {
            if cuts & (1 << bit) != 0 {
                parts.push(&input[start..end]);
                start = end;
            }
        }
        parts.push(&input[start..]);

        assert_eq!(render(&parts)?, expected, "{parts:?}");
    }

    Ok(())
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

    assert_eq!(
        tree.er_report()
            .to_string()
            .matches("connect timeout")
            .count(),
        2
    );
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
    let tree = ErTree::from(Chunked(&["kept", "too long"]));

    let mut sink = Budget {
        remaining: 4,
        written: String::new(),
    };

    assert!(tree.er_write_report(&mut sink).is_err());
    assert_eq!(sink.written, "kept");
}
