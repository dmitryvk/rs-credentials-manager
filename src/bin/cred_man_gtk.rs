extern crate cred_man_lib;
extern crate linenoise;
extern crate rustc_serialize;
extern crate rand;
extern crate chrono;

extern crate gtk;
extern crate glib;

use gtk::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use cred_man_lib::{Db, DbLocation, DbLoadResult};

const BUILDER_UI: &'static str = include_str!("cred_man_gtk.ui");

struct Ui {
    window: gtk::Window,
    treeCredentials: gtk::TreeView,
    listStore: gtk::ListStore,
    btnUnlock: gtk::ToolButton,
    dlgPassword: gtk::Dialog,
    entryPassword: gtk::Entry,
    
    db: Option<Db>,
}

impl Ui {
    pub fn new() -> Rc<RefCell<Self>> {
        let b = gtk::Builder::new();
        
        b.add_from_string(BUILDER_UI).expect("Unable to load GtkBuilder definition");
        
        let w: gtk::Window = b.get_object("wndMain").expect("Unable to find wndMain in GtkBuilder definition");
        
        let treeCredentials: gtk::TreeView = b.get_object("treeCredentials").expect("Unable to find treeCredentials");
        
        let m: gtk::ListStore = b.get_object("listCredentials").expect("Unable to find treeCredentials");
        
        let btnUnlock: gtk::ToolButton = b.get_object("btnUnlock").expect("Unable to find btnUnlock");
        
        let dlgPassword: gtk::Dialog = b.get_object("dlgPassword").expect("Unable to find dlgPassword");
        
        let entryPassword: gtk::Entry = b.get_object("entryPassword").expect("Unable to find entryPassword");
        
        let result = Rc::new(RefCell::new(Ui {
            window: w.clone(),
            treeCredentials: treeCredentials.clone(),
            listStore: m.clone(),
            btnUnlock: btnUnlock.clone(),
            dlgPassword: dlgPassword.clone(),
            entryPassword: entryPassword.clone(),
            db: None,
        }));
        
        let result2 = result.clone();
        btnUnlock.connect_clicked(move |_| {
            result2.borrow().dlgPassword.show_all();
        });
        
        let result2 = result.clone();
        dlgPassword.connect_delete_event(move |_, _| {
            result2.borrow().dlgPassword.hide();
            Inhibit(true)
        });
        
        let result2 = result.clone();
        dlgPassword.connect_response(move |_, response| {
            if response == -<gtk::ResponseType as Into<i32>>::into(gtk::ResponseType::Ok) {
                let password = result2.borrow().entryPassword.get_text().unwrap_or("".to_owned());
                
                match Db::load(&DbLocation::DotLocal, &password) {
                    Ok(DbLoadResult::Loaded(db)) => {
                        result2.borrow_mut().db = Some(db);
                
                        result2.borrow().treeCredentials.set_sensitive(true);
                        result2.borrow().btnUnlock.set_sensitive(false);
                        result2.borrow().dlgPassword.hide();
                        
                        let listStore = result2.borrow().listStore.clone();
            
                        for name in result2.borrow().db.as_ref().unwrap().data.keys() {
                            let it = listStore.append();
                            listStore.set_value(&it, 0, &glib::Value::from(&name));
                        }
                    },
                    Ok(DbLoadResult::WrongPassword) => {
                        let dlg = gtk::MessageDialog::new(
                            Some(&result2.borrow().dlgPassword),
                            gtk::DIALOG_MODAL,
                            gtk::MessageType::Error,
                            gtk::ButtonsType::Close,
                            &"Wrong password"
                        );
                        dlg.run();
                        dlg.destroy();
                        return;
                    },
                    Err(e) => {
                        let dlg = gtk::MessageDialog::new(
                            Some(&result2.borrow().dlgPassword),
                            gtk::DIALOG_MODAL,
                            gtk::MessageType::Error,
                            gtk::ButtonsType::Close,
                            &format!("error: {:}", e)
                        );
                        dlg.run();
                        dlg.destroy();
                        return;
                    },
                }
            }
        });
        
        let result2 = result.clone();
        treeCredentials.connect_row_activated(move |_, path, _| {
            let store = result2.borrow().listStore.clone();
            let iter = store.get_iter(path).unwrap();
            let name = store.get_value(&iter, 0).get::<String>().unwrap();
            println!("chose {}", name);
        });
        
        w.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });
        
        treeCredentials.set_sensitive(false);
        
        w.show_all();
        
        result
    }
}

fn main() {
    gtk::init().expect("Unable to initialize Gtk+");
    
    let ui = Ui::new();
    
    gtk::main();
}
