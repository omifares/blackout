use rand::{prelude::*, rngs::OsRng};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const EFF_WORDLIST_RAW: &str = include_str!("wordlist/eff_wordlist.txt");

fn get_wordlist() -> &'static Vec<&'static str> {
    static WORDLIST: OnceLock<Vec<&'static str>> = OnceLock::new();

    WORDLIST.get_or_init(|| {
        EFF_WORDLIST_RAW
            .lines()
            .filter_map(|line| line.split_whitespace().last())
            .collect()
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GeneratorMode {
    RandomChars,
    Passphrase,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeneratorConfig {
    pub mode: GeneratorMode,
    pub length: usize,
    pub word_count: usize,
    pub separator: char,
    pub capitalize: bool,
    pub uppercase: bool,
    pub lowercase: bool,
    pub numbers: bool,
    pub symbols: bool,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            mode: GeneratorMode::Passphrase,
            length: 16,
            word_count: 5,
            uppercase: true,
            lowercase: true,
            numbers: false,
            symbols: false,
            capitalize: true,
            separator: '_',
        }
    }
}

impl GeneratorMode {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => GeneratorMode::RandomChars,
            _ => GeneratorMode::Passphrase,
        }
    }
}

pub fn generate(config: GeneratorConfig) -> Result<String, &'static str> {
    match config.mode {
        GeneratorMode::RandomChars => generate_random_chars(&config),
        GeneratorMode::Passphrase => generate_passphrase(&config),
    }
}

fn generate_random_chars(config: &GeneratorConfig) -> Result<String, &'static str> {
    let mut charset = String::new();

    if config.lowercase {
        charset.push_str("abcdefghijklmnopqrstuvwxyz");
    }
    if config.uppercase {
        charset.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    }
    if config.numbers {
        charset.push_str("0123456789");
    }
    if config.symbols {
        charset.push_str("!@#$%^&*()_+-=[]{}|;:,.<>?");
    }

    if charset.is_empty() {
        return Err("No charset selected!");
    }

    let mut rng = OsRng;
    let charset_chars: Vec<char> = charset.chars().collect();

    let password: String = (0..config.length)
        .map(|_| {
            let idx = rng.gen_range(0..charset_chars.len());
            charset_chars[idx]
        })
        .collect();

    Ok(password)
}

fn generate_passphrase(config: &GeneratorConfig) -> Result<String, &'static str> {
    if config.word_count == 0 {
        return Err("A contagem de palavras deve ser maior que zero.");
    }

    let mut rng = OsRng;
    let wordlist = get_wordlist();

    let mut chosen_words = Vec::with_capacity(config.word_count);

    for _ in 0..config.word_count {
        let idx = rng.gen_range(0..wordlist.len());
        let mut word = wordlist[idx].to_string();

        if config.capitalize {
            if let Some(first_char) = word.chars().next() {
                let first_upper = first_char.to_uppercase().to_string();
                word = format!("{}{}", first_upper, &word[first_char.len_utf8()..]);
            }
        }
        chosen_words.push(word);
    }

    Ok(chosen_words.join(&config.separator.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passphrase_generation() {
        let res = generate_passphrase(&GeneratorConfig {
            word_count: 4,
            separator: '-',
            capitalize: true,
            ..Default::default()
        })
        .unwrap();
        let split: Vec<&str> = res.split('-').collect();
        assert_eq!(split.len(), 4);

        for word in split {
            assert!(word.chars().next().unwrap().is_uppercase());
        }
    }
}
