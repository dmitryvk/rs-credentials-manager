use std::collections::BTreeMap;

use cli_clipboard::ClipboardProvider;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListState, Paragraph},
    DefaultTerminal, Frame,
};

use crate::app_state::AppState;

pub(crate) fn ui_main(app_state: &mut AppState) {
    let terminal = ratatui::init();

    let mut app_view = AppView::Login(LoginView {
        password: "".into(),
        show_error: false,
    });

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

struct LoginView {
    password: String,
    show_error: bool,
}

struct MainView {
    search: String,
    search_results: Vec<String>,
    focus: MainViewFocus,
    list_state: ListState,
    sublist_state: ListState,
    scroll_page_size: usize,
    reveal_data: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum MainViewFocus {
    Search,
    List,
    Sublist,
}

impl MainView {
    fn new(app_state: &mut AppState) -> Self {
        let app_view = app_state
            .view()
            .into_opened()
            .expect("main view is active when db is open");
        Self {
            search: "".into(),
            search_results: app_view.db().data.keys().cloned().collect(),
            list_state: ListState::default().with_selected(Some(0)),
            sublist_state: ListState::default().with_selected(Some(0)),
            scroll_page_size: 1,
            focus: MainViewFocus::Search,
            reveal_data: false,
        }
    }
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
    fn draw(&mut self, app_state: &mut AppState, frame: &mut ratatui::Frame<'_>) {
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

impl LoginView {
    fn draw(&mut self, _app_state: &mut AppState, frame: &mut ratatui::Frame<'_>) {
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

    fn handle_event(&mut self, app_state: &mut AppState, event: Event) -> EventHandleResult {
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

impl MainView {
    fn draw(&mut self, app_state: &mut AppState, frame: &mut Frame<'_>) {
        let app_state = app_state
            .view()
            .into_opened()
            .expect("main view is active only if database is open");
        let [search_area, main_area, help_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());
        let [list_area, sublist_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(2)]).areas(main_area);

        let focused_block_style = Style::new().fg(Color::Green);
        let default_block_style = Style::new();
        let search_border_style = if self.focus == MainViewFocus::Search {
            focused_block_style
        } else {
            default_block_style
        };
        let list_border_style = if self.focus == MainViewFocus::List {
            focused_block_style
        } else {
            default_block_style
        };
        let sublist_border_style = if self.focus == MainViewFocus::Sublist {
            focused_block_style
        } else {
            default_block_style
        };

        let search = Paragraph::new(self.search.clone()).block(
            Block::new()
                .borders(Borders::ALL)
                .title("Search")
                .border_style(search_border_style),
        );
        frame.render_widget(search, search_area);

        let list_block = Block::new()
            .borders(Borders::ALL)
            .title("Credentials")
            .border_style(list_border_style);
        let list = List::new(self.search_results.clone())
            .highlight_style(Style::new().bg(Color::Green).fg(Color::Black))
            .highlight_symbol(">");
        self.scroll_page_size = list_block.inner(list_area).height.saturating_sub(1) as usize;
        frame.render_stateful_widget(list.block(list_block), list_area, &mut self.list_state);

        let empty_btreemap = BTreeMap::new();
        let sublist_data = self
            .list_state
            .selected()
            .and_then(|idx| self.search_results.get(idx))
            .and_then(|key| app_state.db().data.get(key))
            .map_or(&empty_btreemap, |rec| &rec.value);

        let sublist_block = Block::new()
            .borders(Borders::ALL)
            .title("Credentials data")
            .border_style(sublist_border_style);
        let sublist = List::new(sublist_data.iter().map(|(key, value)| {
            if self.reveal_data {
                format!("{key}: {value}")
            } else {
                format!("{key}: ***")
            }
        }))
        .highlight_style(Style::new().bg(Color::Green).fg(Color::Black))
        .highlight_symbol(">");
        frame.render_stateful_widget(
            sublist.block(sublist_block),
            sublist_area,
            &mut self.sublist_state,
        );

        let help = Paragraph::new(match self.focus {
            MainViewFocus::Search => "<Tab>/<Shift-Tab> switch",
            MainViewFocus::List => "<Tab>/<Shift-Tab> switch",
            MainViewFocus::Sublist => {
                "<Tab>/<Shift-Tab> switch <c> copy to clipboard <v> reveal/hide values"
            }
        });

        frame.render_widget(help, help_area);
    }

    fn handle_event(&mut self, app_state: &mut AppState, event: Event) -> EventHandleResult {
        let app_state = app_state
            .view()
            .into_opened()
            .expect("main view is active only for opened database");
        let Event::Key(key_event) = event else {
            return EventHandleResult::Continue;
        };
        if key_event.kind != KeyEventKind::Press {
            return EventHandleResult::Continue;
        }

        match key_event.code {
            KeyCode::Esc => {
                return EventHandleResult::Quit;
            }
            KeyCode::Tab => {
                self.focus = match self.focus {
                    MainViewFocus::Search => MainViewFocus::List,
                    MainViewFocus::List => MainViewFocus::Sublist,
                    MainViewFocus::Sublist => MainViewFocus::Search,
                };
            }
            KeyCode::BackTab => {
                self.focus = match self.focus {
                    MainViewFocus::Search => MainViewFocus::Sublist,
                    MainViewFocus::List => MainViewFocus::Search,
                    MainViewFocus::Sublist => MainViewFocus::List,
                };
            }
            KeyCode::Char(c) if self.focus == MainViewFocus::Search => {
                self.search.push(c);
                self.search_results = app_state
                    .db()
                    .data
                    .keys()
                    .filter(|s| s.contains(&self.search))
                    .cloned()
                    .collect();
                self.list_state.select_first();
                self.sublist_state.select_first();
            }
            KeyCode::Backspace if self.focus == MainViewFocus::Search => {
                self.search.pop();
                self.search_results = app_state
                    .db()
                    .data
                    .keys()
                    .filter(|s| s.contains(&self.search))
                    .cloned()
                    .collect();
                self.list_state.select_first();
                self.sublist_state.select_first();
            }
            KeyCode::Up
                if self.focus == MainViewFocus::Search || self.focus == MainViewFocus::List =>
            {
                self.focus = MainViewFocus::List;
                self.list_state.select_previous();
                self.sublist_state.select_first();
            }
            KeyCode::Down
                if self.focus == MainViewFocus::Search || self.focus == MainViewFocus::List =>
            {
                self.focus = MainViewFocus::List;
                self.list_state.select_next();
                self.sublist_state.select_first();
            }
            KeyCode::PageUp
                if self.focus == MainViewFocus::Search || self.focus == MainViewFocus::List =>
            {
                self.focus = MainViewFocus::List;
                self.list_state.scroll_up_by(self.scroll_page_size as u16);
                self.sublist_state.select_first();
            }
            KeyCode::PageDown
                if self.focus == MainViewFocus::Search || self.focus == MainViewFocus::List =>
            {
                self.focus = MainViewFocus::List;
                self.list_state.scroll_down_by(self.scroll_page_size as u16);
                self.sublist_state.select_first();
            }
            KeyCode::Home
                if self.focus == MainViewFocus::Search || self.focus == MainViewFocus::List =>
            {
                self.focus = MainViewFocus::List;
                self.list_state.select_first();
                self.sublist_state.select_first();
            }
            KeyCode::End
                if self.focus == MainViewFocus::Search || self.focus == MainViewFocus::List =>
            {
                self.focus = MainViewFocus::List;
                self.list_state.select_last();
                self.sublist_state.select_first();
            }
            KeyCode::Up if self.focus == MainViewFocus::Sublist => {
                self.sublist_state.select_previous();
            }
            KeyCode::Down if self.focus == MainViewFocus::Sublist => {
                self.sublist_state.select_next();
            }
            KeyCode::PageUp if self.focus == MainViewFocus::Sublist => {
                self.sublist_state
                    .scroll_up_by(self.scroll_page_size as u16);
            }
            KeyCode::PageDown if self.focus == MainViewFocus::Sublist => {
                self.sublist_state
                    .scroll_down_by(self.scroll_page_size as u16);
            }
            KeyCode::Home if self.focus == MainViewFocus::Sublist => {
                self.sublist_state.select_first();
            }
            KeyCode::End if self.focus == MainViewFocus::Sublist => {
                self.sublist_state.select_last()
            }
            KeyCode::Char('c') if self.focus == MainViewFocus::Sublist => {
                if let (Some(list_idx), Some(sublist_idx)) =
                    (self.list_state.selected(), self.sublist_state.selected())
                {
                    if let Some((_key, value)) = self
                        .search_results
                        .get(list_idx)
                        .and_then(|key| app_state.db.data.get(key))
                        .and_then(|rec| rec.value.iter().nth(sublist_idx))
                    {
                        if let Some(clipboard) = app_state.clipboard {
                            clipboard.set_contents(value.clone()).unwrap();
                        }
                    }
                }
            }
            KeyCode::Char('v') if self.focus == MainViewFocus::Sublist => {
                self.reveal_data = !self.reveal_data;
            }
            _ => {}
        }

        EventHandleResult::Continue
    }
}
