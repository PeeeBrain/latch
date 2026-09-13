use hmac::{Hmac, Mac};
use sha1::Sha1;

pub const PERIOD_SECS: u64 = 30;
pub const DIGITS: u32 = 6;

type HmacSha1 = Hmac<Sha1>;

/// Normalize a raw Base32 secret: trim, strip separators and padding, uppercase.
/// Rejects empty input and characters outside the RFC 4648 alphabet.
pub fn normalize_secret(input: &str) -> Result<String, String> {
    let normalized: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '=')
        .map(|c| c.to_ascii_uppercase())
        .collect();

    if normalized.is_empty() {
        return Err("TOTP secret cannot be empty".to_string());
    }
    if let Some(invalid) = normalized
        .chars()
        .find(|c| !matches!(c, 'A'..='Z' | '2'..='7'))
    {
        return Err(format!("Invalid Base32 character '{}'", invalid));
    }

    Ok(normalized)
}

/// Accept either a raw Base32 secret or an `otpauth://` provisioning URI.
pub fn extract_secret(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.to_ascii_lowercase().starts_with("otpauth://") {
        let url = url::Url::parse(trimmed).map_err(|e| format!("Invalid otpauth URI: {}", e))?;
        let secret = url
            .query_pairs()
            .find(|(key, _)| key == "secret")
            .map(|(_, value)| value.into_owned())
            .ok_or("otpauth URI is missing a secret parameter")?;
        normalize_secret(&secret)
    } else {
        normalize_secret(trimmed)
    }
}

/// Generate the 6-digit TOTP for `unix_time` (RFC 6238, SHA-1).
pub fn generate_token(secret: &str, unix_time: u64) -> Result<String, String> {
    let key = decode_secret(&normalize_secret(secret)?)?;
    let counter = unix_time / PERIOD_SECS;

    let mut mac =
        HmacSha1::new_from_slice(&key).map_err(|e| format!("Failed to init HMAC: {}", e))?;
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();

    let offset = (digest[19] & 0x0f) as usize;
    let binary = ((digest[offset] as u32 & 0x7f) << 24)
        | ((digest[offset + 1] as u32) << 16)
        | ((digest[offset + 2] as u32) << 8)
        | (digest[offset + 3] as u32);
    let code = binary % 10u32.pow(DIGITS);

    Ok(format!("{:0>width$}", code, width = DIGITS as usize))
}

/// Seconds remaining in the current 30-second window.
pub fn remaining_seconds(unix_time: u64) -> u64 {
    PERIOD_SECS - (unix_time % PERIOD_SECS)
}

fn decode_secret(normalized: &str) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::with_capacity(normalized.len() * 5 / 8);
    let mut buffer: u32 = 0;
    let mut bits_in_buffer: u32 = 0;

    for c in normalized.chars() {
        let value = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            '2'..='7' => c as u32 - '2' as u32 + 26,
            _ => return Err(format!("Invalid Base32 character '{}'", c)),
        };
        buffer = (buffer << 5) | value;
        bits_in_buffer += 5;
        if bits_in_buffer >= 8 {
            bits_in_buffer -= 8;
            bytes.push(((buffer >> bits_in_buffer) & 0xff) as u8);
        }
    }

    if bytes.is_empty() {
        return Err("TOTP secret is too short".to_string());
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 6238 test secret: ASCII "12345678901234567890".
    const RFC_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

    #[test]
    fn generates_rfc6238_sha1_vectors_as_six_digits() {
        let cases = [
            (59u64, "287082"),
            (1111111109, "081804"),
            (1111111111, "050471"),
            (1234567890, "005924"),
            (2000000000, "279037"),
            (20000000000, "353130"),
        ];

        for (time, expected) in cases {
            assert_eq!(
                generate_token(RFC_SECRET, time).unwrap(),
                expected,
                "t={time}"
            );
        }
    }

    #[test]
    fn normalizes_lowercase_and_spaced_secrets() {
        assert_eq!(
            generate_token("gezd gnbv gy3t qojq gezd gnbv gy3t qojq", 59).unwrap(),
            "287082"
        );
    }

    #[test]
    fn normalizes_secrets_by_stripping_separators_and_padding() {
        assert_eq!(normalize_secret(" gezd-gnbv\n").unwrap(), "GEZDGNBV");
        assert_eq!(normalize_secret("GEZDGNBV====").unwrap(), "GEZDGNBV");
    }

    #[test]
    fn rejects_invalid_secrets() {
        assert!(normalize_secret("").is_err());
        assert!(normalize_secret("   ").is_err());
        assert!(normalize_secret("GEZD1NBV").is_err());
        assert!(generate_token("AAAA1AAA", 59).is_err());
    }

    #[test]
    fn extracts_secret_from_otpauth_uri() {
        let uri =
            "otpauth://totp/ACME%20Co:john@example.com?secret=gezdgnbvgy3tqojq&issuer=ACME%20Co";
        assert_eq!(extract_secret(uri).unwrap(), "GEZDGNBVGY3TQOJQ");
    }

    #[test]
    fn rejects_otpauth_uri_without_secret() {
        assert!(extract_secret("otpauth://totp/ACME?issuer=ACME").is_err());
    }

    #[test]
    fn extracts_plain_secret() {
        assert_eq!(extract_secret(" gezd gnbv ").unwrap(), "GEZDGNBV");
    }

    #[test]
    fn remaining_seconds_counts_down_to_window_boundary() {
        assert_eq!(remaining_seconds(0), 30);
        assert_eq!(remaining_seconds(59), 1);
        assert_eq!(remaining_seconds(60), 30);
        assert_eq!(remaining_seconds(61), 29);
    }
}
