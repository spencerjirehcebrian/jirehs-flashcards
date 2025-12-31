//! Answer matching for typed mode study sessions.

use crate::types::MatchingMode;
use serde::{Deserialize, Serialize};

/// Result of comparing a typed answer to the correct answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    /// Whether the answer is considered correct.
    pub is_correct: bool,
    /// Similarity score between 0.0 and 1.0.
    pub similarity: f64,
    /// The matching mode used.
    pub matching_mode: MatchingMode,
    /// Normalized typed answer (for display).
    pub typed_normalized: String,
    /// Normalized correct answer (for display).
    pub correct_normalized: String,
}

/// Compare a typed answer to the correct answer.
pub fn compare_answers(
    typed: &str,
    correct: &str,
    mode: MatchingMode,
    fuzzy_threshold: f64,
) -> MatchResult {
    let typed_normalized = normalize_whitespace(typed);
    let correct_normalized = normalize_whitespace(correct);

    match mode {
        MatchingMode::Exact => {
            let is_correct = typed_normalized == correct_normalized;
            MatchResult {
                is_correct,
                similarity: if is_correct { 1.0 } else { 0.0 },
                matching_mode: mode,
                typed_normalized,
                correct_normalized,
            }
        }
        MatchingMode::CaseInsensitive => {
            let is_correct = typed_normalized.to_lowercase() == correct_normalized.to_lowercase();
            MatchResult {
                is_correct,
                similarity: if is_correct { 1.0 } else { 0.0 },
                matching_mode: mode,
                typed_normalized,
                correct_normalized,
            }
        }
        MatchingMode::Fuzzy => {
            let similarity = normalized_similarity(
                &typed_normalized.to_lowercase(),
                &correct_normalized.to_lowercase(),
            );
            let is_correct = similarity >= fuzzy_threshold;
            MatchResult {
                is_correct,
                similarity,
                matching_mode: mode,
                typed_normalized,
                correct_normalized,
            }
        }
    }
}

/// Normalize whitespace in a string (trim and collapse multiple spaces).
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Calculate Levenshtein distance between two strings.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // Use two rows instead of full matrix for memory efficiency
    let mut prev = (0..=n).collect::<Vec<_>>();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;

        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };

            curr[j] = (prev[j] + 1) // deletion
                .min(curr[j - 1] + 1) // insertion
                .min(prev[j - 1] + cost); // substitution
        }

        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Calculate normalized similarity (0.0 to 1.0) based on Levenshtein distance.
pub fn normalized_similarity(a: &str, b: &str) -> f64 {
    let max_len = a.len().max(b.len());
    if max_len == 0 {
        return 1.0; // Both empty strings are identical
    }

    let distance = levenshtein_distance(a, b);
    1.0 - (distance as f64 / max_len as f64)
}

/// Generate a diff between two strings for display.
/// Returns a list of (text, diff_type) tuples.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiffType {
    /// Text is the same in both strings.
    Same,
    /// Text was added (in correct but not typed).
    Added,
    /// Text was removed (in typed but not correct).
    Removed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSegment {
    pub text: String,
    pub diff_type: DiffType,
}

/// Simple word-level diff between typed and correct answers.
pub fn word_diff(typed: &str, correct: &str) -> Vec<DiffSegment> {
    let typed_words: Vec<&str> = typed.split_whitespace().collect();
    let correct_words: Vec<&str> = correct.split_whitespace().collect();

    let mut result = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < typed_words.len() || j < correct_words.len() {
        if i < typed_words.len() && j < correct_words.len() {
            if typed_words[i].to_lowercase() == correct_words[j].to_lowercase() {
                result.push(DiffSegment {
                    text: typed_words[i].to_string(),
                    diff_type: DiffType::Same,
                });
                i += 1;
                j += 1;
            } else {
                // Look ahead to see if we can find a match
                let mut found = false;

                // Check if typed word appears later in correct
                for k in j + 1..correct_words.len().min(j + 3) {
                    if typed_words[i].to_lowercase() == correct_words[k].to_lowercase() {
                        // Add missing words as added
                        for l in j..k {
                            result.push(DiffSegment {
                                text: correct_words[l].to_string(),
                                diff_type: DiffType::Added,
                            });
                        }
                        j = k;
                        found = true;
                        break;
                    }
                }

                if !found {
                    // Check if correct word appears later in typed
                    for k in i + 1..typed_words.len().min(i + 3) {
                        if correct_words[j].to_lowercase() == typed_words[k].to_lowercase() {
                            // Add extra words as removed
                            for l in i..k {
                                result.push(DiffSegment {
                                    text: typed_words[l].to_string(),
                                    diff_type: DiffType::Removed,
                                });
                            }
                            i = k;
                            found = true;
                            break;
                        }
                    }
                }

                if !found {
                    // No match found, mark typed as removed and correct as added
                    result.push(DiffSegment {
                        text: typed_words[i].to_string(),
                        diff_type: DiffType::Removed,
                    });
                    result.push(DiffSegment {
                        text: correct_words[j].to_string(),
                        diff_type: DiffType::Added,
                    });
                    i += 1;
                    j += 1;
                }
            }
        } else if i < typed_words.len() {
            // Extra words in typed
            result.push(DiffSegment {
                text: typed_words[i].to_string(),
                diff_type: DiffType::Removed,
            });
            i += 1;
        } else {
            // Missing words from correct
            result.push(DiffSegment {
                text: correct_words[j].to_string(),
                diff_type: DiffType::Added,
            });
            j += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("saturday", "sunday"), 3);
    }

    #[test]
    fn test_normalized_similarity() {
        assert_eq!(normalized_similarity("abc", "abc"), 1.0);
        assert_eq!(normalized_similarity("", ""), 1.0);
        assert!(normalized_similarity("kitten", "sitting") > 0.5);
        assert!(normalized_similarity("abc", "xyz") < 0.5);
    }

    #[test]
    fn test_compare_exact() {
        let result = compare_answers("hello", "hello", MatchingMode::Exact, 0.8);
        assert!(result.is_correct);
        assert_eq!(result.similarity, 1.0);

        let result = compare_answers("Hello", "hello", MatchingMode::Exact, 0.8);
        assert!(!result.is_correct);
    }

    #[test]
    fn test_compare_case_insensitive() {
        let result = compare_answers("Hello", "hello", MatchingMode::CaseInsensitive, 0.8);
        assert!(result.is_correct);

        let result = compare_answers("HELLO WORLD", "hello world", MatchingMode::CaseInsensitive, 0.8);
        assert!(result.is_correct);
    }

    #[test]
    fn test_compare_fuzzy() {
        let result = compare_answers("helo", "hello", MatchingMode::Fuzzy, 0.8);
        assert!(result.is_correct); // 80% similarity

        let result = compare_answers("xyz", "hello", MatchingMode::Fuzzy, 0.8);
        assert!(!result.is_correct);
    }

    #[test]
    fn test_whitespace_normalization() {
        let result = compare_answers("  hello   world  ", "hello world", MatchingMode::Exact, 0.8);
        assert!(result.is_correct);
    }
}
