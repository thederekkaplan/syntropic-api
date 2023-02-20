use rand::random;
use time::OffsetDateTime;

pub fn snowflake(timestamp: OffsetDateTime) -> Vec<u8> {
    let millis = (timestamp.unix_timestamp_nanos() / 1_000_000) as u64;
    let random = (random::<u32>() % (1 << 20)) as u64;
    (millis << 20 | random).to_be_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::parallel;

    #[test]
    #[parallel]
    fn test_snowflake() {
        let timestamp = OffsetDateTime::from_unix_timestamp_nanos(946684800000000000).unwrap();
        let id = snowflake(timestamp);
        assert_eq!(id.len(), 8);
        assert_eq!(id[0], 13);
        assert_eq!(id[1], 198);
        assert_eq!(id[2], 172);
        assert_eq!(id[3], 250);
        assert_eq!(id[4], 192);
        println!("{:?}", id);
    }
}
