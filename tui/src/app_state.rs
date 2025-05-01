use cli_clipboard::{ClipboardContext, ClipboardProvider};
use cred_man_lib::{Db, DbLoadResult, DbLocation};

pub(crate) struct AppState {
    db_location: DbLocation,
    db: Option<Db>,
    clipboard: Option<ClipboardContext>,
}

impl AppState {
    pub(crate) fn new(db_location: DbLocation) -> Self {
        Self {
            db_location,
            db: None,
            clipboard: ClipboardContext::new().ok(),
        }
    }

    pub(crate) fn view(&mut self) -> AppStateView {
        if self.db.is_some() {
            AppStateView::Opened(AppStateOpened {
                db: self.db.as_mut().expect("db.is_some()"),
                clipboard: &mut self.clipboard,
            })
        } else {
            AppStateView::NotOpened(AppStateNotOpened {
                db_location: &self.db_location,
                db: &mut self.db,
            })
        }
    }
}

pub(crate) enum AppStateView<'a> {
    NotOpened(AppStateNotOpened<'a>),
    Opened(AppStateOpened<'a>),
}

impl<'a> AppStateView<'a> {
    pub(crate) fn into_not_opened(self) -> Option<AppStateNotOpened<'a>> {
        match self {
            Self::NotOpened(app_state_not_opened) => Some(app_state_not_opened),
            Self::Opened(_) => None,
        }
    }
    pub(crate) fn into_opened(self) -> Option<AppStateOpened<'a>> {
        match self {
            Self::NotOpened(_) => None,
            Self::Opened(app_state_opened) => Some(app_state_opened),
        }
    }
}

pub(crate) struct AppStateNotOpened<'a> {
    db_location: &'a DbLocation,
    db: &'a mut Option<Db>,
}

impl AppStateNotOpened<'_> {
    pub(crate) fn open(&mut self, password: &str) -> std::io::Result<bool> {
        match Db::load(self.db_location, password)? {
            DbLoadResult::Loaded(db) => {
                *self.db = Some(db);
                Ok(true)
            }
            DbLoadResult::WrongPassword => Ok(false),
        }
    }
}

pub(crate) struct AppStateOpened<'a> {
    pub(crate) db: &'a mut Db,
    pub(crate) clipboard: &'a mut Option<ClipboardContext>,
}

impl AppStateOpened<'_> {
    pub(crate) fn db(&self) -> &Db {
        self.db
    }
}
