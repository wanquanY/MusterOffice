use mo_unicode::{UnicodeLimits, line_break::analyze_line_breaks};
use std::{error::Error, fs};
fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("expected official LineBreakTest path")?;
    let input = fs::read_to_string(path)?;
    let mut cases = 0;
    let mut failures = 0;
    for (line_number, line) in input.lines().enumerate() {
        let data = line.split('#').next().unwrap().trim();
        if data.is_empty() {
            continue;
        }
        let mut text = String::new();
        let mut expected = Vec::new();
        let mut count = 0;
        for part in data.split_whitespace() {
            match part {
                "÷" => expected.push(count),
                "×" => {}
                hex => {
                    let cp = u32::from_str_radix(hex, 16)?;
                    text.push(char::from_u32(cp).ok_or("non-scalar official test")?);
                    count += 1;
                }
            }
        }
        let result = analyze_line_breaks(&text, UnicodeLimits::default(), &|| false)?;
        let actual: Vec<_> = result
            .opportunities
            .iter()
            .map(|b| b.boundary.scalar_offset)
            .collect();
        cases += 1;
        if actual != expected {
            failures += 1;
            if failures <= 20 {
                eprintln!(
                    "{}: expected {:?}, actual {:?}: {}",
                    line_number + 1,
                    expected,
                    actual,
                    line
                );
            }
        }
    }
    println!("{{\"cases\":{cases},\"failures\":{failures},\"skipped\":0}}");
    if failures > 0 {
        return Err("official line break differences".into());
    }
    Ok(())
}
