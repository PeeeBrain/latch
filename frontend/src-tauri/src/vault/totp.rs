pub const PERIOD_SECS: u64 = 30;
pub const DIGITS: u32 = 6;

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
/// URIs requesting parameters other than the supported SHA-1/6-digit/30-second
/// defaults are rejected instead of silently generating incompatible codes.
pub fn extract_secret(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.to_ascii_lowercase().starts_with("otpauth://") {
        let url = url::Url::parse(trimmed).map_err(|e| format!("Invalid otpauth URI: {}", e))?;

        for (key, value) in url.query_pairs() {
            let supported = match key.as_ref() {
                "algorithm" => value.eq_ignore_ascii_case("sha1"),
                "digits" => value == "6",
                "period" => value == "30",
                _ => true,
            };
            if !supported {
                return Err(
                    "Only SHA-1, 6-digit, 30-second TOTP is supported for this secret".to_string(),
                );
            }
        }

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
    let normalized = normalize_secret(secret)?;
    let key = data_encoding::BASE32_NOPAD
        .decode(normalized.as_bytes())
        .map_err(|_| "TOTP secret is not valid Base32".to_string())?;

    Ok(totp_lite::totp_custom::<totp_lite::Sha1>(
        PERIOD_SECS,
        DIGITS,
        &key,
        unix_time,
    ))
}

/// Seconds remaining in the current 30-second window.
pub fn remaining_seconds(unix_time: u64) -> u64 {
    PERIOD_SECS - (unix_time % PERIOD_SECS)
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
    fn accepts_otpauth_uri_with_explicit_default_parameters() {
        let uri = "otpauth://totp/ACME?secret=GEZDGNBVGY3TQOJQ&algorithm=SHA1&digits=6&period=30";
        assert_eq!(extract_secret(uri).unwrap(), "GEZDGNBVGY3TQOJQ");
    }

    #[test]
    fn rejects_otpauth_uri_with_unsupported_parameters() {
        for uri in [
            "otpauth://totp/ACME?secret=GEZDGNBVGY3TQOJQ&algorithm=SHA256",
            "otpauth://totp/ACME?secret=GEZDGNBVGY3TQOJQ&digits=8",
            "otpauth://totp/ACME?secret=GEZDGNBVGY3TQOJQ&period=60",
        ] {
            assert!(extract_secret(uri).is_err(), "{uri} should be rejected");
        }
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
