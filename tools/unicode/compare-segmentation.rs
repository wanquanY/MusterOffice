//! Development comparison only; unicode-segmentation is not a kernel dependency.
use std::{env,fs};
use unicode_segmentation::UnicodeSegmentation;
fn main() {
    let source=fs::read_to_string(env::args().nth(1).expect("GraphemeBreakTest path")).unwrap();
    let mut tested=0;let mut differences=Vec::new();
    for (line_index,line) in source.lines().enumerate() {
        let body=line.split('#').next().unwrap().trim();if body.is_empty(){continue;}
        let mut text=String::new();let mut expected=Vec::new();
        for token in body.split_whitespace() {
            match token {"÷"=>expected.push(text.len()),"×"=>{},_=>text.push(char::from_u32(u32::from_str_radix(token,16).unwrap()).unwrap())}
        }
        let mut actual:Vec<_>=text.grapheme_indices(true).map(|(byte,_)|byte).collect();actual.push(text.len());
        tested+=1;if actual!=expected{differences.push(line_index+1);}
    }
    println!("{{\"version\":\"1.13.3\",\"unicodeVersion\":\"17.0.0\",\"tested\":{tested},\"differingLines\":{differences:?}}}");
}
