//! Development host: query every valid scalar through the actual Rust lookup.
use mo_unicode::line_break::line_break_properties;
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
        let p = line_break_properties(c);
        let value = (
            p.class,
            u8::from(p.combining_mark)
                | u8::from(p.initial_punctuation) << 1
                | u8::from(p.final_punctuation) << 2
                | u8::from(p.unassigned) << 3
                | u8::from(p.east_asian) << 4,
        );
        if previous.is_some_and(|v| v != value) || (previous.is_some() && cp != end + 1) {
            let (class, flags) = previous.unwrap();
            writeln!(
                output,
                "{start:X};{end:X};{};{flags}",
                format!("{class:?}").to_uppercase()
            )?;
            start = cp;
        }
        previous = Some(value);
        end = cp;
    }
    let (class, flags) = previous.unwrap();
    writeln!(
        output,
        "{start:X};{end:X};{};{flags}",
        format!("{class:?}").to_uppercase()
    )
}
