use crate::bus::{EventBus, SystemEvent};
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::sync::Arc;

/// Команда палитры
#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub usage: String,
    pub action: CommandAction,
    pub aliases: Vec<String>,
}

/// Действие команды
#[derive(Debug, Clone)]
pub enum CommandAction {
    /// Отправить новую цель
    SubmitGoal { task: String },
    /// Убить цель
    KillGoal { goal_id: String },
    /// Повторить задачу
    RetryTask { task_id: String },
    /// Показать логи задачи
    ShowTaskLogs { task_id: String },
    /// Переключить тему
    ToggleTheme,
    /// Показать помощь
    ShowHelp,
    /// Выход
    Quit,
    /// Переключить вкладку
    SwitchTab { tab: usize },
    /// Очистить логи
    ClearLogs,
    /// Статус системы
    ShowStatus,
    /// Перезагрузить демона
    RestartDaemon { daemon: String },
}

/// Состояние палитры команд
#[derive(Debug, Clone)]
pub struct CommandPalette {
    pub visible: bool,
    pub input: String,
    pub commands: Vec<Command>,
    pub filtered: Vec<usize>, // индексы в commands
    pub selected: usize,
    pub list_state: ListState,
}

impl CommandPalette {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let commands = vec![
            Command {
                name: "submit".to_string(),
                description: "Отправить новую задачу (goal)".to_string(),
                usage: ":submit "Описание задачи" [--priority N]".to_string(),
                action: CommandAction::SubmitGoal { task: String::new() },
                aliases: vec!["s".to_string(), "new".to_string(), "goal".to_string()],
            },
            Command {
                name: "kill".to_string(),
                description: "Убить цель/задачу".to_string(),
                usage: ":kill <goal_id|task_id>".to_string(),
                action: CommandAction::KillGoal { goal_id: String::new() },
                aliases: vec!["k".to_string(), "stop".to_string(), "cancel".to_string()],
            },
            Command {
                name: "retry".to_string(),
                description: "Повторить неудачную задачу".to_string(),
                usage: ":retry <task_id>".to_string(),
                action: CommandAction::RetryTask { task_id: String::new() },
                aliases: vec!["r".to_string(), "repeat".to_string()],
            },
            Command {
                name: "logs".to_string(),
                description: "Показать логи задачи".to_string(),
                usage: ":logs <task_id>".to_string(),
                action: CommandAction::ShowTaskLogs { task_id: String::new() },
                aliases: vec!["l".to_string(), "log".to_string()],
            },
            Command {
                name: "theme".to_string(),
                description: "Переключить тему (dark/light)".to_string(),
                usage: ":theme [dark|light]".to_string(),
                action: CommandAction::ToggleTheme,
                aliases: vec!["t".to_string()],
            },
            Command {
                name: "help".to_string(),
                description: "Показать справку по командам".to_string(),
                usage: ":help [command]".to_string(),
                action: CommandAction::ShowHelp,
                aliases: vec!["h".to_string(), "?".to_string()],
            },
            Command {
                name: "quit".to_string(),
                description: "Выйти из ANT OS".to_string(),
                usage: ":quit".to_string(),
                action: CommandAction::Quit,
                aliases: vec!["q".to_string(), "exit".to_string()],
            },
            Command {
                name: "tab".to_string(),
                description: "Переключить вкладку".to_string(),
                usage: ":tab <1-7|dashboard|memory|graph|logs|agents|git|help>".to_string(),
                action: CommandAction::SwitchTab { tab: 0 },
                aliases: vec![],
            },
            Command {
                name: "clear-logs".to_string(),
                description: "Очистить логи".to_string(),
                usage: ":clear-logs".to_string(),
                action: CommandAction::ClearLogs,
                aliases: vec!["cl".to_string()],
            },
            Command {
                name: "status".to_string(),
                description: "Показать статус системы".to_string(),
                usage: ":status".to_string(),
                action: CommandAction::ShowStatus,
                aliases: vec!["st".to_string()],
            },
            Command {
                name: "restart".to_string(),
                description: "Перезапустить демона".to_string(),
                usage: ":restart <daemon_name>".to_string(),
                action: CommandAction::RestartDaemon { daemon: String::new() },
                aliases: vec![],
            },
        ];

        Self {
            visible: false,
            input: String::new(),
            commands,
            filtered: (0..commands.len()).collect(),
            selected: 0,
            list_state,
        }
    }

    /// Открыть палитру
    pub fn open(&mut self) {
        self.visible = true;
        self.input.clear();
        self.filter();
        self.selected = 0;
        self.list_state.select(Some(0));
    }

    /// Закрыть палитру
    pub fn close(&mut self) {
        self.visible = false;
        self.input.clear();
    }

    /// Переключить видимость
    pub fn toggle(&mut self) {
        if self.visible {
            self.close();
        } else {
            self.open();
        }
    }

    /// Добавить символ к вводу
    pub fn push_char(&mut self, c: char) {
        self.input.push(c);
        self.filter();
        self.selected = 0;
        self.list_state.select(Some(0));
    }

    /// Удалить последний символ
    pub fn pop_char(&mut self) {
        self.input.pop();
        self.filter();
        self.selected = 0;
        self.list_state.select(Some(0));
    }

    /// Фильтрация команд
    fn filter(&mut self) {
        let query = self.input.trim().to_lowercase();
        
        if query.is_empty() {
            self.filtered = (0..self.commands.len()).collect();
        } else {
            // Проверяем, начинается ли с двоеточия (прямой вызов команды)
            if query.starts_with(':') {
                let cmd_query = &query[1..];
                self.filtered = self.commands.iter().enumerate()
                    .filter(|(_, cmd)| {
                        cmd.name.starts_with(cmd_query) || 
                        cmd.aliases.iter().any(|a| a.starts_with(cmd_query))
                    })
                    .map(|(i, _)| i)
                    .collect();
            } else {
                // Fuzzy search по имени, описанию, алиасам
                self.filtered = self.commands.iter().enumerate()
                    .filter(|(_, cmd)| {
                        cmd.name.contains(&query) ||
                        cmd.description.to_lowercase().contains(&query) ||
                        cmd.aliases.iter().any(|a| a.contains(&query))
                    })
                    .map(|(i, _)| i)
                    .collect();
            }
        }
        
        if self.filtered.is_empty() {
            self.filtered = (0..self.commands.len()).collect();
        }
    }

    /// Перемещение вверх
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.list_state.select(Some(self.selected));
        }
    }

    /// Перемещение вниз
    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
            self.list_state.select(Some(self.selected));
        }
    }

    /// Получить выбранную команду
    pub fn selected_command(&self) -> Option<&Command> {
        self.filtered.get(self.selected).and_then(|&i| self.commands.get(i))
    }

    /// Выполнить команду
    pub fn execute(&mut self, bus: &Arc<EventBus>) -> Option<CommandAction> {
        if let Some(cmd) = self.selected_command() {
            let action = self.parse_command(&cmd)?;
            self.close();
            Some(action)
        } else {
            // Попробуем распарсить ввод напрямую
            self.parse_input(bus)
        }
    }

    /// Парсинг ввода пользователя
    fn parse_input(&self, _bus: &Arc<EventBus>) -> Option<CommandAction> {
        let input = self.input.trim();
        
        if input.is_empty() {
            return None;
        }

        // Прямой вызов команды через двоеточие
        if input.starts_with(':') {
            let parts: Vec<&str> = input[1..].split_whitespace().collect();
            if parts.is_empty() {
                return None;
            }
            
            let cmd_name = parts[0].to_lowercase();
            let args = &parts[1..];
            
            return self.parse_direct_command(&cmd_name, args);
        }

        // Иначе считаем это задачей для submit
        Some(CommandAction::SubmitGoal { 
            task: input.to_string() 
        })
    }

    /// Парсинг прямой команды
    fn parse_direct_command(&self, cmd: &str, args: &[&str]) -> Option<CommandAction> {
        match cmd {
            "submit" | "s" | "new" | "goal" => {
                if args.is_empty() {
                    return None;
                }
                let task = args.join(" ");
                Some(CommandAction::SubmitGoal { task })
            }
            "kill" | "k" | "stop" | "cancel" => {
                if args.is_empty() {
                    return None;
                }
                Some(CommandAction::KillGoal { 
                    goal_id: args[0].to_string() 
                })
            }
            "retry" | "r" | "repeat" => {
                if args.is_empty() {
                    return None;
                }
                Some(CommandAction::RetryTask { 
                    task_id: args[0].to_string() 
                })
            }
            "logs" | "l" | "log" => {
                if args.is_empty() {
                    return None;
                }
                Some(CommandAction::ShowTaskLogs { 
                    task_id: args[0].to_string() 
                })
            }
            "theme" | "t" => Some(CommandAction::ToggleTheme),
            "help" | "h" | "?" => Some(CommandAction::ShowHelp),
            "quit" | "q" | "exit" => Some(CommandAction::Quit),
            "tab" => {
                if args.is_empty() {
                    return None;
                }
                let tab = match args[0].to_lowercase().as_str() {
                    "1" | "dashboard" => 0,
                    "2" | "memory" => 1,
                    "3" | "graph" => 2,
                    "4" | "logs" => 3,
                    "5" | "agents" => 4,
                    "6" | "git" => 5,
                    "7" | "help" => 6,
                    _ => return None,
                };
                Some(CommandAction::SwitchTab { tab })
            }
            "clear-logs" | "cl" => Some(CommandAction::ClearLogs),
            "status" | "st" => Some(CommandAction::ShowStatus),
            "restart" => {
                if args.is_empty() {
                    return None;
                }
                Some(CommandAction::RestartDaemon { 
                    daemon: args[0].to_string() 
                })
            }
            _ => None,
        }
    }

    /// Отрисовка палитры
    pub fn draw(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        if !self.visible {
            return;
        }

        // Центрированное окно
        let popup_area = centered_rect(60, 50, area);
        
        // Затемнение фона
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" ⌘ Command Palette ")
            .title_alignment(ratatui::layout::Alignment::Center)
            .border_style(Style::default().fg(theme.colors.accent))
            .style(Style::default().bg(theme.colors.background));

        f.render_widget(block.clone(), popup_area);

        let inner = block.inner(popup_area);
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Input line
                Constraint::Min(5),     // Results
                Constraint::Length(3),  // Help
            ])
            .split(inner);

        // Input line
        let input_text = format!(":{}", self.input);
        let input = Paragraph::new(input_text)
            .style(Style::default().fg(theme.colors.foreground).add_modifier(Modifier::BOLD))
            .block(Block::default()
                .borders(Borders::BOTTOM)
                .border_style(theme.border_style()));
        f.render_widget(input, chunks[0]);

        // Results list
        let items: Vec<ListItem> = self.filtered.iter()
            .map(|&i| {
                let cmd = &self.commands[i];
                let is_selected = self.filtered[self.selected] == i;
                
                let style = if is_selected {
                    Style::default()
                        .bg(theme.colors.selected)
                        .fg(theme.colors.foreground)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.colors.foreground)
                };

                let content = Line::from(vec![
                    Span::styled(
                        format!(":{:<15} ", cmd.name),
                        Style::default().fg(theme.colors.accent).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(cmd.description.clone(), style),
                ]);

                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::NONE))
            .highlight_style(Style::default()
                .bg(theme.colors.selected)
                .add_modifier(Modifier::BOLD));

        f.render_stateful_widget(list, chunks[1], &mut self.list_state);

        // Help text
        let help = Paragraph::new(Line::from(vec![
            Span::styled("↑/↓ ", Style::default().fg(theme.colors.accent)),
            Span::raw("Navigate  "),
            Span::styled("Enter ", Style::default().fg(theme.colors.accent)),
            Span::raw("Execute  "),
            Span::styled("Esc ", Style::default().fg(theme.colors.accent)),
            Span::raw("Close  "),
            Span::styled("Tab ", Style::default().fg(theme.colors.accent)),
            Span::raw("Complete"),
        ]))
        .block(Block::default()
            .borders(Borders::TOP)
            .border_style(theme.border_style()))
        .style(Style::default().fg(theme.colors.disabled));
        
        f.render_widget(help, chunks[2]);
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

/// Вспомогательная функция для центрированного прямоугольника
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
