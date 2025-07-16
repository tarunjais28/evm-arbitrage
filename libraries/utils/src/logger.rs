#[macro_export]
macro_rules! debug_time {
    ($label:expr, $block:block) => {{
        use std::time::Instant;
        let start = Instant::now();
        let result = $block;
        log::debug!("{} took {:?}", $label, start.elapsed());
        result
    }};
}
#[macro_export]
macro_rules! info_time {
    ($label:expr, $block:block) => {{
        use std::time::Instant;
        let start = Instant::now();
        let result = $block;
        log::info!("{} took {:?}", $label, start.elapsed());
        result
    }};
}
