use std::collections::BTreeMap;

use anyhow::Context;
use chrono::Local;
use cli_clipboard::ClipboardProvider;
use cred_man_lib::DbRecord;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListState, Paragraph},
    Frame,
};

use crate::app_state::{AppState, AppStateOpened};

use super::{
    key_name_edit_view::{self, KeyNameEditMode, KeyNameEditView},
    subkey_edit_view::{self, SubkeyEditView},
    EventHandleResult,
};

pub(crate) struct MainView {
    search: String,
    search_results: Vec<String>,
    focus: MainViewFocus,
    list_state: ListState,
    sublist_state: ListState,
    scroll_page_size: u16,
    reveal_data: bool,
    subview: Option<MainViewSubview>,
    is_dirty: bool,
}

enum MainViewSubview {
    EditingKey(Box<KeyNameEditView>),
    EditSubkey(Box<SubkeyEditView>),
}

#[derive(Clone, Copy, PartialEq)]
enum MainViewFocus {
    Search,
    List,
    Sublist,
}

impl MainView {
    pub(crate) fn new(app_state: &mut AppState) -> Self {
        let app_view = app_state
            .view()
            .into_opened()
            .expect("main view is active when db is open");
        Self {
            search: String::new(),
            search_results: app_view.db().data.keys().cloned().collect(),
            list_state: ListState::default().with_selected(Some(0)),
            sublist_state: ListState::default().with_selected(Some(0)),
            scroll_page_size: 1,
            focus: MainViewFocus::Search,
            reveal_data: false,
            subview: None,
            is_dirty: false,
        }
    }
}

impl MainView {
    pub(crate) fn draw(&mut self, app_state: &mut AppState, frame: &mut Frame<'_>) {
        #![allow(clippy::too_many_lines)]
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
        let search_border_style = if self.subview.is_none() && self.focus == MainViewFocus::Search {
            focused_block_style
        } else {
            default_block_style
        };
        let list_border_style = if self.subview.is_none() && self.focus == MainViewFocus::List {
            focused_block_style
        } else {
            default_block_style
        };
        let sublist_border_style = if self.subview.is_none() && self.focus == MainViewFocus::Sublist
        {
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
        self.scroll_page_size = list_block.inner(list_area).height.saturating_sub(1);
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

        let help: Vec<Span> = match self.focus {
            MainViewFocus::Search => vec!["<Tab>/<Shift-Tab> switch".into()],
            MainViewFocus::List => {
                vec![
                    "<Tab>/<Shift-Tab> switch <n> new <r> rename <d> delete".into(),
                    Span::styled(
                        " <s> save",
                        Style::default().fg(if self.is_dirty {
                            Color::Red
                        } else {
                            Color::Gray
                        }),
                    ),
                ]
            }
            MainViewFocus::Sublist => {
                vec![
                    "<Tab>/<Shift-Tab> switch <c> copy to clipboard <v> reveal/hide values <n> new <e> edit <d> del".into(),
                    Span::styled(
                        " <s> save",
                        Style::default().fg(if self.is_dirty {
                            Color::Red
                        } else {
                            Color::Gray
                        }),
                    ),
                ]
            }
        };

        frame.render_widget(Paragraph::new(Line::from(help)), help_area);

        if let Some(subview) = &mut self.subview {
            match subview {
                MainViewSubview::EditingKey(key_name_edit_view) => key_name_edit_view.draw(frame),
                MainViewSubview::EditSubkey(view) => view.draw(frame),
            }
        }
    }

    pub(crate) fn handle_event(
        &mut self,
        app_state: &mut AppState,
        event: &Event,
    ) -> anyhow::Result<EventHandleResult> {
        let mut app_state = app_state
            .view()
            .into_opened()
            .expect("main view is active only for opened database");

        if let Some(result) = self.handle_subview_event(&mut app_state, event) {
            Ok(result)
        } else {
            self.handle_own_event(&mut app_state, event)
        }
    }

    fn refresh(
        &mut self,
        app_state: &AppStateOpened,
        selected_key: Option<&str>,
        selected_attr: Option<&str>,
    ) {
        if let Some(selected_key) = selected_key {
            if !selected_key
                .to_lowercase()
                .contains(&self.search.to_lowercase())
            {
                self.search = String::new();
            }
        }
        self.search_results = app_state
            .db()
            .data
            .keys()
            .filter(|s| s.to_lowercase().contains(&self.search.to_lowercase()))
            .cloned()
            .collect();
        let selected_idx = selected_key
            .and_then(|selected_key| self.search_results.iter().position(|s| *s == selected_key));
        let selected_main_record =
            selected_idx.map(|idx| &app_state.db().data[&self.search_results[idx]]);
        self.list_state
            .select(selected_idx.or(self.list_state.selected()));
        if let (Some(main_record), Some(attr)) = (selected_main_record, selected_attr) {
            let selected_attr_idx = main_record
                .value
                .iter()
                .position(|(key, _value)| key == attr);
            self.sublist_state.select(selected_attr_idx);
        } else {
            self.sublist_state.select_first();
        }
    }
}

impl MainView {
    fn handle_own_event(
        &mut self,
        app_state: &mut AppStateOpened<'_>,
        event: &Event,
    ) -> anyhow::Result<EventHandleResult> {
        #![allow(clippy::too_many_lines)]
        let Event::Key(key_event) = event else {
            return Ok(EventHandleResult::Continue);
        };
        if key_event.kind != KeyEventKind::Press {
            return Ok(EventHandleResult::Continue);
        }

        match key_event.code {
            KeyCode::Esc => {
                return Ok(EventHandleResult::Quit);
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
                    .filter(|s| s.to_lowercase().contains(&self.search.to_lowercase()))
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
                    .filter(|s| s.to_lowercase().contains(&self.search.to_lowercase()))
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
                self.list_state.scroll_up_by(self.scroll_page_size);
                self.sublist_state.select_first();
            }
            KeyCode::PageDown
                if self.focus == MainViewFocus::Search || self.focus == MainViewFocus::List =>
            {
                self.focus = MainViewFocus::List;
                self.list_state.scroll_down_by(self.scroll_page_size);
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
            KeyCode::Char('s') if self.focus == MainViewFocus::List => {
                app_state.db.save().context("save db")?;
                self.is_dirty = false;
            }
            KeyCode::Char('n') if self.focus == MainViewFocus::List => {
                self.subview = Some(MainViewSubview::EditingKey(Box::new(KeyNameEditView::new(
                    KeyNameEditMode::NewKey,
                ))));
            }
            KeyCode::Char('d') if self.focus == MainViewFocus::List => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(key) = self.search_results.get(idx) {
                        app_state.db.data.remove(key);
                        self.refresh(app_state, None, None);
                        self.is_dirty = true;
                    }
                }
            }
            KeyCode::Char('r') if self.focus == MainViewFocus::List => {
                if let Some(idx) = self.list_state.selected() {
                    let key = self.search_results[idx].clone();
                    self.subview = Some(MainViewSubview::EditingKey(Box::new(
                        KeyNameEditView::new(KeyNameEditMode::RenameKey { from_name: key }),
                    )));
                }
            }
            KeyCode::Up if self.focus == MainViewFocus::Sublist => {
                self.sublist_state.select_previous();
            }
            KeyCode::Down if self.focus == MainViewFocus::Sublist => {
                self.sublist_state.select_next();
            }
            KeyCode::PageUp if self.focus == MainViewFocus::Sublist => {
                self.sublist_state.scroll_up_by(self.scroll_page_size);
            }
            KeyCode::PageDown if self.focus == MainViewFocus::Sublist => {
                self.sublist_state.scroll_down_by(self.scroll_page_size);
            }
            KeyCode::Home if self.focus == MainViewFocus::Sublist => {
                self.sublist_state.select_first();
            }
            KeyCode::End if self.focus == MainViewFocus::Sublist => {
                self.sublist_state.select_last();
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
                            clipboard
                                .set_contents(value.clone())
                                .map_err(|error| anyhow::anyhow!("{error:?}"))
                                .context("clipboard copy")?;
                        }
                    }
                }
            }
            KeyCode::Char('v') if self.focus == MainViewFocus::Sublist => {
                self.reveal_data = !self.reveal_data;
            }
            KeyCode::Char('n') if self.focus == MainViewFocus::Sublist => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(key) = self.search_results.get(idx) {
                        self.subview = Some(MainViewSubview::EditSubkey(Box::new(
                            SubkeyEditView::new(subkey_edit_view::EditingMode::NewSubkey {
                                key_name: key.clone(),
                            }),
                        )));
                    }
                }
            }
            KeyCode::Char('d') if self.focus == MainViewFocus::Sublist => {
                if let Some(selected_key) = self
                    .list_state
                    .selected()
                    .and_then(|idx| self.search_results.get(idx))
                    .and_then(|key| app_state.db.data.get_mut(key))
                {
                    if let Some(sublist_idx) = self.sublist_state.selected() {
                        if let Some(subkey) = selected_key.value.keys().nth(sublist_idx).cloned() {
                            selected_key.value.remove(&subkey);
                        }
                    }
                }
            }
            KeyCode::Char('e') if self.focus == MainViewFocus::Sublist => {
                if let Some(selected_key) = self
                    .list_state
                    .selected()
                    .and_then(|idx| self.search_results.get(idx))
                    .and_then(|key| app_state.db.data.get_mut(key))
                {
                    if let Some(sublist_idx) = self.sublist_state.selected() {
                        if let Some((subkey, value)) = selected_key.value.iter().nth(sublist_idx) {
                            self.subview = Some(MainViewSubview::EditSubkey(Box::new(
                                SubkeyEditView::new(subkey_edit_view::EditingMode::EditSubkey {
                                    key_name: selected_key.key.clone(),
                                    name: subkey.clone(),
                                    value: value.clone(),
                                }),
                            )));
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(EventHandleResult::Continue)
    }

    fn handle_subview_event(
        &mut self,
        app_state: &mut AppStateOpened<'_>,
        event: &Event,
    ) -> Option<EventHandleResult> {
        let Some(subview) = &mut self.subview else {
            return None;
        };
        match subview {
            MainViewSubview::EditingKey(key_name_edit_view) => {
                match key_name_edit_view.handle_event(event) {
                    None => {}
                    Some(key_name_edit_view::EditResult::Cancel) => {
                        self.subview = None;
                    }
                    Some(key_name_edit_view::EditResult::Confirm { mode, name }) => match mode {
                        key_name_edit_view::KeyNameEditMode::NewKey => {
                            if app_state.db.data.contains_key(&name) {
                                key_name_edit_view
                                    .set_error_message("this key already exists".into());
                            } else {
                                app_state.db.data.insert(
                                    name.clone(),
                                    DbRecord {
                                        key: name.clone(),
                                        timestamp: Local::now().naive_local(),
                                        value: BTreeMap::new(),
                                    },
                                );
                                self.subview = None;
                                self.refresh(app_state, Some(&name), None);
                                self.is_dirty = true;
                            }
                        }
                        key_name_edit_view::KeyNameEditMode::RenameKey { from_name } => {
                            if app_state.db.data.contains_key(&name) {
                                key_name_edit_view
                                    .set_error_message("this key already exists".into());
                            } else {
                                let mut db_record = app_state
                                    .db
                                    .data
                                    .remove(&from_name)
                                    .expect("the key existed before renaming");
                                db_record.key.clone_from(&name);
                                db_record.timestamp = Local::now().naive_local();
                                app_state.db.data.insert(name.clone(), db_record);
                                self.subview = None;
                                self.refresh(app_state, Some(&name), None);
                                self.is_dirty = true;
                            }
                        }
                    },
                }
            }
            MainViewSubview::EditSubkey(view) => match view.handle_event(event) {
                None => {}
                Some(subkey_edit_view::EditResult::Cancel) => {
                    self.subview = None;
                }
                Some(subkey_edit_view::EditResult::Confirm { mode, name, value }) => match mode {
                    subkey_edit_view::EditingMode::NewSubkey { key_name } => {
                        let db_record = app_state
                            .db
                            .data
                            .get_mut(&key_name)
                            .expect("the key existed before editing");
                        if db_record.value.contains_key(&name) {
                            view.set_error_message("this attribute already exists".to_string());
                        } else {
                            db_record.value.insert(name.clone(), value);
                            self.subview = None;
                            self.refresh(app_state, Some(&key_name), Some(&name));
                            self.is_dirty = true;
                        }
                    }
                    subkey_edit_view::EditingMode::EditSubkey {
                        key_name,
                        name: old_name,
                        value: _old_value,
                    } => {
                        let db_record = app_state
                            .db
                            .data
                            .get_mut(&key_name)
                            .expect("the key existed before editing");
                        if name != old_name && db_record.value.contains_key(&name) {
                            view.set_error_message("this attribute already exists".to_string());
                        } else {
                            db_record.value.remove(&old_name);
                            db_record.value.insert(name.clone(), value);
                            self.subview = None;
                            self.refresh(app_state, Some(&key_name), Some(&name));
                            self.is_dirty = true;
                        }
                    }
                },
            },
        }

        Some(EventHandleResult::Continue)
    }
}
