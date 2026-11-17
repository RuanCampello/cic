//! Tree printing of the AST

use crate::frontend::parser::{Expression, ExpressionKind, Program};

pub struct Tree<'p, 'i>(&'p Program<'i>);

/// A rendered subtree
struct Block {
    rows: Vec<String>,
    width: usize,
    root: usize,
}

impl std::fmt::Display for Tree<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let declarations = self
            .0
            .declarations
            .iter()
            .map(|declaration| Block::binding(&declaration.name.kind, &declaration.value).rows);

        let result = std::iter::once(section("return", Block::of(&self.0.result).rows));

        Ok(for (i, rows) in declarations.chain(result).enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            rows.iter().try_for_each(|row| writeln!(f, "{row}"))?;
        })
    }
}

impl<'i> Program<'i> {
    pub fn tree(&self) -> Tree<'_, 'i> {
        Tree(self)
    }
}

impl Block {
    fn of(expr: &Expression) -> Self {
        match &expr.kind {
            ExpressionKind::Integer(int) => Self::leaf(int.to_string()),
            ExpressionKind::Variable(name) => Self::leaf(name.to_string()),
            ExpressionKind::Binary { left, operator, right } => {
                Self::join(operator.as_str(), Self::of(left), Self::of(right))
            },
        }
    }

    fn binding(name: &str, value: &Expression) -> Self {
        Self::join("=", Self::leaf(name.to_string()), Self::of(value))
    }

    fn leaf(label: String) -> Self {
        let width = label.len();
        Self { root: (width - 1) / 2, rows: vec![label], width }
    }

    fn join(operator: &str, left: Block, right: Block) -> Self {
        // children roots need to be at least `3 + operator width` columns apart, and an
        // even distance apart, so `/`, the operator and `\` each get their own columns
        let minimum = (3 + operator.len()).next_multiple_of(2);
        let distance = left.width - left.root + right.root;
        let mut gap = minimum.saturating_sub(distance).max(1);
        gap += (distance + gap) % 2;

        let offset = left.width + gap;
        let width = offset + right.width;
        let (left_root, right_root) = (left.root, offset + right.root);
        let root = (left_root + right_root) / 2;

        let mut top = vec![b' '; width];
        top[left_root + 2..right_root - 1].fill(b'_');
        let operator_start = root - (operator.len() - 1) / 2;
        top[operator_start..operator_start + operator.len()].copy_from_slice(operator.as_bytes());

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

fn section(label: &str, rows: Vec<String>) -> Vec<String> {
    let indent = |row: String| match row.is_empty() {
        true => row,
        _ => format!("  {row}"),
    };

    std::iter::once(label.to_string()).chain(rows.into_iter().map(indent)).collect()
}
