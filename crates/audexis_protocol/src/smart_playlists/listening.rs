use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListeningPeriod {
    pub year: i32,
    pub month: Option<u32>,
}

impl ListeningPeriod {
    pub fn bounds(self) -> Result<(String, String), String> {
        if !(1970..=9998).contains(&self.year)
            || self.month.is_some_and(|month| !(1..=12).contains(&month))
        {
            return Err("Choose a valid year and month.".into());
        }
        let start = format!("{:04}-{:02}-01", self.year, self.month.unwrap_or(1));
        let end = match self.month {
            Some(month) if month < 12 => format!("{:04}-{:02}-01", self.year, month + 1),
            _ => format!("{:04}-01-01", self.year + 1),
        };
        Ok((start, end))
    }

    pub fn playlist_name(self) -> Result<String, String> {
        self.bounds()?;
        let period = match self.month {
            Some(month) => {
                let months = [
                    "January",
                    "February",
                    "March",
                    "April",
                    "May",
                    "June",
                    "July",
                    "August",
                    "September",
                    "October",
                    "November",
                    "December",
                ];
                format!("{} {}", months[month as usize - 1], self.year)
            }
            None => self.year.to_string(),
        };
        Ok(format!("Rewind - {period}"))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ListeningStats {
    pub plays: i64,
    pub listened_us: i64,
}
