//! Sync service for MD file processing.

use sha2::{Digest, Sha256};

use crate::error::ApiError;
use crate::models::NewIdAssignment;

/// Parsed card from MD content.
#[derive(Debug, Clone)]
pub struct ParsedCard {
    pub id: Option<i64>,
    pub question: String,
    pub answer: String,
    /// Line number where this card starts (1-indexed).
    pub line: usize,
}

/// Result of parsing an MD file.
#[derive(Debug)]
pub struct ParsedMdFile {
    pub cards: Vec<ParsedCard>,
}

/// Parse MD content to extract flashcards.
///
/// Format:
/// ```
/// ID: 123
/// Q: Question text
/// A: Answer text
/// ```
pub fn parse_md_content(content: &str) -> Result<ParsedMdFile, ApiError> {
    let mut cards = Vec::new();
    let mut current_card: Option<ParsedCardBuilder> = None;
    let mut current_field: Option<Field> = None;
    let mut field_buffer = String::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = line_idx + 1;
        let trimmed = line.trim();

        // Check for ID: line (starts a new card)
        if trimmed.starts_with("ID:") {
            // Flush previous card
            if let Some(builder) = current_card.take() {
                flush_field(&mut current_field, &mut field_buffer, &mut cards, builder)?;
            }

            let id_str = trimmed.strip_prefix("ID:").unwrap().trim();
            let id = if id_str.is_empty() {
                None
            } else {
                Some(id_str.parse::<i64>().map_err(|_| {
                    ApiError::Parse(format!("Invalid ID '{}' at line {}", id_str, line_num))
                })?)
            };

            current_card = Some(ParsedCardBuilder {
                id,
                question: None,
                answer: None,
                line: line_num,
            });
            current_field = None;
            field_buffer.clear();
        }
        // Check for Q: line
        else if trimmed.starts_with("Q:") {
            // If no current card, this starts a card without ID
            if current_card.is_none() {
                current_card = Some(ParsedCardBuilder {
                    id: None,
                    question: None,
                    answer: None,
                    line: line_num,
                });
            }

            // Flush previous field if any
            if let Some(builder) = current_card.as_mut() {
                flush_current_field(&current_field, &field_buffer, builder);
            }

            current_field = Some(Field::Question);
            field_buffer = trimmed.strip_prefix("Q:").unwrap().trim().to_string();
        }
        // Check for A: line
        else if trimmed.starts_with("A:") {
            if let Some(builder) = current_card.as_mut() {
                flush_current_field(&current_field, &field_buffer, builder);
            }

            current_field = Some(Field::Answer);
            field_buffer = trimmed.strip_prefix("A:").unwrap().trim().to_string();
        }
        // Regular line - append to current field
        else if current_field.is_some() {
            if !field_buffer.is_empty() {
                field_buffer.push('\n');
            }
            field_buffer.push_str(line);
        }
    }

    // Flush final card
    if let Some(builder) = current_card.take() {
        flush_field(&mut current_field, &mut field_buffer, &mut cards, builder)?;
    }

    Ok(ParsedMdFile { cards })
}

#[derive(Debug)]
enum Field {
    Question,
    Answer,
}

#[derive(Debug)]
struct ParsedCardBuilder {
    id: Option<i64>,
    question: Option<String>,
    answer: Option<String>,
    line: usize,
}

fn flush_current_field(field: &Option<Field>, buffer: &str, builder: &mut ParsedCardBuilder) {
    let content = buffer.trim().to_string();
    if content.is_empty() {
        return;
    }

    match field {
        Some(Field::Question) => builder.question = Some(content),
        Some(Field::Answer) => builder.answer = Some(content),
        None => {}
    }
}

fn flush_field(
    field: &mut Option<Field>,
    buffer: &mut String,
    cards: &mut Vec<ParsedCard>,
    mut builder: ParsedCardBuilder,
) -> Result<(), ApiError> {
    flush_current_field(field, buffer, &mut builder);

    // Only add card if it has both Q and A
    if let (Some(question), Some(answer)) = (builder.question, builder.answer) {
        cards.push(ParsedCard {
            id: builder.id,
            question,
            answer,
            line: builder.line,
        });
    }

    *field = None;
    buffer.clear();
    Ok(())
}

/// Regenerate MD content with new IDs inserted.
///
/// Takes the original content and a list of new ID assignments.
/// Returns the updated content with ID lines inserted.
pub fn regenerate_md_with_ids(content: &str, new_ids: &[NewIdAssignment]) -> String {
    if new_ids.is_empty() {
        return content.to_string();
    }

    // Create a map of line number -> ID to insert before
    let mut id_insertions: std::collections::HashMap<usize, i64> = std::collections::HashMap::new();
    for assignment in new_ids {
        id_insertions.insert(assignment.line, assignment.id);
    }

    let mut result = String::new();
    for (line_idx, line) in content.lines().enumerate() {
        let line_num = line_idx + 1;

        // Check if we need to insert an ID before this line
        if let Some(id) = id_insertions.get(&line_num) {
            result.push_str(&format!("ID: {}\n", id));
        }

        result.push_str(line);
        result.push('\n');
    }

    // Trim trailing newline if original didn't have one
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Calculate SHA256 hash of content.
pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Extract deck path from file path.
///
/// E.g., "rust/ownership.md" -> "rust"
/// E.g., "programming/rust/basics.md" -> "programming/rust"
pub fn extract_deck_path(file_path: &str) -> String {
    let path = std::path::Path::new(file_path);
    path.parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_card() {
        let content = r#"ID: 1
Q: What is Rust?
A: A systems programming language."#;

        let result = parse_md_content(content).unwrap();
        assert_eq!(result.cards.len(), 1);
        assert_eq!(result.cards[0].id, Some(1));
        assert_eq!(result.cards[0].question, "What is Rust?");
        assert_eq!(result.cards[0].answer, "A systems programming language.");
    }

    #[test]
    fn test_parse_card_without_id() {
        let content = r#"Q: What is a closure?
A: A function that captures its environment."#;

        let result = parse_md_content(content).unwrap();
        assert_eq!(result.cards.len(), 1);
        assert_eq!(result.cards[0].id, None);
        assert_eq!(result.cards[0].line, 1);
    }

    #[test]
    fn test_parse_multiline_answer() {
        let content = r#"ID: 2
Q: What are ownership rules?
A: 1. Each value has one owner
2. There can be one owner at a time
3. When owner goes out of scope, value is dropped"#;

        let result = parse_md_content(content).unwrap();
        assert_eq!(result.cards.len(), 1);
        assert!(result.cards[0].answer.contains("1. Each value"));
        assert!(result.cards[0].answer.contains("3. When owner"));
    }

    #[test]
    fn test_regenerate_with_ids() {
        let content = "Q: What is Rust?\nA: A language.\n";
        let new_ids = vec![NewIdAssignment {
            path: "test.md".to_string(),
            line: 1,
            id: 42,
        }];

        let result = regenerate_md_with_ids(content, &new_ids);
        assert!(result.starts_with("ID: 42\n"));
    }

    #[test]
    fn test_hash_content() {
        let hash = hash_content("test content");
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 hex is 64 chars
    }

    #[test]
    fn test_extract_deck_path() {
        assert_eq!(extract_deck_path("rust/ownership.md"), "rust");
        assert_eq!(extract_deck_path("prog/rust/basics.md"), "prog/rust");
        assert_eq!(extract_deck_path("single.md"), "");
    }

    // === Additional parse tests ===

    #[test]
    fn test_parse_multiple_cards() {
        let content = r#"ID: 1
Q: First question?
A: First answer.

ID: 2
Q: Second question?
A: Second answer."#;

        let result = parse_md_content(content).unwrap();
        assert_eq!(result.cards.len(), 2);
        assert_eq!(result.cards[0].id, Some(1));
        assert_eq!(result.cards[0].question, "First question?");
        assert_eq!(result.cards[1].id, Some(2));
        assert_eq!(result.cards[1].question, "Second question?");
    }

    #[test]
    fn test_parse_card_with_empty_id() {
        let content = r#"ID:
Q: Question?
A: Answer."#;

        let result = parse_md_content(content).unwrap();
        assert_eq!(result.cards.len(), 1);
        assert_eq!(result.cards[0].id, None);
    }

    #[test]
    fn test_parse_invalid_id_returns_error() {
        let content = r#"ID: not_a_number
Q: Question?
A: Answer."#;

        let result = parse_md_content(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_card_missing_answer() {
        let content = r#"ID: 1
Q: Question without answer"#;

        let result = parse_md_content(content).unwrap();
        assert_eq!(result.cards.len(), 0);
    }

    #[test]
    fn test_parse_card_missing_question() {
        let content = r#"ID: 1
A: Answer without question"#;

        let result = parse_md_content(content).unwrap();
        assert_eq!(result.cards.len(), 0);
    }

    #[test]
    fn test_parse_empty_content() {
        let result = parse_md_content("").unwrap();
        assert_eq!(result.cards.len(), 0);
    }

    #[test]
    fn test_parse_content_with_only_whitespace() {
        let result = parse_md_content("   \n\n   \t  ").unwrap();
        assert_eq!(result.cards.len(), 0);
    }

    #[test]
    fn test_parse_cards_without_ids_multiple() {
        // Multiple cards without IDs - each needs an empty ID line to delimit
        // This matches the expected file format where ID: line starts each card
        let content = "ID:\nQ: First?\nA: First.\n\nID:\nQ: Second?\nA: Second.";

        let result = parse_md_content(content).unwrap();
        assert_eq!(result.cards.len(), 2);
        assert_eq!(result.cards[0].id, None);
        assert_eq!(result.cards[0].line, 1);
        assert_eq!(result.cards[1].id, None);
        assert_eq!(result.cards[1].line, 5);
    }

    // === Additional regenerate tests ===

    #[test]
    fn test_regenerate_preserves_original_when_no_ids() {
        let content = "ID: 1\nQ: Question?\nA: Answer.";
        let result = regenerate_md_with_ids(content, &[]);
        assert_eq!(result, content);
    }

    #[test]
    fn test_regenerate_multiple_ids() {
        let content = "Q: First?\nA: First.\n\nQ: Second?\nA: Second.";
        let new_ids = vec![
            NewIdAssignment {
                path: "test.md".to_string(),
                line: 1,
                id: 100,
            },
            NewIdAssignment {
                path: "test.md".to_string(),
                line: 4,
                id: 101,
            },
        ];

        let result = regenerate_md_with_ids(content, &new_ids);
        assert!(result.contains("ID: 100\nQ: First?"));
        assert!(result.contains("ID: 101\nQ: Second?"));
    }

    // === Additional hash tests ===

    #[test]
    fn test_hash_deterministic() {
        let content = "test content";
        let hash1 = hash_content(content);
        let hash2 = hash_content(content);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_different_content() {
        let hash1 = hash_content("content 1");
        let hash2 = hash_content("content 2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_empty_string() {
        let hash = hash_content("");
        assert_eq!(hash.len(), 64);
    }

    // === Additional extract_deck_path tests ===

    #[test]
    fn test_extract_deck_path_nested() {
        assert_eq!(extract_deck_path("a/b/c/d.md"), "a/b/c");
    }

    #[test]
    fn test_extract_deck_path_with_spaces() {
        assert_eq!(extract_deck_path("my decks/rust/basics.md"), "my decks/rust");
    }
}
