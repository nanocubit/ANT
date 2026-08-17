//! Система тем для TUI
//! Поддержка тёмной и светлой темы

use ratatui::style::{Color, Style};

/// Тип темы
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeType {
    #[default]
    Dark,
    Light,
}

impl ThemeType {
    pub fn toggle(self) -> Self {
        match self {
            ThemeType::Dark => ThemeType::Light,
            ThemeType::Light => ThemeType::Dark,
        }
    }
}

/// Конфигурация темы
#[derive(Debug, Clone)]
pub struct Theme {
    pub theme_type: ThemeType,
    pub colors: ThemeColors,
}

/// Цвета темы
#[derive(Debug, Clone)]
pub struct ThemeColors {
    // Основные цвета
    pub background: Color,
    pub foreground: Color,
    
    // Цвета акцентов
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    
    // Цвета статусов
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    
    // Цвета для компонентов
    pub border: Color,
    pub selected: Color,
    pub disabled: Color,
    
    // Цвета для графиков
    pub graph_cpu: Color,
    pub graph_ram: Color,
    pub graph_network: Color,
    
    // Цвета для вкладок
    pub tab_active: Color,
    pub tab_inactive: Color,
}

impl Theme {
    /// Создать тёмную тему по умолчанию
    pub fn dark() -> Self {
        Self {
            theme_type: ThemeType::Dark,
            colors: ThemeColors {
                background: Color::Rgb(30, 30, 30),
                foreground: Color::Rgb(220, 220, 220),
                primary: Color::Rgb(0, 150, 255),
                secondary: Color::Rgb(100, 100, 255),
                accent: Color::Rgb(255, 100, 100),
                success: Color::Rgb(0, 255, 100),
                warning: Color::Rgb(255, 200, 0),
                error: Color::Rgb(255, 50, 50),
                info: Color::Rgb(0, 200, 255),
                border: Color::Rgb(80, 80, 80),
                selected: Color::Rgb(60, 60, 100),
                disabled: Color::Rgb(100, 100, 100),
                graph_cpu: Color::Rgb(0, 255, 100),
                graph_ram: Color::Rgb(0, 150, 255),
                graph_network: Color::Rgb(255, 150, 0),
                tab_active: Color::Rgb(0, 150, 255),
                tab_inactive: Color::Rgb(150, 150, 150),
            },
        }
    }

    /// Создать светлую тему
    pub fn light() -> Self {
        Self {
            theme_type: ThemeType::Light,
            colors: ThemeColors {
                background: Color::Rgb(250, 250, 250),
                foreground: Color::Rgb(30, 30, 30),
                primary: Color::Rgb(0, 100, 200),
                secondary: Color::Rgb(80, 80, 200),
                accent: Color::Rgb(200, 50, 50),
                success: Color::Rgb(0, 180, 50),
                warning: Color::Rgb(200, 150, 0),
                error: Color::Rgb(200, 0, 0),
                info: Color::Rgb(0, 150, 200),
                border: Color::Rgb(180, 180, 180),
                selected: Color::Rgb(200, 200, 230),
                disabled: Color::Rgb(150, 150, 150),
                graph_cpu: Color::Rgb(0, 180, 50),
                graph_ram: Color::Rgb(0, 100, 200),
                graph_network: Color::Rgb(200, 120, 0),
                tab_active: Color::Rgb(0, 100, 200),
                tab_inactive: Color::Rgb(100, 100, 100),
            },
        }
    }

    /// Создать тему по умолчанию (тёмная)
    pub fn default() -> Self {
        Self::dark()
    }

    /// Переключить тему
    pub fn toggle(&mut self) {
        *self = match self.theme_type {
            ThemeType::Dark => Self::light(),
            ThemeType::Light => Self::dark(),
        };
    }

    /// Получить стиль для текста
    pub fn text_style(&self) -> Style {
        Style::default().fg(self.colors.foreground)
    }

    /// Получить стиль для заголовка
    pub fn title_style(&self) -> Style {
        Style::default()
            .fg(self.colors.primary)
            .bold()
    }

    /// Получить стиль для выделения
    pub fn selected_style(&self) -> Style {
        Style::default()
            .bg(self.colors.selected)
            .bold()
    }

    /// Получить стиль для успеха
    pub fn success_style(&self) -> Style {
        Style::default().fg(self.colors.success)
    }

    /// Получить стиль для предупреждения
    pub fn warning_style(&self) -> Style {
        Style::default().fg(self.colors.warning)
    }

    /// Получить стиль для ошибки
    pub fn error_style(&self) -> Style {
        Style::default().fg(self.colors.error)
    }

    /// Получить стиль для границы
    pub fn border_style(&self) -> Style {
        Style::default().fg(self.colors.border)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Менеджер тем
#[derive(Debug, Clone)]
pub struct ThemeManager {
    current_theme: Theme,
    available_themes: Vec<ThemeType>,
}

impl ThemeManager {
    pub fn new() -> Self {
        Self {
            current_theme: Theme::default(),
            available_themes: vec![ThemeType::Dark, ThemeType::Light],
        }
    }

    pub fn get_theme(&self) -> &Theme {
        &self.current_theme
    }

    pub fn get_theme_mut(&mut self) -> &mut Theme {
        &mut self.current_theme
    }

    pub fn toggle(&mut self) {
        self.current_theme.toggle();
    }

    pub fn set_theme(&mut self, theme_type: ThemeType) {
        self.current_theme = match theme_type {
            ThemeType::Dark => Theme::dark(),
            ThemeType::Light => Theme::light(),
        };
    }

    pub fn get_available_themes(&self) -> &[ThemeType] {
        &self.available_themes
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}
