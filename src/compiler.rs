pub mod resources {
    pub const TEMPLATE: &str = include_str!("../asm/modelo.s");
    pub const RUNTIME: &str = include_str!("../asm/runtime.s");
    pub const MARKER: &str = "  ## saida do compilador deve ser inserida aqui";
}

pub fn compile(src: &str) -> String {
    let constant = parse_integer_const(src);
    resources::TEMPLATE.replace(resources::MARKER, &format!("  mov ${constant}, &rax"))
}

fn parse_integer_const(src: &str) -> u64 {
    src.trim().parse::<u64>().expect("a valid constant")
}
