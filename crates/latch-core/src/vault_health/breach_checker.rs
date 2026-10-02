use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BreachResult {
    pub hash_suffix: String,
    pub count: u32,
}

pub trait BreachChecker: Send + Sync {
    fn check(
        &self,
        password: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<BreachResult>, String>> + Send + '_>>;
}

pub struct PwnedPasswordsApi;

impl BreachChecker for PwnedPasswordsApi {
    fn check(
        &self,
        password: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<BreachResult>, String>> + Send + '_>> {
        use sha1::{Digest, Sha1};
        let hash_upper = hex::encode_upper(Sha1::digest(password.as_bytes()));
        Box::pin(async move {
            let prefix = &hash_upper[..5];
            let suffix = &hash_upper[5..];
            let client = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|_| "Breach service unavailable")?;
            let mut response = client
                .get(format!("https://api.pwnedpasswords.com/range/{prefix}"))
                .header("User-Agent", "Latch-Password-Manager")
                .header("Add-Padding", "true")
                .send()
                .await
                .map_err(|_| "Breach request failed")?
                .error_for_status()
                .map_err(|_| "Breach service returned an error")?;
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| "Cannot read breach response")?
            {
                if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
                    return Err("Invalid breach response".into());
                }
                bytes.extend_from_slice(&chunk);
            }
            let body = String::from_utf8(bytes).map_err(|_| "Invalid breach response")?;
            parse_response(&body, suffix)
        })
    }
}

#[allow(dead_code)]
pub struct StubBreachChecker {
    pub results: Vec<(String, u32)>,
}

impl BreachChecker for StubBreachChecker {
    fn check(
        &self,
        password: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<BreachResult>, String>> + Send + '_>> {
        let password = password.to_string();
        let results = self.results.clone();
        Box::pin(async move {
            for (pwd, count) in &results {
                if pwd == &password {
                    return Ok(Some(BreachResult {
                        hash_suffix: String::new(),
                        count: *count,
                    }));
                }
            }
            Ok(None)
        })
    }
}

fn parse_response(body: &str, suffix: &str) -> Result<Option<BreachResult>, String> {
    if body.is_empty() || body.len() > 2 * 1024 * 1024 {
        return Err("Invalid breach response".into());
    }
    let mut matched = None;
    for line in body.lines() {
        let (hash, count) = line.split_once(':').ok_or("Invalid breach response")?;
        if hash.len() != 35 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("Invalid breach response".into());
        }
        let count: u32 = count
            .trim()
            .parse()
            .map_err(|_| "Invalid breach response")?;
        if hash.eq_ignore_ascii_case(suffix) && count > 0 {
            matched = Some(BreachResult {
                hash_suffix: suffix.into(),
                count,
            });
        }
    }
    Ok(matched)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_responses_are_not_clean_passwords() {
        let suffix = "A".repeat(35);
        assert!(
            parse_response(&format!("{suffix}:4\r\n{}:0", "B".repeat(35)), &suffix)
                .unwrap()
                .is_some()
        );
        assert!(parse_response(&format!("{}:0", "B".repeat(35)), &suffix)
            .unwrap()
            .is_none());
        for invalid in ["", "<html>service unavailable</html>", "AAA:nope"] {
            assert!(parse_response(invalid, &suffix).is_err());
        }
    }
}
