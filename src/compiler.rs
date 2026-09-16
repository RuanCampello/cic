pub mod resources {
    pub const TEMPLATE: &str = include_str!("../asm/modelo.s");
    pub const RUNTIME: &str = include_str!("../asm/runtime.s");
    pub const MARKER: &str = "  ## saida do compilador deve ser inserida aqui";
}

pub fn compile(src: &str) -> String {
    let constant = parse_integer_const(src);
    resources::TEMPLATE.replace(resources::MARKER, &format!("  mov ${constant}, %rax"))
}

fn parse_integer_const(src: &str) -> u64 {
    src.trim().parse::<u64>().expect("a valid constant")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_valid_constant() {
        let output = compile("42");
        assert!(output.contains("mov $42, %rax"));
        assert!(!output.contains(resources::MARKER));
    }

    #[test]
    fn removes_whitespaces() {
        let output = compile("     7\n");
        assert!(output.contains("mov $7, %rax"));
        assert!(!output.contains(resources::MARKER));
    }

    #[test]
    #[should_panic(expected = "a valid constant")]
    fn invalid_constant() {
        compile("abc");
    }
}
