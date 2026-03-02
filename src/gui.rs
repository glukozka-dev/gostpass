use crate::database::{self, DBentry, DbError};
use gtk::prelude::*;
use gtk::{gio, glib};
use gtk::{Application, ApplicationWindow, Button, Entry, Label, ListView, Revealer};
use gtk::{StringList, NoSelection, SignalListItemFactory, ListItem};
use std::cell::RefCell;
use std::rc::Rc;

const APP_ID: &str = "org.gluk0zka.gostpass";

// Состояние приложения
struct AppState {
    current_db_path: RefCell<Option<String>>,
    current_key: RefCell<Option<String>>,
    entries: RefCell<Vec<DBentry>>,
    entry_list_model: gtk::NoSelection,
}

impl AppState {
    fn new() -> Self {
        let string_list = StringList::new(&[] as &[&str]);
        let selection_model = NoSelection::new(Some(string_list));
        
        Self {
            current_db_path: RefCell::new(None),
            current_key: RefCell::new(None),
            entries: RefCell::new(Vec::new()),
            entry_list_model: selection_model,
        }
    }
    
    fn update_entries_list(&self) {
        let entries = self.entries.borrow();
        let string_list = self.entry_list_model
            .model()
            .and_downcast::<gtk::StringList>()
            .expect("Failed to get StringList");
        
        string_list.splice(0, string_list.n_items(), &[]);
        
        for entry in entries.iter() {
            let display_text = format!("{} | {} | {}", entry.login, entry.password, entry.url);
            string_list.append(&display_text);
        }
    }
}

pub fn gui() -> glib::ExitCode {
    // Create a new application
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);

    // Run the application
    app.run()
}

fn build_ui(app: &Application) {
    // Загружаем UI из XML
    let builder = gtk::Builder::from_string(include_str!("ui/gostpass.ui"));
    
    // Получаем главное окно
    let main_window: ApplicationWindow = builder
        .object("main_window")
        .expect("Failed to get main_window");
    main_window.set_application(Some(app));
    
    // Получаем окно базы данных
    let db_window: ApplicationWindow = builder
        .object("db_window")
        .expect("Failed to get db_window");
    db_window.set_application(Some(app));
    
    // Получаем окно новой записи
    let new_entry_window: gtk::Window = builder
        .object("new_entry_window")
        .expect("Failed to get new_entry_window");
    new_entry_window.set_transient_for(Some(&main_window));
    
    // Получаем виджеты главного окна
    let new_db_button: Button = builder.object("new_db_button").unwrap();
    let open_db_button: Button = builder.object("open_db_button").unwrap();
    let password_entry: Entry = builder.object("password_entry").unwrap();
    let submit_password_button: Button = builder.object("submit_password_button").unwrap();
    let error_revealer: Revealer = builder.object("error_revealer").unwrap();
    let error_label: Label = builder.object("error_label").unwrap();
    
    // Получаем виджеты окна базы данных
    let new_entry_button: Button = builder.object("new_entry_button").unwrap();
    let lock_db_button: Button = builder.object("lock_db_button").unwrap();
    let entries_listview: ListView = builder.object("entries_listview").unwrap();
    
    // Получаем виджеты окна новой записи
    let new_login_entry: Entry = builder.object("new_login_entry").unwrap();
    let new_password_entry: Entry = builder.object("new_password_entry").unwrap();
    let new_url_entry: Entry = builder.object("new_url_entry").unwrap();
    let generate_password_button: Button = builder.object("generate_password_button").unwrap();
    let save_entry_button: Button = builder.object("save_entry_button").unwrap();
    let cancel_entry_button: Button = builder.object("cancel_entry_button").unwrap();
    
    // Создаем состояние приложения
    let state = Rc::new(AppState::new());
    
    // Создаем фабрику для элементов списка
    let factory = SignalListItemFactory::new();
    
    // Настраиваем фабрику для создания виджетов
    factory.connect_setup(move |_, list_item| {
        if let Some(list_item) = list_item.downcast_ref::<ListItem>() {
            // Создаем контейнер для элемента
            let box_container = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            box_container.set_margin_start(12);
            box_container.set_margin_end(12);
            box_container.set_margin_top(6);
            box_container.set_margin_bottom(6);
            
            // Создаем метку для текста
            let label = gtk::Label::new(None);
            label.set_xalign(0.0);
            label.set_hexpand(true);
            
            // Создаем кнопку удаления
            let delete_button = gtk::Button::with_label("Удалить");
            delete_button.set_halign(gtk::Align::End);
            
            // Добавляем виджеты в контейнер
            box_container.append(&label);
            box_container.append(&delete_button);
            
            // Сохраняем метку и кнопку как свойства элемента списка
            list_item.set_child(Some(&box_container));
        }
    });
    
    // Настраиваем фабрику для привязки данных
    let state_clone = state.clone();
    factory.connect_bind(move |_, list_item| {
        if let Some(list_item) = list_item.downcast_ref::<ListItem>() {
            // Получаем индекс элемента
            let position = list_item.position();
            
            // Получаем данные из состояния
            let entries = state_clone.entries.borrow();
            if let Some(entry) = entries.get(position as usize) {
                // Получаем дочерний контейнер
                if let Some(box_container) = list_item.child().and_downcast::<gtk::Box>() {
                    // Ищем метку в контейнере (первый дочерний элемент)
                    if let Some(label) = box_container.first_child().and_downcast::<gtk::Label>() {
                        let display_text = format!("{} | {} | {}", entry.login, entry.password, entry.url);
                        label.set_text(&display_text);
                    }
                    
                    // Ищем кнопку удаления (второй дочерний элемент)
                    if let Some(delete_button) = box_container.last_child().and_downcast::<gtk::Button>() {
                        // Очищаем предыдущие обработчики
                        //delete_button.disconnect();
                        
                        let state_clone = state_clone.clone();
                        let entry_login = entry.login.clone();
                        
                        delete_button.connect_clicked(move |_| {
                            // Находим и удаляем запись
                            let mut entries = state_clone.entries.borrow_mut();
                            if let Some(index) = entries.iter().position(|e| e.login == entry_login) {
                                entries.remove(index);
                                
                                // Обновляем список
                                drop(entries); // Освобождаем заимствование перед обновлением
                                state_clone.update_entries_list();
                                
                                // Сохраняем изменения в файл
                                if let (Some(path), Some(key)) = (
                                    state_clone.current_db_path.borrow().clone(),
                                    state_clone.current_key.borrow().clone()
                                ) {
                                    let entries = state_clone.entries.borrow().clone();
                                    database::write_db(path, key, entries);
                                }
                            }
                        });
                    }
                }
            }
        }
    });
    
    // Устанавливаем фабрику для ListView
    entries_listview.set_factory(Some(&factory));
    
    // Настраиваем модель для ListView
    entries_listview.set_model(Some(&state.entry_list_model));
    
    // Создаем фильтр для выбора файлов
    let gtk_filter = gtk::FileFilter::new();
    gtk_filter.add_pattern("*.db");
    gtk_filter.set_name(Some("Database files"));
    
    // Остальная часть кода без изменений...
    
    // Кнопка "Новая БД"
    let state_clone = state.clone();
    let password_entry_clone = password_entry.clone();
    let error_revealer_clone = error_revealer.clone();
    let error_label_clone = error_label.clone();
    let main_window_clone = main_window.clone();
    let db_window_clone = db_window.clone();

    new_db_button.connect_clicked(move |_| {
        let file_dialog = gtk::FileDialog::new();
        file_dialog.set_title("Создать новую базу данных");
        file_dialog.set_initial_name(Some("database.db"));
        
        let state_clone = state_clone.clone();
        let password_entry_clone = password_entry_clone.clone();
        let error_revealer_clone = error_revealer_clone.clone();
        let error_label_clone = error_label_clone.clone();
        let db_window_clone = db_window_clone.clone();
        
        // Клонируем для передачи в file_dialog
        let main_window_for_dialog = main_window_clone.clone();
        // Клонируем для использования внутри замыкания
        let main_window_for_callback = main_window_clone.clone();
        
        file_dialog.save(Some(&main_window_for_dialog), None::<&gio::Cancellable>, move |result| {
            match result {
                Ok(file) => {
                    if let Some(path) = file.path() {
                        let path_string = path.to_string_lossy().to_string();
                        let password = password_entry_clone.text().to_string();
                        
                        if password.is_empty() {
                            error_label_clone.set_text("Введите пароль!");
                            error_revealer_clone.set_reveal_child(true);
                            return;
                        }
                        
                        // Создаем новую базу данных
                        database::new_db(path_string.clone(), password.clone());
                        
                        // Сохраняем состояние
                        *state_clone.current_db_path.borrow_mut() = Some(path_string);
                        *state_clone.current_key.borrow_mut() = Some(password);
                        *state_clone.entries.borrow_mut() = Vec::new();
                        state_clone.update_entries_list();
                        
                        // Скрываем главное окно и показываем окно БД
                        main_window_for_callback.set_visible(false);
                        db_window_clone.set_visible(true);
                        
                        error_revealer_clone.set_reveal_child(false);
                    }
                }
                Err(_) => {
                    // Пользователь отменил выбор
                }
            }
        });
    });
    
    // Кнопка "Открыть БД"
    let state_clone = state.clone();
    let password_entry_clone = password_entry.clone();
    let error_revealer_clone = error_revealer.clone();
    let error_label_clone = error_label.clone();
    let main_window_clone = main_window.clone();
    let db_window_clone = db_window.clone();

    open_db_button.connect_clicked(move |_| {
        let file_dialog = gtk::FileDialog::new();
        file_dialog.set_title("Открыть базу данных");
        
        let state_clone = state_clone.clone();
        let password_entry_clone = password_entry_clone.clone();
        let error_revealer_clone = error_revealer_clone.clone();
        let error_label_clone = error_label_clone.clone();
        let db_window_clone = db_window_clone.clone();
        
        // Клонируем для передачи в file_dialog
        let main_window_for_dialog = main_window_clone.clone();
        // Клонируем для использования внутри замыкания
        let main_window_for_callback = main_window_clone.clone();
        
        file_dialog.open(Some(&main_window_for_dialog), None::<&gio::Cancellable>, move |result| {
            match result {
                Ok(file) => {
                    if let Some(path) = file.path() {
                        let path_string = path.to_string_lossy().to_string();
                        let password = password_entry_clone.text().to_string();
                        
                        if password.is_empty() {
                            error_label_clone.set_text("Введите пароль!");
                            error_revealer_clone.set_reveal_child(true);
                            return;
                        }
                        
                        // Читаем базу данных
                        match database::read_db(path_string.clone(), password.clone()) {
                            Ok(entries) => {
                                // Сохраняем состояние
                                *state_clone.current_db_path.borrow_mut() = Some(path_string);
                                *state_clone.current_key.borrow_mut() = Some(password);
                                *state_clone.entries.borrow_mut() = entries;
                                state_clone.update_entries_list();
                                
                                // Скрываем главное окно и показываем окно БД
                                main_window_for_callback.set_visible(false);
                                db_window_clone.set_visible(true);
                                
                                error_revealer_clone.set_reveal_child(false);
                            }
                            Err(DbError::InvalidHeader) => {
                                error_label_clone.set_text("Неверный пароль или формат файла!");
                                error_revealer_clone.set_reveal_child(true);
                            }
                            Err(DbError::FileReadError) => {
                                error_label_clone.set_text("Ошибка чтения файла!");
                                error_revealer_clone.set_reveal_child(true);
                            }
                            Err(_) => {
                                error_label_clone.set_text("Ошибка расшифровки!");
                                error_revealer_clone.set_reveal_child(true);
                            }
                        }
                    }
                }
                Err(_) => {
                    // Пользователь отменил выбор
                }
            }
        });
    });
        
    // Кнопка "Ввод" (подтверждение пароля)
    let state_clone = state.clone();
    let error_revealer_clone = error_revealer.clone();
    let error_label_clone = error_label.clone();
    let main_window_clone = main_window.clone();
    let db_window_clone = db_window.clone();
    let password_entry_clone = password_entry.clone();
    
    submit_password_button.connect_clicked(move |_| {
        let path = state_clone.current_db_path.borrow().clone();
        let password = password_entry_clone.text().to_string();
        
        if let Some(path_string) = path {
            if password.is_empty() {
                error_label_clone.set_text("Введите пароль!");
                error_revealer_clone.set_reveal_child(true);
                return;
            }
            
            match database::read_db(path_string, password.clone()) {
                Ok(entries) => {
                    *state_clone.current_key.borrow_mut() = Some(password);
                    *state_clone.entries.borrow_mut() = entries;
                    state_clone.update_entries_list();
                    
                    main_window_clone.set_visible(false);
                    db_window_clone.set_visible(true);
                    
                    error_revealer_clone.set_reveal_child(false);
                }
                Err(DbError::InvalidHeader) => {
                    error_label_clone.set_text("Неверный пароль или формат файла!");
                    error_revealer_clone.set_reveal_child(true);
                }
                Err(DbError::FileReadError) => {
                    error_label_clone.set_text("Ошибка чтения файла!");
                    error_revealer_clone.set_reveal_child(true);
                }
                Err(_) => {
                    error_label_clone.set_text("Неверный пароль!");
                    error_revealer_clone.set_reveal_child(true);
                }
            }
        } else {
            error_label_clone.set_text("Сначала выберите или создайте БД!");
            error_revealer_clone.set_reveal_child(true);
        }
    });
    
    // Кнопка "Создать запись"
    let new_entry_window_clone = new_entry_window.clone();
    let new_login_entry_clone = new_login_entry.clone();
    let new_password_entry_clone = new_password_entry.clone();
    let new_url_entry_clone = new_url_entry.clone();
    
    new_entry_button.connect_clicked(move |_| {
        new_login_entry_clone.set_text("");
        new_password_entry_clone.set_text("");
        new_url_entry_clone.set_text("");
        new_entry_window_clone.set_visible(true);
    });
    
    // Кнопка "Сгенерировать пароль"
    let new_password_entry_clone = new_password_entry.clone();
    
    generate_password_button.connect_clicked(move |_| {
        // Генерируем простой пароль
        let password: String = String::from("1234");
        new_password_entry_clone.set_text(&password);
    });
    
    // Кнопка "Сохранить" (новая запись)
    let state_clone = state.clone();
    let new_entry_window_clone = new_entry_window.clone();
    let new_login_entry_clone = new_login_entry.clone();
    let new_password_entry_clone = new_password_entry.clone();
    let new_url_entry_clone = new_url_entry.clone();
    
    save_entry_button.connect_clicked(move |_| {
        let login = new_login_entry_clone.text().to_string();
        let password = new_password_entry_clone.text().to_string();
        let url = new_url_entry_clone.text().to_string();
        
        if login.is_empty() || password.is_empty() {
            // Показываем ошибку (можно добавить отдельный revealer)
            return;
        }
        
        let new_entry = DBentry { login, password, url };
        
        // Добавляем запись в состояние
        state_clone.entries.borrow_mut().push(new_entry);
        state_clone.update_entries_list();
        
        // Сохраняем базу данных
        if let (Some(path), Some(key)) = (
            state_clone.current_db_path.borrow().clone(),
            state_clone.current_key.borrow().clone()
        ) {
            let entries = state_clone.entries.borrow().clone();
            database::write_db(path, key, entries);
        }
        
        new_entry_window_clone.set_visible(false);
    });
    
    // Кнопка "Отмена"
    let new_entry_window_clone = new_entry_window.clone();
    cancel_entry_button.connect_clicked(move |_| {
        new_entry_window_clone.set_visible(false);
    });
    
    // Кнопка "Заблокировать БД"
    let main_window_clone = main_window.clone();
    let db_window_clone = db_window.clone();
    let password_entry_clone = password_entry.clone();
    
    lock_db_button.connect_clicked(move |_| {
        password_entry_clone.set_text("");
        db_window_clone.set_visible(false);
        main_window_clone.set_visible(true);
    });
    
    // Обработка закрытия окон
    main_window.connect_close_request(move |window| {
        window.set_visible(false);
        glib::Propagation::Stop
    });
    
    db_window.connect_close_request(move |_| {
        // При закрытии окна БД просто скрываем его
        glib::Propagation::Stop
    });
    
    // Показываем главное окно
    main_window.present();
}