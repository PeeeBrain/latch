use rand::distributions::Distribution;
use rand::distributions::Uniform;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use zxcvbn::zxcvbn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordOptions {
    pub length: u32,
    pub uppercase: bool,
    pub lowercase: bool,
    pub numbers: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,
}

impl Default for PasswordOptions {
    fn default() -> Self {
        PasswordOptions {
            length: 16,
            uppercase: true,
            lowercase: true,
            numbers: true,
            symbols: true,
            exclude_ambiguous: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StrengthReport {
    pub score: u8,
    pub entropy: f64,
    pub label: String,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}

const AMBIGUOUS_CHARS: &[char] = &['0', 'O', '1', 'l', 'I'];

const LOWERCASE: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

const UPPERCASE: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

const NUMBERS: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

const SYMBOLS: &[char] = &[
    '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '-', '_', '=', '+', '[', ']', '{', '}', '|',
    '\\', ':', ';', '"', '\'', '<', '>', ',', '.', '?', '/', '~', '`',
];

pub fn generate_password(options: &PasswordOptions) -> Result<String, String> {
    if options.length < 8 {
        return Err("Password length must be at least 8 characters".to_string());
    }
    if options.length > 128 {
        return Err("Password length cannot exceed 128 characters".to_string());
    }

    let mut charset: Vec<char> = Vec::new();

    if options.lowercase {
        charset.extend(LOWERCASE);
    }
    if options.uppercase {
        charset.extend(UPPERCASE);
    }
    if options.numbers {
        charset.extend(NUMBERS);
    }
    if options.symbols {
        charset.extend(SYMBOLS);
    }

    if charset.is_empty() {
        return Err("At least one character type must be selected".to_string());
    }

    let final_charset: Vec<char> = if options.exclude_ambiguous {
        charset
            .into_iter()
            .filter(|c| !AMBIGUOUS_CHARS.contains(c))
            .collect()
    } else {
        charset
    };

    if final_charset.is_empty() {
        return Err("No characters available after excluding ambiguous ones".to_string());
    }

    let mut rng = thread_rng();
    let password: String = (0..options.length)
        .map(|_| {
            let dist = Uniform::new(0, final_charset.len());
            final_charset[dist.sample(&mut rng)]
        })
        .collect();

    Ok(password)
}

pub fn analyze_password_strength(password: &str) -> StrengthReport {
    let result = zxcvbn(password, &[]);

    let score_u8 = result.score() as u8;
    let guesses = result.guesses();

    let entropy = guesses.ilog2() as f64;

    let label = match score_u8 {
        0 => "Very Weak",
        1 => "Weak",
        2 => "Fair",
        3 => "Strong",
        4 => "Very Strong",
        _ => "Excellent",
    }
    .to_string();

    let mut warnings: Vec<String> = Vec::new();
    let mut suggestions: Vec<String> = Vec::new();

    if let Some(feedback) = result.feedback() {
        if let Some(warning) = feedback.warning() {
            warnings.push(warning.to_string());
        }
        for suggestion in feedback.suggestions() {
            suggestions.push(suggestion.to_string());
        }
    }

    StrengthReport {
        score: score_u8,
        entropy,
        label,
        warnings,
        suggestions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_password_default_options() {
        let options = PasswordOptions::default();
        let password = generate_password(&options).unwrap();

        assert_eq!(password.len(), options.length as usize);
        assert!(password.is_ascii());
    }

    #[test]
    fn test_generate_password_custom_length() {
        let options = PasswordOptions {
            length: 32,
            ..Default::default()
        };

        let password = generate_password(&options).unwrap();
        assert_eq!(password.len(), 32);
    }

    #[test]
    fn test_generate_password_lowercase_only() {
        let options = PasswordOptions {
            lowercase: true,
            uppercase: false,
            numbers: false,
            symbols: false,
            exclude_ambiguous: false,
            ..Default::default()
        };

        let password = generate_password(&options).unwrap();
        assert!(password.chars().all(|c| c.is_ascii_lowercase()));
    }

    #[test]
    fn test_generate_password_exclude_ambiguous() {
        let options = PasswordOptions {
            exclude_ambiguous: true,
            ..Default::default()
        };

        let password = generate_password(&options).unwrap();
        for c in password.chars() {
            assert!(!AMBIGUOUS_CHARS.contains(&c));
        }
    }

    #[test]
    fn test_generate_password_too_short() {
        let options = PasswordOptions {
            length: 4,
            ..Default::default()
        };

        let result = generate_password(&options);
        assert!(result.is_err());
    }

    #[test]
    fn test_analyze_weak_password() {
        let report = analyze_password_strength("password123");
        assert!(report.score <= 2);
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn test_analyze_strong_password() {
        let report = analyze_password_strength("Tr0ub4dor&3!p@ss");
        assert!(report.score >= 4);
        assert!(report.entropy >= 40.0);
    }

    #[test]
    fn test_analyze_common_password() {
        let report = analyze_password_strength("qwerty123");
        println!(
            "Score: {}, Entropy: {}, Warnings: {:?}",
            report.score, report.entropy, report.warnings
        );
        assert!(report.score <= 1);
        assert!(report.entropy < 70.0);
        assert!(!report.warnings.is_empty());
    }
}
