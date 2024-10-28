use login_view::LoginView;
use main_view::MainView;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyModifiers},
    DefaultTerminal, Frame,
};

use crate::app_state::AppState;

mod login_view;
mod main_view;

pub(crate) fn ui_main(app_state: &mut AppState) {
    let terminal = ratatui::init();

    let mut app_view = AppView::Login(LoginView::new());

    run(&mut app_view, app_state, terminal);

    ratatui::restore();
}

enum EventHandleResult {
    Continue,
    Quit,
    ChangeView(AppView),
}

enum AppView {
    Login(LoginView),
    Main(MainView),
}

fn run(app_view: &mut AppView, app_state: &mut AppState, mut terminal: DefaultTerminal) {
    loop {
        terminal
            .draw(|f| {
                app_view.draw(app_state, f);
            })
            .unwrap();

        let event = event::read().unwrap();
        match app_view.handle_event(app_state, event) {
            EventHandleResult::Continue => {}
            EventHandleResult::Quit => {
                break;
            }
            EventHandleResult::ChangeView(new_view) => {
                *app_view = new_view;
            }
        }
    }
}

impl AppView {
    fn draw(&mut self, app_state: &mut AppState, frame: &mut Frame<'_>) {
        match self {
            Self::Login(login_view) => login_view.draw(app_state, frame),
            Self::Main(main_view) => main_view.draw(app_state, frame),
        }
    }

    fn handle_event(&mut self, app_state: &mut AppState, event: Event) -> EventHandleResult {
        if let Event::Key(key_event) = &event {
            if key_event.code == KeyCode::Char('c')
                && key_event.modifiers.intersects(KeyModifiers::CONTROL)
            {
                return EventHandleResult::Quit;
            }
        }
        match self {
            Self::Login(login_view) => login_view.handle_event(app_state, event),
            Self::Main(main_view) => main_view.handle_event(app_state, event),
        }
    }
}
