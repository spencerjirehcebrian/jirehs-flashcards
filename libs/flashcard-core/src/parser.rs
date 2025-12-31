//! Markdown parser for flashcard files.
//!
//! # Format
//! ```markdown
//! ID: 1
//! Q: What is Rust?
//! A: A systems programming language.
//!
//! ID: 2
//! Q: Explain borrowing
//! A: Borrowing allows references without ownership.
//! Multiple lines are supported.
//! ```

use crate::error::{ParseError, Result};
use crate::types::RawCard;
use std::collections::HashSet;

/// Parse markdown content into raw cards.
pub fn parse(content: &str) -> Result<Vec<RawCard>> {
    if content.trim().is_empty() {
        return Ok(vec![]);
    }

    let mut cards = Vec::new();
    let mut seen_ids = HashSet::new();
    let mut parser = Parser::new();

    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
        parser.process_line(line, line_num)?;
    }

    parser.finalize(&mut cards, &mut seen_ids)?;
    Ok(cards)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Field {
    Question,
    Answer,
}

struct CardBuilder {
    id: Option<i64>,
    question: Option<String>,
    answer: Option<String>,
    start_line: usize,
}

impl CardBuilder {
    fn new(start_line: usize) -> Self {
        Self {
            id: None,
            question: None,
            answer: None,
            start_line,
        }
    }

    fn build(self) -> Result<RawCard> {
        let question = self.question.ok_or(ParseError::MissingQuestion {
            line: self.start_line,
        })?;
        let answer = self.answer.ok_or(ParseError::MissingAnswer {
            line: self.start_line,
        })?;

        Ok(RawCard {
            id: self.id,
            question: question.trim().to_string(),
            answer: answer.trim().to_string(),
            line_number: self.start_line,
        })
    }
}

struct Parser {
    current: Option<CardBuilder>,
    current_field: Option<Field>,
    buffer: Vec<String>,
}

impl Parser {
    fn new() -> Self {
        Self {
            current: None,
            current_field: None,
            buffer: Vec::new(),
        }
    }

    fn process_line(&mut self, line: &str, line_num: usize) -> Result<()> {
        match Self::parse_line(line) {
            LineType::Id(id_str) => self.handle_id(id_str, line_num)?,
            LineType::Question(text) => self.handle_question(text, line_num),
            LineType::Answer(text) => self.handle_answer(text),
            LineType::Text(text) => self.buffer.push(text.to_string()),
            LineType::Empty => self.buffer.push(String::new()),
        }
        Ok(())
    }

    fn parse_line(line: &str) -> LineType<'_> {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("ID:") {
            LineType::Id(rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix("Q:") {
            LineType::Question(rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix("A:") {
            LineType::Answer(rest.trim())
        } else if trimmed.is_empty() {
            LineType::Empty
        } else {
            LineType::Text(line)
        }
    }

    fn handle_id(&mut self, id_str: &str, line_num: usize) -> Result<()> {
        self.flush_buffer();

        let id = id_str
            .parse::<i64>()
            .map_err(|_| ParseError::InvalidId {
                line: line_num,
                value: id_str.to_string(),
            })?;

        // Start new card with ID
        if self.current.is_none() {
            self.current = Some(CardBuilder::new(line_num));
        }
        if let Some(ref mut card) = self.current {
            card.id = Some(id);
        }
        self.current_field = None;
        Ok(())
    }

    fn handle_question(&mut self, text: &str, line_num: usize) {
        self.flush_buffer();

        // If no current card, start one (card without ID)
        if self.current.is_none() {
            self.current = Some(CardBuilder::new(line_num));
        }

        self.current_field = Some(Field::Question);
        self.buffer.push(text.to_string());
    }

    fn handle_answer(&mut self, text: &str) {
        self.flush_buffer();
        self.current_field = Some(Field::Answer);
        self.buffer.push(text.to_string());
    }

    fn flush_buffer(&mut self) {
        if self.buffer.is_empty() {
            return;
        }

        let content = self.buffer.join("\n");
        self.buffer.clear();

        if let Some(ref mut card) = self.current {
            match self.current_field {
                Some(Field::Question) => card.question = Some(content),
                Some(Field::Answer) => card.answer = Some(content),
                None => {}
            }
        }
    }

    fn finalize(mut self, cards: &mut Vec<RawCard>, seen_ids: &mut HashSet<i64>) -> Result<()> {
        self.flush_buffer();

        if let Some(card) = self.current {
            let raw_card = card.build()?;
            if let Some(id) = raw_card.id {
                if !seen_ids.insert(id) {
                    return Err(ParseError::DuplicateId {
                        id,
                        line: raw_card.line_number,
                    });
                }
            }
            cards.push(raw_card);
        }

        Ok(())
    }
}

enum LineType<'a> {
    Id(&'a str),
    Question(&'a str),
    Answer(&'a str),
    Text(&'a str),
    Empty,
}

/// Inject IDs into markdown content for cards that don't have them.
/// Returns the updated content with IDs inserted.
pub fn inject_ids(content: &str, id_assignments: &[(usize, i64)]) -> String {
    if id_assignments.is_empty() {
        return content.to_string();
    }

    let mut assignments: std::collections::HashMap<usize, i64> = id_assignments.iter().copied().collect();
    let mut result = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
        if let Some(id) = assignments.remove(&line_num) {
            result.push(format!("ID: {}", id));
        }
        result.push(line.to_string());
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_card() {
        let input = "ID: 1\nQ: What is Rust?\nA: A systems programming language.";
        let cards = parse(input).unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, Some(1));
        assert_eq!(cards[0].question, "What is Rust?");
        assert_eq!(cards[0].answer, "A systems programming language.");
    }

    #[test]
    fn parse_card_without_id() {
        let input = "Q: What is Rust?\nA: A language.";
        let cards = parse(input).unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, None);
    }

    #[test]
    fn parse_multiline_answer() {
        let input = "ID: 1\nQ: Explain\nA: Line 1\nLine 2\n\nLine 4";
        let cards = parse(input).unwrap();
        assert_eq!(cards[0].answer, "Line 1\nLine 2\n\nLine 4");
    }

    #[test]
    fn parse_multiple_cards() {
        let input = "ID: 1\nQ: Q1\nA: A1\n\nID: 2\nQ: Q2\nA: A2";
        let cards = parse(input).unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].id, Some(1));
        assert_eq!(cards[1].id, Some(2));
    }

    #[test]
    fn parse_mixed_id_and_no_id() {
        let input = "Q: No ID\nA: Answer\n\nID: 5\nQ: Has ID\nA: Answer2";
        let cards = parse(input).unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].id, None);
        assert_eq!(cards[1].id, Some(5));
    }

    #[test]
    fn reject_duplicate_ids() {
        let input = "ID: 1\nQ: Q1\nA: A1\n\nID: 1\nQ: Q2\nA: A2";
        let result = parse(input);
        assert!(matches!(result, Err(ParseError::DuplicateId { id: 1, .. })));
    }

    #[test]
    fn reject_missing_question() {
        let input = "ID: 1\nA: Answer only";
        let result = parse(input);
        assert!(matches!(result, Err(ParseError::MissingQuestion { .. })));
    }

    #[test]
    fn reject_missing_answer() {
        let input = "ID: 1\nQ: Question only";
        let result = parse(input);
        assert!(matches!(result, Err(ParseError::MissingAnswer { .. })));
    }

    #[test]
    fn parse_empty_content() {
        let cards = parse("").unwrap();
        assert!(cards.is_empty());
    }

    #[test]
    fn inject_ids_works() {
        let content = "Q: New card\nA: Answer";
        let result = inject_ids(content, &[(1, 42)]);
        assert!(result.starts_with("ID: 42\n"));
    }
}
