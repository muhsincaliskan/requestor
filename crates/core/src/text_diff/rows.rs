use super::edits::Edit;
use super::{DiffRow, MAX_ROW_BYTES};

pub(super) fn folded(script: &[Edit], old: &[&str], new: &[&str], context: usize) -> Vec<DiffRow> {
    let mut rows = Vec::new();
    let (mut old_at, mut new_at) = (0, 0);
    let mut start = 0;
    while start < script.len() {
        let end = run_end(script, start);
        match script[start] {
            Edit::Keep => {
                let kept = &new[new_at..new_at + end - start];
                push_kept(&mut rows, kept, start == 0, end == script.len(), context);
                old_at += end - start;
                new_at += end - start;
            }
            Edit::Insert => {
                rows.extend(
                    new[new_at..new_at + end - start]
                        .iter()
                        .map(|line| DiffRow::Added(clipped(line))),
                );
                new_at += end - start;
            }
            Edit::Delete => {
                rows.extend(
                    old[old_at..old_at + end - start]
                        .iter()
                        .map(|line| DiffRow::Removed(clipped(line))),
                );
                old_at += end - start;
            }
        }
        start = end;
    }
    rows
}

fn run_end(script: &[Edit], start: usize) -> usize {
    script[start..]
        .iter()
        .position(|edit| *edit != script[start])
        .map_or(script.len(), |length| start + length)
}

fn push_kept(rows: &mut Vec<DiffRow>, kept: &[&str], first: bool, last: bool, context: usize) {
    let head = if first { 0 } else { context.min(kept.len()) };
    let tail = if last {
        0
    } else {
        context.min(kept.len() - head)
    };
    let hidden = kept.len() - head - tail;
    let (head, tail, hidden) = if hidden == 1 {
        (head + 1, tail, 0)
    } else {
        (head, tail, hidden)
    };
    let shown = |lines: &[&str]| {
        lines
            .iter()
            .map(|line| DiffRow::Kept(clipped(line)))
            .collect::<Vec<_>>()
    };
    rows.extend(shown(&kept[..head]));
    if hidden > 0 {
        rows.push(DiffRow::Folded(hidden));
    }
    rows.extend(shown(&kept[kept.len() - tail..]));
}

fn clipped(line: &str) -> String {
    if line.len() <= MAX_ROW_BYTES {
        return line.to_string();
    }
    let mut end = MAX_ROW_BYTES;
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    line[..end].to_string()
}
