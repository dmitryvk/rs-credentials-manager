use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

use crate::app_state::AppState;

use super::{AppView, EventHandleResult, MainView};

pub(crate) struct LoginView {
    password: String,
    show_error: bool,
}

impl LoginView {
    pub(crate) fn new() -> Self {
        Self {
            password: "".into(),
            show_error: false,
        }
    }

    pub(crate) fn draw(&mut self, _app_state: &mut AppState, frame: &mut Frame<'_>) {
        let [_, v_area, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(4),
            Constraint::Fill(2),
        ])
        .areas(frame.area());
        let [_, area, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(30),
            Constraint::Fill(1),
        ])
        .areas(v_area);
        let view_password: String = "*".repeat(self.password.len());
        let password_line = Line::default().spans([Span::styled(view_password, Style::default())]);
        let block = Block::new().borders(Borders::ALL).title("Enter password");
        let [password_area, message_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(block.inner(area));
        frame.render_widget(block, area);
        frame.render_widget(password_line, password_area);
        if self.show_error {
            let error_message = Line::default().spans([Span::styled(
                "Invalid password",
                Style::default().fg(Color::Red),
            )]);
            frame.render_widget(error_message, message_area);
        }
    }

    pub(crate) fn handle_event(
        &mut self,
        app_state: &mut AppState,
        event: Event,
    ) -> EventHandleResult {
        let mut app_view = app_state
            .view()
            .into_not_opened()
            .expect("when login view is active, state is not opened");

        let Event::Key(key_event) = event else {
            return EventHandleResult::Continue;
        };
        if key_event.kind != KeyEventKind::Press {
            return EventHandleResult::Continue;
        }

        match key_event.code {
            KeyCode::Enter => {
                if app_view.open(&self.password).unwrap() {
                    return EventHandleResult::ChangeView(AppView::Main(MainView::new(app_state)));
                } else {
                    self.show_error = true;
                }
            }
            KeyCode::Esc => {
                return EventHandleResult::Quit;
            }
            KeyCode::Char(c) => {
                self.password.push(c);
            }
            KeyCode::Backspace => {
                self.password.pop();
            }
            _ => {}
        }

        EventHandleResult::Continue
    }
}
