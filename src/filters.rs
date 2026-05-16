use serde::Serialize;
use serde_json;

pub fn json_encode<T: Serialize>(val: &T) -> askama::Result<String> {
    Ok(serde_json::to_string(val).unwrap_or_else(|_| "[]".to_string()))
}

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

pub fn string(s: &dyn std::fmt::Display) -> askama::Result<String> {
    Ok(s.to_string())
}

pub fn i32(s: &usize) -> askama::Result<i32> {
    Ok(*s as i32)
}

pub fn eq(s: &str, other: &str) -> askama::Result<bool> {
    Ok(s == other)
}


pub fn first_image(images: &[String]) -> askama::Result<String> {
    if let Some(img) = images.first() {
        Ok(img.clone())
    } else {
        Ok("https://images.unsplash.com/photo-1519741497674-611481863552?w=1400&q=85".to_string())
    }
}

pub fn default<T: std::fmt::Display>(s: &Option<T>, default_val: &str) -> askama::Result<String> {
    match s {
        Some(v) => Ok(v.to_string()),
        None => Ok(default_val.to_string()),
    }
}

pub fn eq_uuid_opt(opt: &Option<uuid::Uuid>, val: &uuid::Uuid) -> askama::Result<bool> {
    Ok(opt.as_ref() == Some(val))
}

pub fn split_last(s: &str, sep: &str) -> askama::Result<String> {
    Ok(s.split(sep).last().unwrap_or(s).to_string())
}
