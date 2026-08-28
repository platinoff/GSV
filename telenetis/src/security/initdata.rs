use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Maximum allowed age (seconds) between a Telegram `initData` `auth_date`
/// and the server's current time before the handshake is treated as stale.
pub const DEFAULT_MAX_AGE_SECS: u64 = 86_400;

/// Failure kinds for Telegram Mini App `initData` verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitDataError {
    MissingHash,
    MissingAuthDate,
    MissingUser,
    Malformed,
    Stale,
    SignatureMismatch,
}

impl std::fmt::Display for InitDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::MissingHash => "initData has no hash field",
            Self::MissingAuthDate => "initData has no auth_date field",
            Self::MissingUser => "initData has no user field",
            Self::Malformed => "initData is malformed",
            Self::Stale => "initData auth_date is stale",
            Self::SignatureMismatch => "initData signature mismatch",
        };
        write!(f, "{}", msg)
    }
}

/// Verify the Telegram Mini App `initData` query string against `bot_token`.
///
/// Implements the Telegram Bot API algorithm:
/// 1. Split `init_data` into `key=value` pairs on `&` (skipping `hash`).
/// 2. Sort keys alphabetically and join as `key=value` lines with `\n`.
/// 3. `secret_key = HMAC_SHA256(key="WebAppData", msg=bot_token)`.
/// 4. `expected = HMAC_SHA256(key=secret_key, msg=data_check_string)`.
/// 5. Constant-time compare `expected` with the supplied `hash` (hex).
///
/// Also enforces that `auth_date` is present and within `max_age_secs` of
/// `now_unix`, and that a `user` field is present (a Mini App handshake
/// without a user is meaningless for coordination).
pub fn verify_init_data(
    init_data: &str,
    bot_token: &str,
    now_unix: i64,
    max_age_secs: u64,
) -> Result<(), InitDataError> {
    let pairs = parse_pairs(init_data)?;

    let supplied_hash = pairs
        .iter()
        .find(|(k, _)| k == "hash")
        .map(|(_, v)| v.as_str())
        .ok_or(InitDataError::MissingHash)?;

    let auth_date: i64 = pairs
        .iter()
        .find(|(k, _)| k == "auth_date")
        .map(|(_, v)| v.parse().unwrap_or(-1))
        .ok_or(InitDataError::MissingAuthDate)?;
    if auth_date < 0 {
        return Err(InitDataError::MissingAuthDate);
    }

    if !pairs.iter().any(|(k, _)| k == "user") {
        return Err(InitDataError::MissingUser);
    }

    let age = now_unix.saturating_sub(auth_date);
    if age < 0 || age as u64 > max_age_secs {
        return Err(InitDataError::Stale);
    }

    let mut signable: Vec<&(String, String)> = pairs.iter().filter(|(k, _)| k != "hash").collect();
    signable.sort_by(|a, b| a.0.cmp(&b.0));
    let data_check_string = signable
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n");

    let expected = compute_hash(&data_check_string, bot_token);

    if !constant_time_eq_hex(expected.as_slice(), supplied_hash) {
        return Err(InitDataError::SignatureMismatch);
    }

    Ok(())
}

/// Parse an `initData` query string into `(key, value)` pairs. The input is
/// URL-encoded; values are decoded with the same decoding the Telegram client
/// applies (percent-decode). Pair order is preserved but does not matter for
/// signing because the signable form is re-sorted.
fn parse_pairs(init_data: &str) -> Result<Vec<(String, String)>, InitDataError> {
    let mut pairs = Vec::new();
    for raw in init_data.split('&') {
        if raw.is_empty() {
            continue;
        }
        let mut it = raw.splitn(2, '=');
        let key = it
            .next()
            .filter(|k| !k.is_empty())
            .ok_or(InitDataError::Malformed)?;
        let value = it.next().unwrap_or("");
        pairs.push((percent_decode(key), percent_decode(value)));
    }
    if pairs.is_empty() {
        return Err(InitDataError::Malformed);
    }
    Ok(pairs)
}

/// Percent-decode a field value the way the Telegram client encodes it
/// (query-string style, UTF-8 bytes).
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = hex_val(bytes[i + 1]);
            let lo = hex_val(bytes[i + 2]);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn compute_hash(data_check_string: &str, bot_token: &str) -> Vec<u8> {
    let secret_key = secret_key(bot_token);
    let mut mac = <HmacSha256 as Mac>::new_from_slice(&secret_key).unwrap();
    mac.update(data_check_string.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn secret_key(bot_token: &str) -> Vec<u8> {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(b"WebAppData").unwrap();
    mac.update(bot_token.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn constant_time_eq_hex(expected: &[u8], supplied_hex: &str) -> bool {
    let supplied = hex_decode(supplied_hex);
    match supplied {
        Some(bytes) if bytes.len() == expected.len() => {
            let mut diff = 0u8;
            for (a, b) in expected.iter().zip(bytes.iter()) {
                diff |= a ^ b;
            }
            diff == 0
        }
        _ => false,
    }
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_val(bytes[i])?;
        let lo = hex_val(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Independent reference vector computed with OpenSSL (HMAC-SHA256,
    // key = "WebAppData", message = bot_token). See gen_vector.sh at band 214.
    const REF_TOKEN: &str = "test_bot_token_123";
    const REF_SECRET_HEX: &str = "06ef17f532bee79df8de5456b5d99add6cf3ed79c3c4a537f2244e6f0828c442";
    const REF_HASH_HEX: &str = "bd2a98e64f0fa8c7a4a585f3e8d8edceb4142f3499476714f88d7b43a3964422";
    const REF_AUTH_DATE: i64 = 1_750_000_000;

    fn ref_data_check_string() -> String {
        format!(
            "auth_date={}\nquery_id=AAHdF6IQAAAAAN0XohDhrOrc\nuser={{\"id\":279058397,\"first_name\":\"Vlad\",\"language_code\":\"en\"}}",
            REF_AUTH_DATE
        )
    }

    fn ref_init_data() -> String {
        let user = "{\"id\":279058397,\"first_name\":\"Vlad\",\"language_code\":\"en\"}";
        format!(
            "auth_date={}&query_id=AAHdF6IQAAAAAN0XohDhrOrc&user={}&hash={}",
            REF_AUTH_DATE,
            percent_encode(user),
            REF_HASH_HEX
        )
    }

    // Re-encode helpers so the test literal matches Telegram client encoding.
    fn percent_encode(input: &str) -> String {
        input
            .as_bytes()
            .iter()
            .map(|&b| match b {
                b'!'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b'-'
                | b'.'
                | b'_'
                | b'~'
                | b'a'..=b'z'
                | b'A'..=b'Z'
                | b'0'..=b'9' => (b as char).to_string(),
                _ => format!("%{:02X}", b),
            })
            .collect()
    }

    #[test]
    fn secret_key_matches_independent_reference() {
        let key = secret_key(REF_TOKEN);
        let hex = key.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        assert_eq!(hex, REF_SECRET_HEX);
    }

    #[test]
    fn hash_matches_independent_reference() {
        let h = compute_hash(&ref_data_check_string(), REF_TOKEN);
        let hex = h.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        assert_eq!(hex, REF_HASH_HEX);
    }

    #[test]
    fn verify_accepts_valid_init_data() {
        let result = verify_init_data(&ref_init_data(), REF_TOKEN, REF_AUTH_DATE + 10, 86_400);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_rejects_tampered_user() {
        let user = "{\"id\":999999,\"first_name\":\"Eve\",\"language_code\":\"en\"}";
        let forged = format!(
            "auth_date={}&query_id=AAHdF6IQAAAAAN0XohDhrOrc&user={}&hash={}",
            REF_AUTH_DATE,
            percent_encode(user),
            REF_HASH_HEX
        );
        assert!(matches!(
            verify_init_data(&forged, REF_TOKEN, REF_AUTH_DATE + 10, 86_400),
            Err(InitDataError::SignatureMismatch)
        ));
    }

    #[test]
    fn verify_rejects_wrong_bot_token() {
        let result = verify_init_data(
            &ref_init_data(),
            "another_token",
            REF_AUTH_DATE + 10,
            86_400,
        );
        assert!(matches!(result, Err(InitDataError::SignatureMismatch)));
    }

    #[test]
    fn verify_rejects_missing_hash() {
        let user = "{\"id\":1,\"first_name\":\"A\",\"language_code\":\"en\"}";
        let no_hash = format!(
            "auth_date={}&query_id=x&user={}",
            REF_AUTH_DATE,
            percent_encode(user)
        );
        assert!(matches!(
            verify_init_data(&no_hash, REF_TOKEN, REF_AUTH_DATE + 10, 86_400),
            Err(InitDataError::MissingHash)
        ));
    }

    #[test]
    fn verify_rejects_missing_user() {
        let no_user = format!(
            "auth_date={}&query_id=x&hash={}",
            REF_AUTH_DATE, REF_HASH_HEX
        );
        // hash is over a string without user, so it must differ from REF for
        // a token; before reaching hash compare we should fail on MissingUser.
        assert!(matches!(
            verify_init_data(&no_user, REF_TOKEN, REF_AUTH_DATE + 10, 86_400),
            Err(InitDataError::MissingUser)
        ));
    }

    #[test]
    fn verify_rejects_stale_auth_date() {
        let fresh = REF_AUTH_DATE + 10;
        let old = verified_data_with_auth_date(REF_AUTH_DATE - 100_000);
        assert!(matches!(
            verify_init_data(&old, REF_TOKEN, fresh, 86_400),
            Err(InitDataError::Stale)
        ));
    }

    #[test]
    fn verify_rejects_future_auth_date() {
        let future = REF_AUTH_DATE + 200_000;
        let data = verified_data_with_auth_date(future);
        assert!(matches!(
            verify_init_data(&data, REF_TOKEN, REF_AUTH_DATE, 86_400),
            Err(InitDataError::Stale)
        ));
    }

    // Build a genuinely signed initData for a given auth_date using the same
    // construction, so stale/future tests carry a valid signature for that
    // date and fail only on the freshness gate.
    fn verified_data_with_auth_date(auth_date: i64) -> String {
        let user = "{\"id\":279058397,\"first_name\":\"Vlad\",\"language_code\":\"en\"}";
        let data_check = format!(
            "auth_date={}\nquery_id=AAHdF6IQAAAAAN0XohDhrOrc\nuser={}",
            auth_date, user
        );
        let h = compute_hash(&data_check, REF_TOKEN);
        let hash_hex = h.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        format!(
            "auth_date={}&query_id=AAHdF6IQAAAAAN0XohDhrOrc&user={}&hash={}",
            auth_date,
            percent_encode(user),
            hash_hex
        )
    }

    #[test]
    fn constant_time_hex_compare() {
        assert!(constant_time_eq_hex(&[0xab, 0xcd], "abcd"));
        assert!(!constant_time_eq_hex(&[0xab, 0xcd], "abce"));
        assert!(!constant_time_eq_hex(&[0xab, 0xcd], "abcd00"));
        assert!(!constant_time_eq_hex(&[0xab, 0xcd], "ab"));
        assert!(!constant_time_eq_hex(&[0xab, 0xcd], "not-hex"));
    }

    #[test]
    fn percent_decode_handles_json_user() {
        assert_eq!(percent_decode("a%7Bb%7Dc"), "a{b}c");
        assert_eq!(percent_decode("plain"), "plain");
        assert_eq!(percent_decode("sp%20ace"), "sp ace");
    }
}
