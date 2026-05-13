pub fn binary_stem() -> &'static str {
    crate::storage::product_flavor().binary_stem()
}

pub fn command_name() -> &'static str {
    binary_stem()
}

pub fn command_with(args: &str) -> String {
    if args.trim().is_empty() {
        command_name().to_string()
    } else {
        format!("{} {}", command_name(), args.trim())
    }
}
