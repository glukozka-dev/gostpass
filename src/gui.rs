use eframe::egui;
use egui::{Color32, Context, RichText, TextEdit};
use rfd::FileDialog;
use std::time::{Duration, Instant};
use arboard::Clipboard; // Добавьте в Cargo.toml: arboard = "3.2"
use crate::database::{DBentry, read_db, write_db, new_db};

#[derive(PartialEq)]
enum AppState {
    Login,
    Database,
    NewEntry,
    EditEntry(usize), // Храним индекс редактируемой записи
}

pub struct GostPassApp {
    state: AppState,
    db_path: Option<String>,
    db_key: String,
    db_entries: Vec<DBentry>,
    error_message: Option<(String, Instant)>,
    success_message: Option<(String, Instant)>, // Для сообщений об успешном копировании
    show_password: bool,
    
    // Поля для нового/редактируемого входа
    edit_login: String,
    edit_password: String,
    edit_url: String,
    
    // Для генерации пароля
    password_length: usize,
    use_uppercase: bool,
    use_lowercase: bool,
    use_numbers: bool,
    use_symbols: bool,
}

impl Default for GostPassApp {
    fn default() -> Self {
        Self {
            state: AppState::Login,
            db_path: None,
            db_key: String::new(),
            db_entries: Vec::new(),
            error_message: None,
            success_message: None,
            show_password: false,
            edit_login: String::new(),
            edit_password: String::new(),
            edit_url: String::new(),
            password_length: 16,
            use_uppercase: true,
            use_lowercase: true,
            use_numbers: true,
            use_symbols: true,
        }
    }
}

impl GostPassApp {
    fn show_error(&mut self, message: &str) {
        self.error_message = Some((message.to_string(), Instant::now()));
    }
    
    fn show_success(&mut self, message: &str) {
        self.success_message = Some((message.to_string(), Instant::now()));
    }
    
    fn copy_to_clipboard(&mut self, text: &str) {
        let mut clipboard = Clipboard::new().unwrap();
        if clipboard.set_text(text.to_owned()).is_ok() {
            self.show_success("Скопировано в буфер обмена");
        } else {
            self.show_error("Не удалось скопировать");
        }
    }
    
    fn generate_password(&self) -> String {
        use rand::Rng;
        let mut charset = String::new();
        
        if self.use_uppercase {
            charset.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        }
        if self.use_lowercase {
            charset.push_str("abcdefghijklmnopqrstuvwxyz");
        }
        if self.use_numbers {
            charset.push_str("0123456789");
        }
        if self.use_symbols {
            charset.push_str("!@#$%^&*()_+-=[]{}|;:,.<>?");
        }
        
        if charset.is_empty() {
            charset = "abcdefghijklmnopqrstuvwxyz".to_string();
        }
        
        let mut rng = rand::thread_rng();
        (0..self.password_length)
            .map(|_| {
                let idx = rng.gen_range(0..charset.len());
                charset.chars().nth(idx).unwrap()
            })
            .collect()
    }
    
    fn save_current_entry(&mut self) {
        if !self.edit_login.is_empty() && !self.edit_password.is_empty() {
            match self.state {
                AppState::EditEntry(index) => {
                    // Обновляем существующую запись
                    if let Some(entry) = self.db_entries.get_mut(index) {
                        entry.login = self.edit_login.clone();
                        entry.password = self.edit_password.clone();
                        entry.url = self.edit_url.clone();
                    }
                }
                _ => {
                    // Создаем новую запись
                    let new_entry = DBentry {
                        login: self.edit_login.clone(),
                        password: self.edit_password.clone(),
                        url: self.edit_url.clone(),
                    };
                    self.db_entries.push(new_entry);
                }
            }
            
            // Сохраняем в файл
            if let (Some(path), key) = (&self.db_path, &self.db_key) {
                write_db(path.clone(), key.clone(), self.db_entries.clone());
            }
            
            self.state = AppState::Database;
        } else {
            self.show_error("Заполните логин и пароль");
        }
    }
}

impl eframe::App for GostPassApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Проверяем, не прошло ли 5 секунд для сообщений
        if let Some((_, timestamp)) = self.error_message {
            if timestamp.elapsed() > Duration::from_secs(5) {
                self.error_message = None;
            }
        }
        if let Some((_, timestamp)) = self.success_message {
            if timestamp.elapsed() > Duration::from_secs(2) {
                self.success_message = None;
            }
        }
        
        match self.state {
            AppState::Login => self.render_login_window(ctx),
            AppState::Database => self.render_database_window(ctx),
            AppState::NewEntry | AppState::EditEntry(_) => self.render_entry_window(ctx),
        }
        
        // Запрашиваем перерисовку для анимации таймера
        if self.error_message.is_some() || self.success_message.is_some() {
            ctx.request_repaint();
        }
    }
}

impl GostPassApp {
    fn render_login_window(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Верхняя панель с кнопками
            ui.horizontal_top(|ui| {
                ui.style_mut().spacing.button_padding = egui::vec2(10.0, 5.0);
                
                if ui.button(RichText::new("Новая БД").size(16.0)).clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("GOST Database", &["db", "gdb"])
                        .save_file() 
                    {
                        if !self.db_key.is_empty() {
                            new_db(path.display().to_string(), self.db_key.clone());
                            self.db_path = Some(path.display().to_string());
                            if let Ok(entries) = read_db(self.db_path.clone().unwrap(), self.db_key.clone()) {
                                self.db_entries = entries;
                                self.state = AppState::Database;
                            }
                        } else {
                            self.show_error("Введите пароль для новой базы данных");
                        }
                    }
                }
                
                ui.add_space(10.0);
                
                if ui.button(RichText::new("Открыть БД").size(16.0)).clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("GOST Database", &["db", "gdb"])
                        .pick_file() 
                    {
                        self.db_path = Some(path.display().to_string());
                    }
                }
                
                // Показываем путь к текущей БД, если выбран
                if let Some(path) = &self.db_path {
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new(format!("Файл: {}", path))
                            .color(Color32::GRAY)
                            .italics()
                    );
                }
            });
            
            ui.add_space(50.0);
            
            // Центральная часть с формой входа
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                
                // Заголовок
                ui.heading(
                    RichText::new("GostPass")
                        .size(36.0)
                        .color(Color32::from_rgb(70, 130, 200))
                );
                
                ui.add_space(10.0);
                ui.label(
                    RichText::new("Безопасное хранилище паролей")
                        .size(14.0)
                        .color(Color32::GRAY)
                );
                
                ui.add_space(40.0);
                
                // Форма ввода пароля
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 3.0);
                    
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Пароль:")
                                    .size(16.0)
                            );
                            
                            // Поле ввода пароля с возможностью скрытия
                            let password_input = if self.show_password {
                                TextEdit::singleline(&mut self.db_key)
                                    .hint_text("Введите пароль")
                                    .desired_width(200.0)
                            } else {
                                TextEdit::singleline(&mut self.db_key)
                                    .password(true)
                                    .hint_text("Введите пароль")
                                    .desired_width(200.0)
                            };
                            
                            let response = ui.add(password_input);
                            
                            // Обработка нажатия Enter в поле пароля
                            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                if let Some(path) = &self.db_path {
                                    match read_db(path.clone(), self.db_key.clone()) {
                                        Ok(entries) => {
                                            self.db_entries = entries;
                                            self.state = AppState::Database;
                                        }
                                        Err(_) => {
                                            self.show_error("Неверный пароль");
                                        }
                                    }
                                } else {
                                    self.show_error("Сначала выберите файл базы данных");
                                }
                            }
                            
                            // Кнопка показа/скрытия пароля
                            if ui.button(if self.show_password { "Hide" } else { "Show" }).clicked() {
                                self.show_password = !self.show_password;
                            }
                        });
                        
                        ui.add_space(10.0);
                        
                        // Кнопка входа
                        let login_btn = ui.add_sized(
                            [200.0, 40.0],
                            egui::Button::new(
                                RichText::new("Войти")
                                    .size(16.0)
                            )
                        );
                        
                        if login_btn.clicked() {
                            if let Some(path) = &self.db_path {
                                match read_db(path.clone(), self.db_key.clone()) {
                                    Ok(entries) => {
                                        self.db_entries = entries;
                                        self.state = AppState::Database;
                                    }
                                    Err(_) => {
                                        self.show_error("Неверный пароль");
                                    }
                                }
                            } else {
                                self.show_error("Сначала выберите файл базы данных");
                            }
                        }
                    });
                    
                    ui.add_space(ui.available_width() / 3.0);
                });
                
                // Отображаем сообщения
                if let Some((message, _)) = &self.error_message {
                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() / 3.0);
                        ui.colored_label(Color32::RED, 
                            RichText::new(format!("Ошибка: {}", message)).size(14.0)
                        );
                        ui.add_space(ui.available_width() / 3.0);
                    });
                }
            });
        });
    }
    
    fn render_database_window(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Верхняя панель
            ui.horizontal(|ui| {
                ui.style_mut().spacing.button_padding = egui::vec2(10.0, 5.0);
                
                if ui.button(RichText::new("➕ Новая запись").size(16.0)).clicked() {
                    self.edit_login.clear();
                    self.edit_password.clear();
                    self.edit_url.clear();
                    self.state = AppState::NewEntry;
                }
                
                ui.add_space(10.0);
                
                if ui.button(RichText::new("🔒 Заблокировать БД").size(16.0)).clicked() {
                    self.db_key.clear();
                    self.db_path = None;
                    self.db_entries.clear();
                    self.state = AppState::Login;
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(path) = &self.db_path {
                        let display_path = if path.len() > 50 {
                            format!("...{}", &path[path.len()-50..])
                        } else {
                            path.clone()
                        };
                        ui.label(
                            RichText::new(format!("📁 {}", display_path))
                                .color(Color32::GRAY)
                                .italics()
                        );
                    }
                });
            });
            
            ui.add_space(20.0);
            
            if self.db_entries.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.label(
                        RichText::new("📭 База данных пуста")
                            .size(24.0)
                            .color(Color32::GRAY)
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Нажмите 'Новая запись' для добавления")
                            .size(14.0)
                            .color(Color32::LIGHT_GRAY)
                    );
                });
            } else {
                // Собираем все действия
                let mut to_delete = None;
                let mut to_edit = None;
                let mut to_copy_login = None;
                let mut to_copy_password = None;
                let mut to_copy_url = None;
                
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("entries_grid")
                        .striped(true)
                        .spacing([20.0, 5.0])
                        .min_col_width(100.0)
                        .show(ui, |ui| {
                            // Заголовки
                            ui.label(RichText::new("Логин").strong());
                            ui.label(RichText::new("Пароль").strong());
                            ui.label(RichText::new("Сайт").strong());
                            ui.label(RichText::new("Действия").strong());
                            ui.end_row();
                            
                            ui.separator();
                            ui.end_row();
                            
                            // Записи
                            for (index, entry) in self.db_entries.iter().enumerate() {
                                // Логин
                                let login_response = ui.label(&entry.login);
                                if login_response.clicked() {
                                    to_copy_login = Some(entry.login.clone());
                                }
                                login_response.on_hover_text("Нажмите чтобы скопировать логин");
                                
                                // Пароль
                                let masked_password: String = entry.password.chars().map(|_| '•').collect();
                                let pass_response = ui.label(masked_password);
                                if pass_response.clicked() {
                                    to_copy_password = Some(entry.password.clone());
                                }
                                pass_response.on_hover_text("Нажмите чтобы скопировать пароль");
                                
                                // Сайт
                                let url_response = ui.hyperlink_to(&entry.url, &entry.url);
                                if url_response.clicked() {
                                    to_copy_url = Some(entry.url.clone());
                                }
                                url_response.on_hover_text("Нажмите чтобы скопировать ссылку");
                                
                                // Кнопки действий
                                ui.horizontal(|ui| {
                                    if ui.button("✏️").clicked() {
                                        to_edit = Some(index);
                                    }
                                    if ui.button("🗑️").clicked() {
                                        to_delete = Some(index);
                                    }
                                });
                                
                                ui.end_row();
                            }
                        });
                });
                
                // Обработка всех действий ПОСЛЕ итерации
                if let Some(text) = to_copy_login {
                    self.copy_to_clipboard(&text);
                }
                if let Some(text) = to_copy_password {
                    self.copy_to_clipboard(&text);
                }
                if let Some(text) = to_copy_url {
                    self.copy_to_clipboard(&text);
                }
                
                if let Some(index) = to_delete {
                    self.db_entries.remove(index);
                    if let (Some(path), key) = (&self.db_path, &self.db_key) {
                        write_db(path.clone(), key.clone(), self.db_entries.clone());
                    }
                }
                
                if let Some(index) = to_edit {
                    if let Some(entry) = self.db_entries.get(index) {
                        self.edit_login = entry.login.clone();
                        self.edit_password = entry.password.clone();
                        self.edit_url = entry.url.clone();
                        self.state = AppState::EditEntry(index);
                    }
                }
            }
            
            // Отображаем сообщения
            if let Some((message, _)) = &self.success_message {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.colored_label(Color32::GREEN, 
                        RichText::new(format!("✓ {}", message)).size(14.0)
                    );
                });
            }
            
            if let Some((message, _)) = &self.error_message {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.colored_label(Color32::RED, 
                        RichText::new(format!("✗ {}", message)).size(14.0)
                    );
                });
            }
        });
    }

    fn render_entry_window(&mut self, ctx: &Context) {
        let is_edit = matches!(self.state, AppState::EditEntry(_));
        
        egui::CentralPanel::default().show(ctx, |ui| {
            // Заголовок
            ui.horizontal(|ui| {
                ui.heading(RichText::new(
                    if is_edit { "✏️ Редактирование записи" } else { "➕ Новая запись" }
                ).size(24.0));
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✖️ Закрыть").clicked() {
                        self.state = AppState::Database;
                    }
                });
            });
            
            ui.add_space(20.0);
            
            // Форма
            ui.vertical_centered(|ui| {
                ui.set_width(500.0);
                
                // Поле для логина
                ui.horizontal(|ui| {
                    ui.label(RichText::new("👤 Логин:").size(16.0));
                    ui.add_sized(
                        [300.0, 30.0],
                        TextEdit::singleline(&mut self.edit_login)
                            .hint_text("Введите логин")
                    );
                });
                
                ui.add_space(15.0);
                
                // Поле для пароля
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔑 Пароль:").size(16.0));
                    
                    ui.add_sized(
                        [200.0, 30.0],
                        TextEdit::singleline(&mut self.edit_password)
                            .hint_text("Введите пароль")
                    );
                    
                    if ui.button("🎲 Генерировать").clicked() {
                        self.edit_password = self.generate_password();
                    }
                });
                
                ui.add_space(10.0);
                
                // Настройки генерации пароля
                ui.collapsing("⚙️ Настройки генерации", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Длина:");
                        ui.add(egui::DragValue::new(&mut self.password_length).clamp_range(4..=64));
                    });
                    
                    ui.checkbox(&mut self.use_uppercase, "Заглавные буквы (A-Z)");
                    ui.checkbox(&mut self.use_lowercase, "Строчные буквы (a-z)");
                    ui.checkbox(&mut self.use_numbers, "Цифры (0-9)");
                    ui.checkbox(&mut self.use_symbols, "Спецсимволы (!@#$%...)");
                });
                
                ui.add_space(15.0);
                
                // Поле для сайта
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🌐 Сайт:").size(16.0));
                    ui.add_sized(
                        [300.0, 30.0],
                        TextEdit::singleline(&mut self.edit_url)
                            .hint_text("https://example.com")
                    );
                });
                
                ui.add_space(30.0);
                
                // Кнопки действий
                ui.horizontal(|ui| {
                    ui.add_space(50.0);
                    
                    if ui.add_sized([120.0, 40.0], egui::Button::new(
                        RichText::new(if is_edit { "💾 Обновить" } else { "💾 Сохранить" }).size(16.0)
                    )).clicked() {
                        self.save_current_entry();
                    }
                    
                    ui.add_space(20.0);
                    
                    if ui.add_sized([120.0, 40.0], egui::Button::new(
                        RichText::new("❌ Отмена").size(16.0)
                    )).clicked() {
                        self.state = AppState::Database;
                    }
                    
                    ui.add_space(50.0);
                });
            });
            
            // Отображаем сообщения
            if let Some((message, _)) = &self.error_message {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.colored_label(Color32::RED, 
                        RichText::new(format!("Ошибка: {}", message)).size(14.0)
                    );
                });
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 650.0])
            .with_min_inner_size([700.0, 500.0])
            .with_title("GostPass - Безопасное хранилище паролей"),
        ..Default::default()
    };
    
    eframe::run_native(
        "GostPass",
        options,
        Box::new(|_cc| Box::new(GostPassApp::default())),
    )
}