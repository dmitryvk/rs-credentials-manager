use anyhow::Context;
use login_view::LoginView;
use main_view::MainView;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyModifiers},
    DefaultTerminal, Frame,
};

use crate::app_state::AppState;

mod key_name_edit_view;
mod login_view;
mod main_view;
mod subkey_edit_view;

pub(crate) fn ui_main(app_state: &mut AppState) -> anyhow::Result<()> {
    let terminal = ratatui::init();

    let mut app_view = AppView::Login(LoginView::new());

    run(&mut app_view, app_state, terminal)?;

    ratatui::restore();

    Ok(())
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

fn run(
    app_view: &mut AppView,
    app_state: &mut AppState,
    mut terminal: DefaultTerminal,
) -> anyhow::Result<()> {
    loop {
        terminal
            .draw(|f| {
                app_view.draw(app_state, f);
            })
            .context("draw on terminal")?;

        let event = event::read().context("read ui event")?;
        match app_view.handle_event(app_state, &event)? {
            EventHandleResult::Continue => {}
            EventHandleResult::Quit => {
                break;
            }
            EventHandleResult::ChangeView(new_view) => {
                *app_view = new_view;
            }
        }
    }

    Ok(())
}

impl AppView {
    fn draw(&mut self, app_state: &mut AppState, frame: &mut Frame<'_>) {
        match self {
            Self::Login(login_view) => login_view.draw(app_state, frame),
            Self::Main(main_view) => main_view.draw(app_state, frame),
        }
    }

    fn handle_event(
        &mut self,
        app_state: &mut AppState,
        event: &Event,
    ) -> anyhow::Result<EventHandleResult> {
        if let Event::Key(key_event) = &event {
            if key_event.code == KeyCode::Char('c')
                && key_event.modifiers.intersects(KeyModifiers::CONTROL)
            {
                return Ok(EventHandleResult::Quit);
            }
        }
        match self {
            Self::Login(login_view) => login_view.handle_event(app_state, event),
            Self::Main(main_view) => main_view.handle_event(app_state, event),
        }
    }
}
