use rand::random;
use time::error::ComponentRange;
use time::OffsetDateTime;

pub fn time_millis() -> Result<OffsetDateTime, ComponentRange> {
    let timestamp = OffsetDateTime::now_utc();
    timestamp.replace_nanosecond(timestamp.nanosecond() / 1_000_000 * 1_000_000)
}

pub fn snowflake(timestamp: OffsetDateTime) -> Vec<u8> {
    let millis = (timestamp.unix_timestamp_nanos() / 1_000_000) as u64;
    let random = (random::<u32>() % (1 << 20)) as u64;
    (millis << 20 | random).to_be_bytes().to_vec()
}
