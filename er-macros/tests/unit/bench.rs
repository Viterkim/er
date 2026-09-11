use crate::generate::expand;
use std::{hint::black_box, time::Instant};

pub fn cases() -> Vec<(&'static str, String, usize)> {
    let variants = (0..1000)
        .map(|i| format!("Variant{i} {{ input: String }},"))
        .collect::<String>();
    let large_enum = format!("pub enum ManyEr {{ {variants} }}");

    let nested = format!("{}Self{}", "Vec<(T, ".repeat(32), ")>".repeat(32));
    let recursive = format!("pub struct Recursive<T> {{ pub children: {nested} }}");

    vec![
        ("unit", "pub struct ReadEr;".into(), 85),
        ("named", "pub struct ReadEr { pub path: std::path::PathBuf, pub message: String }".into(), 85),
        ("enum", large_enum, 3),
        ("recursive", recursive, 20),
        (
            "projection",
            "pub struct Projected<T> {
            #[er(skip)] pub marker: std::marker::PhantomData<T>,
            pub value: <Self as HasValue>::Value,
        }"
            .into(),
            85,
        ),
        ("wrap", "#[er(wrap)] pub struct ReadEr;".into(), 85),
        (
            "format",
            "#[er(format = \"{value} / {value:?} / {value:x}; {secret:>width$}\", wrap(output = report))]
        pub struct SecretEr<T> {
            pub value: T,
            pub width: usize,
            #[er(censor)] pub secret: String,
        }"
            .into(),
            85,
        ),
    ]
}

#[test]
#[ignore]
pub fn expansion_cost() -> syn::Result<()> {
    for (name, source, repeats) in cases() {
        let input = syn::parse_str(&source)?;
        let bytes = expand(&input)?.to_string().len();
        let mut expansion = [0.0; 3];
        let mut with_parsing = [0.0; 3];

        for sample in 0..3 {
            let start = Instant::now();
            for _ in 0..repeats {
                black_box(expand(black_box(&input))?);
            }
            expansion[sample] = start.elapsed().as_secs_f64() * 1e6 / repeats as f64;

            let start = Instant::now();
            for _ in 0..repeats {
                let input = syn::parse_str(black_box(&source))?;
                black_box(expand(&input)?);
            }
            with_parsing[sample] = start.elapsed().as_secs_f64() * 1e6 / repeats as f64;
        }

        expansion.sort_by(f64::total_cmp);
        with_parsing.sort_by(f64::total_cmp);
        println!(
            "{name}: expand={:.2}us parse+expand={:.2}us bytes={bytes}",
            expansion[1], with_parsing[1]
        );
    }

    Ok(())
}
