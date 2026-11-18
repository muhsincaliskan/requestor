use super::{DiffRow, LineDiff};

fn kept(text: &str) -> DiffRow {
    DiffRow::Kept(text.to_string())
}

#[test]
fn changed_lines_show_as_removed_then_added_between_their_context() {
    let diff = LineDiff::between("a\nb\nc", "a\nB\nc").unwrap();
    assert_eq!(
        diff.rows,
        [
            kept("a"),
            DiffRow::Removed("b".to_string()),
            DiffRow::Added("B".to_string()),
            kept("c"),
        ]
    );
    assert_eq!((diff.added, diff.removed), (1, 1));
}

#[test]
fn long_unchanged_stretches_are_folded_around_the_context() {
    let before: Vec<String> = (0..20).map(|line| line.to_string()).collect();
    let mut after = before.clone();
    after[10] = "ten".to_string();
    let diff = LineDiff::between(&before.join("\n"), &after.join("\n")).unwrap();
    assert_eq!(diff.rows.first(), Some(&DiffRow::Folded(7)));
    assert_eq!(diff.rows.last(), Some(&DiffRow::Folded(6)));
    assert_eq!(diff.rows.len(), 2 + 3 + 2 + 3);
}

#[test]
fn insertions_and_deletions_anywhere_are_found() {
    let diff = LineDiff::between("x\na\nb\nc\ny", "a\nb\nz\nc").unwrap();
    assert_eq!((diff.added, diff.removed), (1, 2));
    let removed: Vec<&DiffRow> = diff
        .rows
        .iter()
        .filter(|row| matches!(row, DiffRow::Removed(_)))
        .collect();
    assert_eq!(
        removed,
        [
            &DiffRow::Removed("x".to_string()),
            &DiffRow::Removed("y".to_string())
        ]
    );
}

#[test]
fn identical_and_empty_texts_have_no_changes() {
    assert!(LineDiff::between("a\nb", "a\nb").unwrap().is_unchanged());
    assert!(LineDiff::between("", "").unwrap().is_unchanged());
    let fresh = LineDiff::between("", "a").unwrap();
    assert_eq!(fresh.rows, [DiffRow::Added("a".to_string())]);
}

#[test]
fn texts_too_different_to_compare_give_no_diff() {
    let before: String = (0..1200).map(|line| format!("old {line}\n")).collect();
    let after: String = (0..1200).map(|line| format!("new {line}\n")).collect();
    assert!(LineDiff::between(&before, &after).is_none());
}

#[test]
fn every_edit_script_turns_the_old_lines_into_the_new_ones() {
    use super::edits::{edit_script, Edit};
    let mut seed: u32 = 7;
    let mut next = move || {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (seed >> 16) % 4
    };
    for _ in 0..200 {
        let old: Vec<String> = (0..next() * 3).map(|_| next().to_string()).collect();
        let new: Vec<String> = (0..next() * 3).map(|_| next().to_string()).collect();
        let old: Vec<&str> = old.iter().map(String::as_str).collect();
        let new: Vec<&str> = new.iter().map(String::as_str).collect();
        let script = edit_script(&old, &new).unwrap();
        let (mut at_old, mut rebuilt) = (0, Vec::new());
        for edit in &script {
            match edit {
                Edit::Keep => {
                    rebuilt.push(old[at_old]);
                    at_old += 1;
                }
                Edit::Delete => at_old += 1,
                Edit::Insert => rebuilt.push(new[rebuilt.len()]),
            }
        }
        assert_eq!(at_old, old.len());
        assert_eq!(rebuilt, new);
    }
}
