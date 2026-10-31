use super::Result;

pub(super) fn parse(value: &str) -> Result<i64> {
    let value = value.trim().to_ascii_uppercase();
    let units = [
        ("GB", 1_000_000_000),
        ("MB", 1_000_000),
        ("TB", 1_000_000_000_000),
        ("B", 1),
    ];
    let (number, multiplier) = units
        .iter()
        .find_map(|(unit, multiplier)| {
            value
                .strip_suffix(unit)
                .map(|number| (number.trim(), *multiplier))
        })
        .unwrap_or((&value, 1));
    if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("Invalid quota. Use a byte count or a size such as 100GB.".into());
    }
    number
        .parse::<u64>()
        .ok()
        .and_then(|number| number.checked_mul(multiplier))
        .and_then(|bytes| i64::try_from(bytes).ok())
        .ok_or_else(|| "Quota is too large.".into())
}

pub(crate) fn default_quota() -> Result<i64> {
    parse(&std::env::var("AUDEXIS_QUOTA_BYTES").unwrap_or_else(|_| "100GB".into()))
}
