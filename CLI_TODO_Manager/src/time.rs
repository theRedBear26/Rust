
use chrono::NaiveDate;

pub fn validate_date(date: &str) -> Result<(), String> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| {
            "Invalid date. Use YYYY-MM-DD format.".to_string()
        })
}

pub fn format_date(date: &str) -> String {
    match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(parsed) => parsed.format("%d %b %Y").to_string(),
        Err(_) => date.to_string(),
    }
}
