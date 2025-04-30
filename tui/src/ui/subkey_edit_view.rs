use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear},
    Frame,
};

pub(crate) struct SubkeyEditView {
    mode: EditingMode,
    focus: Focus,
    name: String,
    value: String,
    error_message: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Name,
    Value,
}

#[derive(Clone)]
pub(crate) enum EditingMode {
    NewSubkey {
        key_name: String,
    },
    EditSubkey {
        key_name: String,
        name: String,
        value: String,
    },
}

pub(crate) enum EditResult {
    Cancel,
    Confirm {
        mode: EditingMode,
        name: String,
        value: String,
    },
}

impl SubkeyEditView {
    pub(crate) fn new(mode: EditingMode) -> Self {
        Self {
            mode: mode.clone(),
            focus: Focus::Name,
            name: match &mode {
                EditingMode::NewSubkey { .. } => "".to_string(),
                EditingMode::EditSubkey { name, .. } => name.clone(),
            },
            value: match &mode {
                EditingMode::NewSubkey { .. } => "".to_string(),
                EditingMode::EditSubkey { value, .. } => value.clone(),
            },
            error_message: "".to_string(),
        }
    }

    pub(crate) fn set_error_message(&mut self, message: String) {
        self.error_message = message;
    }

    pub(crate) fn draw(&self, frame: &mut Frame<'_>) {
        let [_, v_area, _] = Layout::vertical([
            Constraint::Fill(1),
            // 2 - box (incl. title)
            // 1 - name
            // 1 - value
            // 1 - message
            // 1 - help
            Constraint::Length(6),
            Constraint::Fill(2),
        ])
        .areas(frame.area());
        let [_, area, _] = Layout::horizontal([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(2),
        ])
        .areas(v_area);
        let title = match &self.mode {
            EditingMode::NewSubkey { .. } => "new attribute".into(),
            EditingMode::EditSubkey { name, .. } => format!("edit attribute {name}"),
        };
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .title(title);
        frame.render_widget(Clear, area);
        frame.render_widget(&block, area);
        let [name_area, value_area, message_area, help_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(block.inner(area));

        let name_line = Line::default().spans([
            Span::styled(
                "name: ",
                if self.focus == Focus::Name {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                },
            ),
            Span::styled(
                self.name.clone(),
                if self.focus == Focus::Name {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                },
            ),
        ]);
        frame.render_widget(name_line, name_area);

        let value_line = Line::default().spans([
            Span::styled(
                "value: ",
                if self.focus == Focus::Value {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                },
            ),
            Span::styled(
                self.value.clone(),
                if self.focus == Focus::Value {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                },
            ),
        ]);
        frame.render_widget(value_line, value_area);

        let error_message = Line::default().spans([Span::styled(
            self.error_message.clone(),
            Style::default().fg(Color::Red),
        )]);
        frame.render_widget(error_message, message_area);

        let help_message = Line::default().spans([Span::styled(
            "<Enter> - confirm <Esc> - cancel",
            Style::default(),
        )]);
        frame.render_widget(help_message, help_area);
    }

    pub(crate) fn handle_event(&mut self, event: &Event) -> Option<EditResult> {
        let Event::Key(key_event) = event else {
            return None;
        };
        if key_event.kind != KeyEventKind::Press {
            return None;
        }

        match key_event.code {
            KeyCode::Enter => {
                if self.name.is_empty() {
                    self.error_message = "Name must not be empty".to_string();
                } else {
                    return Some(EditResult::Confirm {
                        mode: self.mode.clone(),
                        name: self.name.trim().to_string(),
                        value: self.value.trim().to_string(),
                    });
                }
            }
            KeyCode::Esc => {
                return Some(EditResult::Cancel);
            }
            KeyCode::Up => {
                self.focus = Focus::Name;
            }
            KeyCode::Down => {
                self.focus = Focus::Value;
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = match self.focus {
                    Focus::Name => Focus::Value,
                    Focus::Value => Focus::Name,
                }
            }
            KeyCode::Char(c) if self.focus == Focus::Name => {
                self.name.push(c);
                self.error_message.clear();
            }
            KeyCode::Backspace if self.focus == Focus::Name => {
                self.name.pop();
                self.error_message.clear();
            }
            KeyCode::Char(c) if self.focus == Focus::Value => {
                self.value.push(c);
                self.error_message.clear();
            }
            KeyCode::Backspace if self.focus == Focus::Value => {
                self.value.pop();
                self.error_message.clear();
            }
            _ => {}
        }

        None
    }
}
