const MAX_EDITS: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Edit {
    Keep,
    Insert,
    Delete,
}

pub(super) fn edit_script(old: &[&str], new: &[&str]) -> Option<Vec<Edit>> {
    let prefix = old
        .iter()
        .zip(new)
        .take_while(|(left, right)| left == right)
        .count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(left, right)| left == right)
        .count();
    let middle = shortest_edits(
        &old[prefix..old.len() - suffix],
        &new[prefix..new.len() - suffix],
    )?;
    let mut script = vec![Edit::Keep; prefix];
    script.extend(middle);
    script.extend(std::iter::repeat_n(Edit::Keep, suffix));
    Some(script)
}

fn shortest_edits(old: &[&str], new: &[&str]) -> Option<Vec<Edit>> {
    let (old_len, new_len) = (old.len() as isize, new.len() as isize);
    let limit = (old.len() + new.len()).min(MAX_EDITS) as isize;
    let offset = limit + 1;
    let mut furthest = vec![0isize; (2 * limit + 3) as usize];
    let mut trace: Vec<Vec<isize>> = Vec::new();
    for depth in 0..=limit {
        for diagonal in (-depth..=depth).step_by(2) {
            let index = (offset + diagonal) as usize;
            let mut x = if moves_down(&furthest, index, diagonal, depth) {
                furthest[index + 1]
            } else {
                furthest[index - 1] + 1
            };
            let mut y = x - diagonal;
            while x < old_len && y < new_len && old[x as usize] == new[y as usize] {
                x += 1;
                y += 1;
            }
            furthest[index] = x;
            if x >= old_len && y >= new_len {
                return Some(backtrack(&trace, old_len, new_len));
            }
        }
        let window = (offset - depth) as usize..=(offset + depth) as usize;
        trace.push(furthest[window].to_vec());
    }
    None
}

fn moves_down(furthest: &[isize], index: usize, diagonal: isize, depth: isize) -> bool {
    diagonal == -depth || (diagonal != depth && furthest[index - 1] < furthest[index + 1])
}

fn backtrack(trace: &[Vec<isize>], old_len: isize, new_len: isize) -> Vec<Edit> {
    let (mut x, mut y) = (old_len, new_len);
    let mut script = Vec::new();
    for depth in (1..=trace.len() as isize).rev() {
        let previous = &trace[(depth - 1) as usize];
        let at = |diagonal: isize| previous[(diagonal + depth - 1) as usize];
        let diagonal = x - y;
        let down = diagonal == -depth || (diagonal != depth && at(diagonal - 1) < at(diagonal + 1));
        let previous_diagonal = if down { diagonal + 1 } else { diagonal - 1 };
        let previous_x = at(previous_diagonal);
        let previous_y = previous_x - previous_diagonal;
        let step_x = if down { previous_x } else { previous_x + 1 };
        let snake = (x - step_x) as usize;
        script.extend(std::iter::repeat_n(Edit::Keep, snake));
        script.push(if down { Edit::Insert } else { Edit::Delete });
        x = previous_x;
        y = previous_y;
    }
    script.extend(std::iter::repeat_n(Edit::Keep, x as usize));
    script.reverse();
    script
}
