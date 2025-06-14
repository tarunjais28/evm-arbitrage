pub fn format_with_decimals(value: u128, decimals: u32) -> String {
    let tens = 10u128.pow(decimals);
    let int_part = value / tens;
    let frac_part = value % tens;
    format!("{}.{:018}", int_part, frac_part)
}
