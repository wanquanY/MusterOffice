//! A line plan derived from immutable paragraph context; independent of fonts.
use super::*;
use mo_unicode::TextSegmentation;
pub(crate) struct LinePlan {
    pub items: Vec<TextItem>,
    pub notices: Vec<ItemizationNotice>,
    pub lines: Vec<ShapedLine>,
    pub cascade_items: Vec<CascadeItem>,
    pub indices: Vec<u32>,
    pub scopes: Vec<std::ops::Range<u32>>,
}
pub(crate) fn plan(
    q: &ParagraphShapeRequest,
    original: &ItemizationResult,
    segmentation: &TextSegmentation,
    bidi_lines: &[BidiLine],
    paragraph_level: u8,
    check: &dyn Fn() -> bool,
) -> Result<LinePlan, TextError> {
    let mut items: Vec<TextItem> = Vec::new();
    let mut lines = Vec::new();
    let mut notices = original.notices.clone();
    let mut mixed: std::collections::BTreeSet<_> = notices
        .iter()
        .filter(|n| n.kind == ItemizationNoticeKind::MixedLevelCluster)
        .map(|n| (n.start, n.end))
        .collect();
    for line in bidi_lines {
        let mut source_item = original
            .items
            .partition_point(|item| item.end.scalar_offset <= line.start.scalar_offset);
        let mut cluster = segmentation
            .boundaries
            .partition_point(|b| b.scalar_offset < line.start.scalar_offset);
        cancelled(check)?;
        let item_start = items.len();
        while cluster + 1 < segmentation.boundaries.len()
            && segmentation.boundaries[cluster].scalar_offset < line.end.scalar_offset
        {
            cancelled(check)?;
            let start = &segmentation.boundaries[cluster];
            let end = &segmentation.boundaries[cluster + 1];
            while start.scalar_offset >= original.items[source_item].end.scalar_offset {
                source_item += 1;
            }
            let source = &original.items[source_item];
            let levels = &line.levels[(start.scalar_offset - line.start.scalar_offset) as usize
                ..(end.scalar_offset - line.start.scalar_offset) as usize];
            let level = levels
                .iter()
                .copied()
                .flatten()
                .next()
                .unwrap_or(paragraph_level);
            if levels.iter().flatten().any(|&other| other != level)
                && mixed.insert((start.scalar_offset, end.scalar_offset))
            {
                if notices.len() >= 65536 {
                    return Err(TextError::Limit("line shaping notices"));
                }
                notices.push(ItemizationNotice {
                    start: start.scalar_offset,
                    end: end.scalar_offset,
                    kind: ItemizationNoticeKind::MixedLevelCluster,
                });
            }
            if items.len() > item_start
                && let Some(last) = items.last_mut()
                && last.kind == TextItemKind::Text
                && source.kind == TextItemKind::Text
                && last.level == level
                && last.style == source.style
                && last.script == source.script
            {
                last.end = end.clone();
            } else {
                if items.len() >= 4096 {
                    return Err(TextError::Limit("line shaping items"));
                }
                items.push(TextItem {
                    start: start.clone(),
                    end: end.clone(),
                    level,
                    ..source.clone()
                });
            }
            cluster += 1;
        }
        lines.push(ShapedLine {
            start: line.start.clone(),
            end: line.end.clone(),
            item_start: item_start as u32,
            item_end: items.len() as u32,
            fallback_start: 0,
            fallback_end: 0,
        });
    }
    let mut cascade_items = Vec::new();
    let mut indices = Vec::new();
    let mut scopes = Vec::new();
    for line in &mut lines {
        line.fallback_start = cascade_items.len() as u32;
        for index in line.item_start..line.item_end {
            cancelled(check)?;
            let part = &items[index as usize];
            if part.kind != TextItemKind::Text {
                continue;
            }
            let direction = if part.level.is_multiple_of(2) {
                Direction::LeftToRight
            } else {
                Direction::RightToLeft
            };
            let mut item = paragraph::item(
                part.start.scalar_offset,
                part.end.scalar_offset,
                direction,
                part.script.tag().into(),
                &q.styles[part.style as usize],
                segmentation.boundaries.last().unwrap().scalar_offset,
            );
            item.beginning_of_text = item.start == line.start.scalar_offset;
            item.end_of_text = item.end == line.end.scalar_offset;
            cascade_items.push(item);
            indices.push(index);
            scopes.push(line.start.scalar_offset..line.end.scalar_offset);
        }
        line.fallback_end = cascade_items.len() as u32;
    }
    Ok(LinePlan {
        items,
        notices,
        lines,
        cascade_items,
        indices,
        scopes,
    })
}
impl LinePlan {
    pub fn finish(self, bidi: BidiParagraphResult, fallback: FallbackResult) -> LineShapeResult {
        LineShapeResult {
            profile: "unicode18-paragraph-bidi-line-context-hb14.5-v1".into(),
            bidi,
            items: self.items,
            notices: self.notices,
            lines: self.lines,
            shaped_item_indices: self.indices,
            fallback,
        }
    }
}
