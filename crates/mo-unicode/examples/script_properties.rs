//! Development-only exhaustive property dump, compressed into identical ranges.
use mo_unicode::script::{script, script_extensions};
use std::io::{self, Write};
fn main() -> io::Result<()> {
    let mut output = io::BufWriter::new(io::stdout().lock());
    let mut previous = None;
    let mut start = 0;
    let mut end = 0;
    for cp in 0..=0x10ffff {
        let Some(c) = char::from_u32(cp) else {
            continue;
        };
        let value = (script(c), script_extensions(c));
        if previous.is_some_and(|v| v != value) || (previous.is_some() && cp != end + 1) {
            let (primary, extensions) = previous.unwrap();
            writeln!(
                output,
                "{start:X};{end:X};{};{}",
                primary.tag(),
                extensions
                    .iter()
                    .map(|s| s.tag())
                    .collect::<Vec<_>>()
                    .join(" ")
            )?;
            start = cp;
        }
        previous = Some(value);
        end = cp;
    }
    let (primary, extensions) = previous.unwrap();
    writeln!(
        output,
        "{start:X};{end:X};{};{}",
        primary.tag(),
        extensions
            .iter()
            .map(|s| s.tag())
            .collect::<Vec<_>>()
            .join(" ")
    )
}
