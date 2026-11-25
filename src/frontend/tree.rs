//! Tree printing of the AST

use crate::frontend::parser::{
    Body, Command, CommandKind, Expression, ExpressionKind, Function, Item, Program,
};

pub struct Tree<'p, 'i>(&'p Program<'i>);

/// A rendered subtree
struct Block {
    rows: Vec<String>,
    width: usize,
    root: usize,
}

impl std::fmt::Display for Tree<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let items = self.0.items.iter().map(|item| match item {
            Item::Var(var) => Block::binding(var.name.kind, &var.value).rows,
            Item::Fn(function) => function_rows(function),
        });
        let main = std::iter::once(section("main", body_rows(Vec::new(), &self.0.main)));

        Ok(for (i, rows) in items.chain(main).enumerate() {
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
            ExpressionKind::Call { name, args } => {
                Self::call(&format!("{}()", name.kind), args.iter().map(Self::of).collect())
            },
        }
    }

    /// a node with any number of children side by side, `|` marks the one right below it
    fn call(label: &str, children: Vec<Block>) -> Self {
        if children.is_empty() {
            return Self::leaf(label.to_string());
        }

        let mut offsets = Vec::with_capacity(children.len());
        let mut width = 0;
        for child in &children {
            offsets.push(width);
            width += child.width + 2;
        }
        width -= 2;

        let roots: Vec<_> =
            children.iter().zip(&offsets).map(|(child, at)| at + child.root).collect();
        let (first, last) = (roots[0], roots[roots.len() - 1]);
        let half = (label.len() - 1) / 2;
        let shift = half.saturating_sub((first + last) / 2);
        let root = (first + last) / 2 + shift;
        let width = (width + shift).max(root - half + label.len());

        let mut branches = vec![b' '; width];
        for &child_root in &roots {
            let child_root = child_root + shift;
            match child_root.cmp(&root) {
                std::cmp::Ordering::Less => branches[child_root + 1] = b'/',
                std::cmp::Ordering::Greater => branches[child_root - 1] = b'\\',
                std::cmp::Ordering::Equal => branches[child_root] = b'|',
            }
        }

        let mut top = vec![b' '; width];
        let ends = branches.iter().enumerate().filter(|(_, b)| **b != b' ').map(|(i, _)| i);
        let (left, right) = (ends.clone().min().unwrap_or(root), ends.max().unwrap_or(root));
        if left + 1 < right {
            top[left + 1..right].fill(b'_');
        }
        top[root - half..root - half + label.len()].copy_from_slice(label.as_bytes());

        let mut rows: Vec<_> = [top, branches]
            .into_iter()
            .map(|row| String::from_utf8(row).expect("tree rows to be ascii"))
            .collect();

        let height = children.iter().map(|child| child.rows.len()).max().unwrap_or(0);
        for i in 0..height {
            let mut row = " ".repeat(shift);
            for (child, at) in children.iter().zip(&offsets) {
                let cell = child.rows.get(i).map_or("", String::as_str);
                row = format!("{row:<width$}{cell}", width = shift + at);
            }
            rows.push(row);
        }

        for row in &mut rows {
            row.truncate(row.trim_end().len());
        }

        Self { rows, width, root }
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

fn command_rows(command: &Command) -> Vec<String> {
    match &command.kind {
        CommandKind::Assign { name, value } => Block::binding(name.kind, value).rows,
        CommandKind::If { condition, then, otherwise } => {
            let mut rows = Block::of(condition).rows;
            rows.extend(section("then", commands_rows(then)));
            rows.extend(section("else", commands_rows(otherwise)));
            section("if", rows)
        },
        CommandKind::While { condition, body } => {
            let mut rows = Block::of(condition).rows;
            rows.extend(section("do", commands_rows(body)));
            section("while", rows)
        },
    }
}

fn function_rows(function: &Function) -> Vec<String> {
    let params: Vec<_> = function.params.iter().map(|param| param.kind).collect();
    let header = format!("fun {}({})", function.name.kind, params.join(", "));
    let locals: Vec<_> = function
        .locals
        .iter()
        .map(|var| Block::binding(var.name.kind, &var.value).rows)
        .collect();

    section(&header, body_rows(locals, &function.body))
}

fn body_rows(locals: Vec<Vec<String>>, body: &Body) -> Vec<String> {
    let commands = body.commands.iter().map(command_rows);
    let result = std::iter::once(section("return", Block::of(&body.result).rows));

    let mut rows = Vec::new();
    for (i, block) in locals.into_iter().chain(commands).chain(result).enumerate() {
        if i > 0 {
            rows.push(String::new());
        }
        rows.extend(block);
    }
    rows
}

fn commands_rows(commands: &[Command]) -> Vec<String> {
    let mut rows = Vec::new();

    for (i, command) in commands.iter().enumerate() {
        if i > 0 {
            rows.push(String::new());
        }
        rows.extend(command_rows(command));
    }

    rows
}

fn section(label: &str, rows: Vec<String>) -> Vec<String> {
    let indent = |row: String| match row.is_empty() {
        true => row,
        _ => format!("  {row}"),
    };

    std::iter::once(label.to_string()).chain(rows.into_iter().map(indent)).collect()
}
