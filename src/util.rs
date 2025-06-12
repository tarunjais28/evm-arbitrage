pub fn format_with_decimals(value: u128) -> String {
    let int_part = value / 1_000_000_000_000_000_000u128;
    let frac_part = value % 1_000_000_000_000_000_000u128;
    format!("{}.{}", int_part, format!("{:018}", frac_part))
}
