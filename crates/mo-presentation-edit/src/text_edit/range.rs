use super::*;

pub(super) struct Range {
    pub first: usize,
    pub last: usize,
    pub start: TextAnchor,
    pub end: TextAnchor,
}

pub(super) fn ordered(
    body: &TextBody,
    selection: &TextSelection,
    check: &dyn Fn() -> bool,
) -> Result<Range, EditError> {
    let locate = |a: &TextAnchor| -> Result<usize, EditError> {
        let i = body
            .paragraphs
            .iter()
            .position(|p| p.id == a.paragraph)
            .ok_or_else(|| EditError::input("text selection paragraph does not exist"))?;
        let text = plain(&body.paragraphs[i]);
        let segmented = segment(&text, check)?;
        if !segmented
            .boundaries
            .iter()
            .any(|b| b.scalar_offset == a.scalar_offset)
        {
            return Err(EditError::input("selection must be at a grapheme boundary"));
        }
        Ok(i)
    };
    let a = locate(&selection.anchor)?;
    let f = locate(&selection.focus)?;
    let (first, last, start, end) =
        if (a, selection.anchor.scalar_offset) <= (f, selection.focus.scalar_offset) {
            (a, f, selection.anchor.clone(), selection.focus.clone())
        } else {
            (f, a, selection.focus.clone(), selection.anchor.clone())
        };
    Ok(Range {
        first,
        last,
        start,
        end,
    })
}

pub(super) fn segment(
    text: &str,
    check: &dyn Fn() -> bool,
) -> Result<mo_unicode::TextSegmentation, EditError> {
    mo_unicode::segment(text, mo_unicode::UnicodeLimits::default(), check).map_err(|e| match e {
        mo_unicode::UnicodeError::Cancelled => EditError::Cancelled,
        e => EditError::input(e.to_string()),
    })
}

pub(super) fn plain(paragraph: &Paragraph) -> String {
    paragraph
        .runs
        .iter()
        .map(|run| match &run.content {
            InlineContent::Text { text } => text.as_str(),
            InlineContent::Break => "\n",
            InlineContent::Tab => "\t",
        })
        .collect()
}

pub(super) fn length(paragraph: &Paragraph) -> usize {
    paragraph.runs.iter().map(|r| r.content.scalar_len()).sum()
}

pub(super) fn slice_runs(
    paragraph: &Paragraph,
    start: usize,
    end: usize,
    check: &dyn Fn() -> bool,
) -> Result<Vec<TextRun>, EditError> {
    let mut offset = 0;
    let mut result = Vec::new();
    for run in &paragraph.runs {
        cancelled(check)?;
        let len = run.content.scalar_len();
        let from = start.saturating_sub(offset).min(len);
        let to = end.saturating_sub(offset).min(len);
        if from < to {
            let mut part = run.clone();
            if let InlineContent::Text { text } = &run.content {
                part.content = InlineContent::Text {
                    text: text.chars().skip(from).take(to - from).collect(),
                };
            }
            result.push(part);
        }
        offset += len;
    }
    Ok(result)
}

pub(super) fn style_at(p: &Paragraph, offset: usize, affinity: Affinity) -> CharacterStyle {
    let mut cursor = 0;
    for run in &p.runs {
        let end = cursor + run.content.scalar_len();
        if offset < end || (offset == end && affinity == Affinity::Before) {
            return run.style.clone();
        }
        cursor = end;
    }
    p.runs
        .last()
        .map_or_else(CharacterStyle::default, |r| r.style.clone())
}

pub(super) fn styled_runs(
    p: &Paragraph,
    start: usize,
    end: usize,
    patch: &CharacterStylePatch,
    check: &dyn Fn() -> bool,
) -> Result<Vec<TextRun>, EditError> {
    let mut offset = 0;
    let mut runs = Vec::new();
    for run in &p.runs {
        cancelled(check)?;
        let len = run.content.scalar_len();
        let from = start.saturating_sub(offset).min(len);
        let to = end.saturating_sub(offset).min(len);
        if from == to {
            runs.push(run.clone());
        } else {
            for (a, b, selected) in [(0, from, false), (from, to, true), (to, len, false)] {
                if a == b {
                    continue;
                }
                let mut part = run.clone();
                if let InlineContent::Text { text } = &run.content {
                    part.content = InlineContent::Text {
                        text: text.chars().skip(a).take(b - a).collect(),
                    };
                }
                if selected {
                    patch.apply(&mut part.style);
                }
                runs.push(part);
            }
        }
        offset += len;
    }
    Ok(runs)
}

pub(super) struct Ids {
    seed: Digest,
    next: u32,
}
impl Ids {
    pub fn new(command: &TextEditCommand) -> Result<Self, EditError> {
        Ok(Self {
            seed: digest("musteroffice.text-edit-ids/1", command)?,
            next: 0,
        })
    }
    fn next(&mut self) -> String {
        let value = format!("text:{}:{}", self.seed, self.next);
        self.next += 1;
        value
    }
    pub fn run(&mut self) -> RunId {
        RunId::new(self.next()).expect("bounded derived identity")
    }
    pub fn paragraph(&mut self) -> ParagraphId {
        ParagraphId::new(self.next()).expect("bounded derived identity")
    }
    pub fn unique_runs(&mut self, paragraphs: &mut [Paragraph]) {
        let mut seen = std::collections::BTreeSet::new();
        for p in paragraphs {
            for run in &mut p.runs {
                if !seen.insert(run.id.clone()) {
                    run.id = self.run();
                    seen.insert(run.id.clone());
                }
            }
        }
    }
}
