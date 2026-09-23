//! Tree printing of the AST

use crate::frontend::parser::{Expression, ExpressionKind};

pub struct Tree<'expr, 'i>(&'expr Expression<'i>);

/// A rendered subtree
struct Block {
    rows: Vec<String>,
    width: usize,
    root: usize,
}

impl std::fmt::Display for Tree<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Block::of(self.0).rows.iter().try_for_each(|row| writeln!(f, "{row}"))
    }
}

impl<'i> Expression<'i> {
    pub fn tree(&self) -> Tree<'_, 'i> {
        Tree(self)
    }
}

impl Block {
    fn of(expr: &Expression) -> Self {
        match &expr.kind {
            ExpressionKind::Integer(int) => Self::leaf(int.to_string()),
            ExpressionKind::Binary { left, operator, right } => {
                Self::join(char::from(*operator), Self::of(left), Self::of(right))
            },
            ExpressionKind::_M(_) => unreachable!(),
        }
    }

    fn leaf(label: String) -> Self {
        let width = label.len();
        Self { root: (width - 1) / 2, rows: vec![label], width }
    }

    fn join(operator: char, left: Block, right: Block) -> Self {
        // children roots need to be at least 4 columns apart, and an even distance
        // apart, so `/`, the operator and `\` each get their own centered column
        let distance = left.width - left.root + right.root;
        let mut gap = 4usize.saturating_sub(distance).max(1);
        gap += (distance + gap) % 2;

        let offset = left.width + gap;
        let width = offset + right.width;
        let (left_root, right_root) = (left.root, offset + right.root);
        let root = (left_root + right_root) / 2;

        let mut top = vec![b' '; width];
        top[left_root + 2..right_root - 1].fill(b'_');
        top[root] = operator as u8;

        let mut branches = vec![b' '; width];
        branches[left_root + 1] = b'/';
        branches[right_root - 1] = b'\\';

        let mut rows: Vec<_> = [top, branches]
            .into_iter()
            .map(|row| String::from_utf8(row).expect("tree rows to be ascii"))
            .collect();

        for i in 0..left.rows.len().max(right.rows.len()) {
            let l = left.rows.get(i).map_or("", String::as_str);
            let r = right.rows.get(i).map_or("", String::as_str);
            rows.push(format!("{l:<offset$}{r}"));
        }

        for row in &mut rows {
            row.truncate(row.trim_end().len());
        }

        Self { rows, width, root }
    }
}
