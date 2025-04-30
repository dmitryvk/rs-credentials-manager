use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear},
    Frame,
};

pub(crate) struct KeyNameEditView {
    name: String,
    mode: KeyNameEditMode,
    error_message: String,
}

#[derive(Clone)]
pub(crate) enum KeyNameEditMode {
    NewKey,
    RenameKey { from_name: String },
}

pub(crate) enum EditResult {
    Cancel,
    Confirm { mode: KeyNameEditMode, name: String },
}

impl KeyNameEditView {
    pub(crate) fn new(mode: KeyNameEditMode) -> Self {
        Self {
            name: match &mode {
                KeyNameEditMode::NewKey => "".into(),
                KeyNameEditMode::RenameKey { from_name } => from_name.clone(),
            },
            mode,
            error_message: "".into(),
        }
    }

    pub(crate) fn draw(&mut self, frame: &mut Frame<'_>) {
        let [_, v_area, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(5),
            Constraint::Fill(2),
        ])
        .areas(frame.area());
        let [_, area, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(50),
            Constraint::Fill(1),
        ])
        .areas(v_area);
        let name_line = Line::default().spans([Span::styled(self.name.clone(), Style::default())]);
        let title = match &self.mode {
            KeyNameEditMode::NewKey => "name for new key".into(),
            KeyNameEditMode::RenameKey { from_name } => format!("rename {from_name} into"),
        };
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .title(title);
        let [name_area, message_area, help_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(block.inner(area));
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);
        frame.render_widget(name_line, name_area);

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
                        name: self.name.clone(),
                    });
                }
            }
            KeyCode::Esc => {
                return Some(EditResult::Cancel);
            }
            KeyCode::Char(c) => {
                self.name.push(c);
                self.error_message.clear();
            }
            KeyCode::Backspace => {
                self.name.pop();
                self.error_message.clear();
            }
            _ => {}
        }

        None
    }

    pub(crate) fn set_error_message(&mut self, message: String) {
        self.error_message = message;
    }
}
