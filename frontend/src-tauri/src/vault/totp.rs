pub fn generate_token(secret: &str, timestamp: u64) -> Result<String, String> {
    let normalized = secret.trim().trim_end_matches('=').to_ascii_uppercase();
    let secret = data_encoding::BASE32_NOPAD
        .decode(normalized.as_bytes())
        .map_err(|_| "TOTP secret is not valid Base32".to_string())?;

    Ok(totp_lite::totp_custom::<totp_lite::Sha1>(
        30, 6, &secret, timestamp,
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn generates_six_digit_tokens_from_rfc_6238_vectors() {
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        let vectors = [
            (59, "287082"),
            (1_111_111_109, "081804"),
            (1_111_111_111, "050471"),
            (1_234_567_890, "005924"),
            (2_000_000_000, "279037"),
            (20_000_000_000, "353130"),
        ];

        for (timestamp, expected) in vectors {
            assert_eq!(super::generate_token(secret, timestamp).unwrap(), expected);
        }
    }
}
