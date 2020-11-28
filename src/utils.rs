use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};

pub fn timestamp_to_datetime(timestamp_in_nanoseconds: u128) -> DateTime<Local> {
    let date_time = NaiveDateTime::from_timestamp(
        (timestamp_in_nanoseconds / 1_000_000_000) as i64,
        (timestamp_in_nanoseconds % 1_000_000_000) as u32,
    );
    let date_time_local = DateTime::<Local>::from(DateTime::<Utc>::from_utc(date_time, Utc));
    date_time_local
}

pub fn _datetime_to_timestamp<T: TimeZone>(datetime: &DateTime<T>) -> u128 {
    (datetime.timestamp() * 1_000_000_000 + datetime.timestamp_nanos()) as u128
}
