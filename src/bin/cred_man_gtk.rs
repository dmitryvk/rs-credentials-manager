use cred_man_lib::{Db, DbLoadResult, DbLocation};
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

const BUILDER_UI: &str = include_str!("cred_man_gtk.ui");

struct Ui {
    window: gtk::Window,
    tree_credentials: gtk::TreeView,
    store_credentials: gtk::TreeStore,
    btn_unlock: gtk::ToolButton,
    dlg_password: gtk::Dialog,
    entry_password: gtk::Entry,
    entry_search_credentials: gtk::Entry,

    dialog_credinfo: gtk::Dialog,
    label_credinfo_key: gtk::Label,
    label_credinfo_attr: gtk::Label,
    entry_credinfo_value: gtk::Entry,

    db: Option<Db>,
    credinfo_value: Option<(String, String, String)>,
}

impl Ui {
    pub fn new(db_location: DbLocation) -> Rc<RefCell<Self>> {
        let b = gtk::Builder::new();

        b.add_from_string(BUILDER_UI)
            .expect("Unable to load GtkBuilder definition");

        let w: gtk::Window = b
            .object("wndMain")
            .expect("Unable to find wndMain in GtkBuilder definition");

        let tree_credentials: gtk::TreeView = b
            .object("treeCredentials")
            .expect("Unable to find treeCredentials");

        let store_credentials: gtk::TreeStore = b
            .object("storeCredentials")
            .expect("Unable to find storeCredentials");

        let btn_unlock: gtk::ToolButton = b.object("btnUnlock").expect("Unable to find btnUnlock");

        let dlg_password: gtk::Dialog =
            b.object("dlgPassword").expect("Unable to find dlgPassword");

        let entry_password: gtk::Entry = b
            .object("entryPassword")
            .expect("Unable to find entryPassword");

        let entry_search_credentials: gtk::Entry = b
            .object("entrySearchCredentials")
            .expect("Unable to find entrySearchCredentials");

        let dialog_credinfo: gtk::Dialog = b
            .object("dialog_credinfo")
            .expect("Unable to find dialog_credinfo");
        let label_credinfo_key: gtk::Label = b
            .object("label_credinfo_key")
            .expect("Unable to find label_credinfo_key");
        let label_credinfo_attr: gtk::Label = b
            .object("label_credinfo_attr")
            .expect("Unable to find label_credinfo_attr");
        let entry_credinfo_value: gtk::Entry = b
            .object("entry_credinfo_value")
            .expect("Unable to find entry_credinfo_value");

        let result = Rc::new(RefCell::new(Ui {
            window: w.clone(),
            tree_credentials: tree_credentials.clone(),
            store_credentials,
            btn_unlock: btn_unlock.clone(),
            dlg_password: dlg_password.clone(),
            entry_password,
            entry_search_credentials: entry_search_credentials.clone(),
            dialog_credinfo,
            label_credinfo_key,
            label_credinfo_attr,
            entry_credinfo_value: entry_credinfo_value.clone(),
            db: None,
            credinfo_value: None,
        }));

        let result2 = result.clone();
        btn_unlock.connect_clicked(move |_| {
            result2.borrow().dlg_password.show_all();
        });

        let result2 = result.clone();
        dlg_password.connect_delete_event(move |_, _| {
            result2.borrow().dlg_password.hide();
            Inhibit(true)
        });

        let result2 = result.clone();
        dlg_password.connect_response(move |_, response| {
            if response == gtk::ResponseType::Other(5) {
                let password = result2.borrow().entry_password.text().as_str().to_owned();

                match Db::load(&db_location, &password) {
                    Ok(DbLoadResult::Loaded(db)) => {
                        result2.borrow_mut().db = Some(db);

                        result2
                            .borrow()
                            .entry_search_credentials
                            .set_sensitive(true);
                        result2.borrow().tree_credentials.set_sensitive(true);
                        result2.borrow().btn_unlock.set_sensitive(false);
                        result2.borrow().dlg_password.hide();

                        Ui::refresh_tree(&result2);
                    }
                    Ok(DbLoadResult::WrongPassword) => {
                        let dlg = gtk::MessageDialog::new(
                            Some(&result2.borrow().dlg_password),
                            gtk::DialogFlags::MODAL,
                            gtk::MessageType::Error,
                            gtk::ButtonsType::Close,
                            "Wrong password",
                        );
                        dlg.run();
                        dlg.close();
                    }
                    Err(e) => {
                        let dlg = gtk::MessageDialog::new(
                            Some(&result2.borrow().dlg_password),
                            gtk::DialogFlags::MODAL,
                            gtk::MessageType::Error,
                            gtk::ButtonsType::Close,
                            &format!("error: {:}", e),
                        );
                        dlg.run();
                        dlg.close();
                    }
                }
            }
        });

        let result2 = result.clone();
        tree_credentials.connect_row_activated(move |_, path, _| {
            let store = result2.borrow().store_credentials.clone();
            let iter = store.iter(path).unwrap();
            let parent_iter = match store.iter_parent(&iter) {
                None => return,
                Some(it) => it,
            };

            let key = store.value(&parent_iter, 0).get::<String>().unwrap();
            let attr_name = store.value(&iter, 0).get::<String>().unwrap();
            Ui::show_attr(&result2, &key, &attr_name);
        });

        let result2 = result.clone();
        entry_search_credentials.connect_changed(move |_| {
            Ui::refresh_tree(&result2);
        });

        let result2 = result.clone();
        entry_credinfo_value.connect_icon_release(move |_, position, _| match position {
            gtk::EntryIconPosition::Primary => Ui::credinfo_reveal(&result2),
            gtk::EntryIconPosition::Secondary => Ui::credinfo_copy(&result2),
            _ => {}
        });

        w.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });

        entry_search_credentials.set_sensitive(false);
        tree_credentials.set_sensitive(false);

        w.show_all();

        result
    }

    fn refresh_tree(ui: &Rc<RefCell<Self>>) {
        let store_credentials = ui.borrow().store_credentials.clone();
        let search_criteria = ui
            .borrow()
            .entry_search_credentials
            .text()
            .as_str()
            .trim()
            .to_owned();
        let filter_key = if !search_criteria.is_empty() {
            Some(&search_criteria)
        } else {
            None
        };

        store_credentials.clear();

        for (name, record) in ui.borrow().db.as_ref().unwrap().data.iter() {
            let is_match = match filter_key {
                None => true,
                Some(key) => name.contains(key),
            };
            if is_match {
                let it = store_credentials.append(None);
                store_credentials.set_value(&it, 0, &glib::Value::from(&name));
                for attr_name in record.value.keys() {
                    let it2 = store_credentials.append(Some(&it));
                    store_credentials.set_value(&it2, 0, &glib::Value::from(&attr_name));
                }
            }
        }
    }

    fn show_attr(ui_ref: &Rc<RefCell<Self>>, key: &str, attr: &str) {
        let dialog_credinfo;
        let label_credinfo_key;

        {
            let ui = &mut *ui_ref.borrow_mut();

            dialog_credinfo = ui.dialog_credinfo.clone();
            label_credinfo_key = ui.label_credinfo_key.clone();
            let label_credinfo_attr = ui.label_credinfo_attr.clone();
            let entry_credinfo_value = ui.entry_credinfo_value.clone();

            let db = ui.db.as_ref().unwrap();

            let value = db.data.get(key).unwrap().value.get(attr).unwrap();

            ui.credinfo_value = Some((key.to_owned(), attr.to_owned(), value.clone()));

            label_credinfo_key.set_text(key);
            label_credinfo_attr.set_text(attr);
            entry_credinfo_value.set_text("<click \"refresh\" to reveal>");
        }

        dialog_credinfo.run();
        dialog_credinfo.hide();

        {
            let ui = &mut *ui_ref.borrow_mut();
            ui.credinfo_value = None;
        }
    }

    fn credinfo_reveal(ui_ref: &Rc<RefCell<Self>>) {
        let ui = &*ui_ref.borrow_mut();
        let value = ui.credinfo_value.as_ref().unwrap().2.clone();
        ui.entry_credinfo_value.set_text(&value);
    }

    fn credinfo_copy(ui_ref: &Rc<RefCell<Self>>) {
        let ui = &*ui_ref.borrow();
        let value = ui.credinfo_value.as_ref().unwrap().2.clone();
        let display = ui.window.display();
        let clipboard = gtk::Clipboard::default(&display).unwrap();
        clipboard.set_text(&value);

        let dlg = gtk::MessageDialog::new(
            Some(&ui.dialog_credinfo),
            gtk::DialogFlags::MODAL,
            gtk::MessageType::Error,
            gtk::ButtonsType::Close,
            "Copied the password to clipboard",
        );
        dlg.run();
        dlg.close();
    }
}

fn parse_args() -> DbLocation {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        DbLocation::DotLocal
    } else {
        let mut it = args.into_iter();
        let s = it.next().unwrap();
        DbLocation::SpecifiedDirectory(s)
    }
}

fn main() {
    let db_location = parse_args();

    gtk::init().expect("Unable to initialize Gtk+");

    let _ = Ui::new(db_location);

    gtk::main();
}
