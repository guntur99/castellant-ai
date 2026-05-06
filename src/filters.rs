pub fn round(s: &f64, precision: usize) -> askama::Result<String> {
    Ok(format!("{:.1$}", s, precision))
}

pub fn mult(s: usize, factor: f64) -> askama::Result<f64> {
    Ok((s as f64) * factor)
}

pub fn first(s: &str) -> askama::Result<String> {
    Ok(s.chars().next().unwrap_or(' ').to_string())
}

pub fn length(s: &str) -> askama::Result<usize> {
    Ok(s.len())
}
