//! The one-line quick-eval input's text, its submitted-expression history and the Up/Down
//! navigation over that history.
//!
//! RUN-002 and LOG-003: the window owns one [`QuickEval`] next to the workbench. Enter hands
//! [`QuickEval::expression`] to the workbench; only an accepted submission reaches
//! [`QuickEval::accept`], which is what clears the field and records the expression. A refused
//! submission therefore leaves the text exactly where it was. History navigation is a view over
//! accepted submissions only, newest first.
//!
//! The model is deliberately independent of egui so its ordering rules can be pinned without a
//! window: the UI layer is responsible for reading the text, translating Up/Down, and calling
//! the workbench.

/// Maximum number of submitted expressions retained for Up/Down navigation.
pub const HISTORY_CAP: usize = 100;

/// The text and submitted history of the quick-eval input.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QuickEval {
    text: String,
    /// Accepted expressions, oldest first. Navigation walks this backwards (newest first).
    history: Vec<String>,
    /// Index into `history` while navigating; `None` means the field is a fresh draft.
    cursor: Option<usize>,
    /// The draft that was in the field when navigation started, restored by going past the
    /// newest entry.
    draft: String,
}

impl QuickEval {
    /// Creates an empty input with no history.
    pub fn new() -> QuickEval {
        QuickEval::default()
    }

    /// The current field text, including any leading/trailing whitespace.
    #[allow(dead_code)] // host-facing model accessor exercised by the unit tests
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The editable field buffer handed to the egui text widget.
    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    /// The expression Enter would submit: the trimmed text, or `None` when it is blank.
    ///
    /// Trimming happens here so the echo line and the submitted source are the same string; an
    /// all-whitespace field is not a submission.
    pub fn expression(&self) -> Option<&str> {
        let expression = self.text.trim();
        if expression.is_empty() {
            None
        } else {
            Some(expression)
        }
    }

    /// Records an accepted submission and clears the field.
    ///
    /// Only the UI calls this after the workbench accepted the request; a refused request must
    /// leave the field alone.
    pub fn accept(&mut self, expression: &str) {
        let expression = expression.trim();
        if expression.is_empty() {
            return;
        }
        if self.history.len() >= HISTORY_CAP {
            self.history.remove(0);
        }
        self.history.push(expression.to_string());
        self.cursor = None;
        self.draft.clear();
        self.text.clear();
    }

    /// Every accepted submission, oldest first.
    #[allow(dead_code)] // host-facing model accessor exercised by the unit tests
    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// True while Up/Down is walking the submitted history instead of a fresh draft.
    #[allow(dead_code)] // host-facing model accessor exercised by the unit tests
    pub fn is_navigating(&self) -> bool {
        self.cursor.is_some()
    }

    /// Replaces the field with the next older expression, newest first.
    ///
    /// The first Up from a fresh draft saves the draft and selects the newest submission. When
    /// the oldest submission is already selected, Up leaves both the selection and the text
    /// unchanged: navigation stops there instead of wrapping around.
    ///
    /// Returns true when the field text changed.
    pub fn navigate_up(&mut self) -> bool {
        if self.history.is_empty() {
            return false;
        }
        match self.cursor {
            None => {
                self.draft = self.text.clone();
                self.cursor = Some(self.history.len() - 1);
            }
            Some(0) => return false,
            Some(index) => self.cursor = Some(index - 1),
        }
        self.text = self.history[self.cursor.expect("a cursor was just selected")].clone();
        true
    }

    /// Replaces the field with the next newer expression, or restores the saved draft when the
    /// newest submission is passed.
    ///
    /// Returns true when the field text changed. With no active navigation this is a no-op.
    pub fn navigate_down(&mut self) -> bool {
        let Some(index) = self.cursor else {
            return false;
        };
        if index + 1 < self.history.len() {
            self.cursor = Some(index + 1);
            self.text = self.history[index + 1].clone();
        } else {
            self.cursor = None;
            self.text = std::mem::take(&mut self.draft);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LOG-003: Up returns previously submitted expressions newest first and stops at the
    /// oldest instead of wrapping. The field really contains the restored text after each step.
    #[test]
    fn history_navigation_returns_previous_expressions_newest_first_and_stops_at_the_oldest() {
        let mut input = QuickEval::new();
        input.accept("1 + 1");
        input.accept("2 + 2");
        assert_eq!(input.text(), "", "accepting clears the field");
        assert_eq!(input.history(), ["1 + 1", "2 + 2"]);

        assert!(input.navigate_up());
        assert_eq!(input.text(), "2 + 2", "the first Up returns the newest");
        assert!(input.navigate_up());
        assert_eq!(input.text(), "1 + 1", "the second Up returns the older");
        assert!(
            !input.navigate_up(),
            "Up at the oldest must report no change"
        );
        assert_eq!(
            input.text(),
            "1 + 1",
            "Up at the oldest must keep the oldest text"
        );
    }

    /// Enter accepts only a non-blank expression, and its trimmed form is both what is recorded
    /// and what the field clears. This is the model half of "the input clears only after the
    /// request is accepted".
    #[test]
    fn an_accepted_expression_is_recorded_trimmed_once_and_clears_the_field() {
        let mut input = QuickEval::new();
        input.text_mut().push_str("  6 * 7  ");
        let expression = input
            .expression()
            .expect("a non-blank expression")
            .to_string();
        assert_eq!(expression, "6 * 7");
        input.accept(&expression);

        assert_eq!(input.text(), "");
        assert_eq!(input.history(), ["6 * 7"]);
        assert_eq!(
            input
                .history()
                .iter()
                .filter(|item| *item == "6 * 7")
                .count(),
            1,
            "one accepted submission records exactly one history entry"
        );
    }

    /// Going down past the newest submission restores the draft that was in the field before
    /// navigation started, and a later Down is a no-op.
    #[test]
    fn history_navigation_down_restores_the_newest_then_the_draft() {
        let mut input = QuickEval::new();
        input.accept("1 + 1");
        input.accept("2 + 2");
        input.text_mut().push_str("3 + 3");

        assert!(input.navigate_up());
        assert_eq!(input.text(), "2 + 2");
        assert!(input.navigate_up());
        assert_eq!(input.text(), "1 + 1");
        assert!(input.navigate_down());
        assert_eq!(input.text(), "2 + 2");
        assert!(input.navigate_down());
        assert_eq!(input.text(), "3 + 3", "past the newest restores the draft");
        assert!(!input.is_navigating());
        assert!(!input.navigate_down(), "with no navigation Down is a no-op");
        assert_eq!(input.text(), "3 + 3");
    }

    #[test]
    fn blank_text_is_not_an_expression_and_is_never_recorded() {
        let mut input = QuickEval::new();
        input.text_mut().push_str(" \t ");
        assert_eq!(input.expression(), None);
        input.accept(" \t ");
        assert!(input.history().is_empty());
        assert_eq!(
            input.text(),
            " \t ",
            "a blank accept must not clear the draft"
        );
    }
}
