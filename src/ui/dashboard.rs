use crate::bus::{EventBus, SystemEvent};
use crate::ui::theme::{Theme, ThemeManager, ThemeType};
use crate::ui::dag_editor::{DagEditorState, DagEditorCommand, draw_dag_editor};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, MouseEventKind, MouseButton},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap, Gauge, Sparkline, Tabs},
    Frame, Terminal,
};
use std::{
    io::stdout,
    sync::{Arc, Mutex},
    collections::HashMap,
    time::Duration,
};

/// Вкладки TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Memory,
    Graph,
    Logs,
    Agents,
    Git,
    Help,
}

impl Tab {
    pub fn titles() -> Vec<&'static str> {
        vec!["📊 Dashboard", "🧠 Memory", "📈 Graph", "📜 Logs", "🤖 Agents", "🔗 Git", "❓ Help"]
    }

    pub fn all() -> Vec<Tab> {
        vec![Tab::Dashboard, Tab::Memory, Tab::Graph, Tab::Logs, Tab::Agents, Tab::Git, Tab::Help]
    }
}

/// Состояние приложения TUI
#[derive(Clone)]
pub struct DashboardState {
    pub dags: Arc<Mutex<HashMap<String, Vec<String>>>>,
    pub daemons: Arc<Mutex<HashMap<String, String>>>,
    pub logs: Arc<Mutex<Vec<String>>>,
    pub goals: Arc<Mutex<Vec<GoalInfo>>>,
    pub input_text: String,
    pub chat_history: Vec<String>,
    pub memory_stats: Option<()>,
    pub memory_documents: Vec<()>,
    pub memory_search_results: Vec<()>,
    pub memory_search_query: String,
    pub ram_mb: Arc<Mutex<u64>>,
    pub cpu_percent: Arc<Mutex<f32>>,
    pub active_tab: Tab,
    pub scroll_offset: usize,
    pub memory_scroll: ListState,
    pub log_scroll: ListState,
    pub git_status: Option<GitStatusInfo>,
    pub agents_status: Vec<AgentStatusInfo>,
    pub resource_history: Arc<Mutex<Vec<(u64, f32)>>>, // (RAM, CPU) история
    pub show_memory_panel: bool,
    pub memory_search_mode: bool,
    pub theme_manager: ThemeManager,
    pub dag_editor_state: DagEditorState,
}

#[derive(Clone)]
pub struct GoalInfo {
    pub id: String,
    pub task: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Clone)]
pub struct GitStatusInfo {
    pub branch: String,
    pub changes: usize,
    pub is_clean: bool,
}

#[derive(Clone)]
pub struct AgentStatusInfo {
    pub name: String,
    pub available: bool,
    pub version: Option<String>,
}

impl DashboardState {
    pub fn new() -> Self {
        let mut memory_scroll = ListState::default();
        memory_scroll.select(Some(0));
        
        let mut log_scroll = ListState::default();
        log_scroll.select(Some(0));

        Self {
            dags: Arc::new(Mutex::new(HashMap::new())),
            daemons: Arc::new(Mutex::new(HashMap::new())),
            logs: Arc::new(Mutex::new(Vec::new())),
            goals: Arc::new(Mutex::new(Vec::new())),
            input_text: String::new(),
            chat_history: vec!["🦀 ANT OS v8.0: Система готова. Введите команду.".to_string()],
            memory_stats: None,
            memory_documents: Vec::new(),
            memory_search_results: Vec::new(),
            memory_search_query: String::new(),
            ram_mb: Arc::new(Mutex::new(0)),
            cpu_percent: Arc::new(Mutex::new(0.0)),
            active_tab: Tab::Dashboard,
            scroll_offset: 0,
            memory_scroll,
            log_scroll,
            git_status: None,
            agents_status: Vec::new(),
            resource_history: Arc::new(Mutex::new(Vec::new())),
            show_memory_panel: false,
            memory_search_mode: false,
            theme_manager: ThemeManager::new(),
            dag_editor_state: DagEditorState::new(),
        }
    }
}

impl Default for DashboardState {
    fn default() -> Self {
        Self::new()
    }
}

/// Основной цикл TUI
pub async fn run_ui(bus: Arc<EventBus>) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;

    let mut state = DashboardState::new();

    // Подписка на события
    let state_clone = state.clone();
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            match ev {
                SystemEvent::DaemonStatus { daemon, status } => {
                    state_clone.daemons.lock().unwrap().insert(daemon, status);
                }
                SystemEvent::Log { source, message, level } => {
                    let mut l = state_clone.logs.lock().unwrap();
                    let timestamp = chrono::Local::now().format("%H:%M:%S");
                    l.push(format!("[{}] [{}] {}: {}", timestamp, level, source, message));
                    if l.len() > 200 {
                        l.remove(0);
                    }
                }
                SystemEvent::TaskDispatched { task_id, tool, input } => {
                    state_clone.dags.lock().unwrap().insert(
                        task_id.clone(),
                        vec![tool, "RUNNING".into(), input],
                    );
                }
                SystemEvent::TaskCompleted { task_id, result } => {
                    if let Some(t) = state_clone.dags.lock().unwrap().get_mut(&task_id) {
                        t[1] = "DONE".into();
                        t.push(result.chars().take(50).collect());
                    }
                }
                SystemEvent::TaskFailed { task_id, error } => {
                    if let Some(t) = state_clone.dags.lock().unwrap().get_mut(&task_id) {
                        t[1] = "FAILED".into();
                        t.push(error);
                    }
                }
                SystemEvent::GoalCreated { id, task } => {
                    let mut goals = state_clone.goals.lock().unwrap();
                    goals.push(GoalInfo {
                        id,
                        task,
                        status: "PLANNING".into(),
                        created_at: chrono::Local::now().format("%H:%M:%S").to_string(),
                    });
                }
                SystemEvent::GoalCompleted { id, result } => {
                    let mut goals = state_clone.goals.lock().unwrap();
                    if let Some(goal) = goals.iter_mut().find(|g| g.id == id) {
                        goal.status = "COMPLETED".into();
                    }
                }
                SystemEvent::GoalFailed { id, reason } => {
                    let mut goals = state_clone.goals.lock().unwrap();
                    if let Some(goal) = goals.iter_mut().find(|g| g.id == id) {
                        goal.status = "FAILED".into();
                    }
                }
                _ => {}
            }
        }
    });

    // Запуск фонового сбора метрик
    let metrics_state = state.clone();
    tokio::spawn(async move {
        let mut sys = sysinfo::System::new();
        loop {
            sys.refresh_memory();
            sys.refresh_cpu_usage();
            
            let ram_used = sys.used_memory() / 1024 / 1024;
            let cpu = sys.global_cpu_info().cpu_usage();
            
            *metrics_state.ram_mb.lock().unwrap() = ram_used;
            *metrics_state.cpu_percent.lock().unwrap() = cpu;
            
            // Добавляем в историю
            let mut history = metrics_state.resource_history.lock().unwrap();
            history.push((ram_used, cpu));
            if history.len() > 50 {
                history.remove(0);
            }
            
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    // Главный цикл
    loop {
        terminal.draw(|f| draw_ui(f, &mut state))?;

        if event::poll(Duration::from_millis(100))? {
            // Сначала проверяем мышь для DAG редактора
            if let CEvent::Mouse(mouse) = event::read()? {
                if state.active_tab == Tab::Dashboard && !state.command_palette.visible {
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            state.dag_editor_state.handle_mouse_press(mouse.row, mouse.column);
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            state.dag_editor_state.handle_mouse_drag(mouse.row, mouse.column);
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            state.dag_editor_state.handle_mouse_release();
                        }
                        _ => {}
                    }
                }
            }
            
            if let CEvent::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char(':') if state.input_text.is_empty() && !state.memory_search_mode => {
                        state.command_palette.toggle();
                    }
                    KeyCode::Char('q') | KeyCode::Esc if state.input_text.is_empty() && !state.command_palette.visible => break,
                    KeyCode::Enter => {
                        if state.command_palette.visible {
                            if let Some(action) = state.command_palette.execute(&bus) {
                                handle_command_action(action, &bus, &mut state);
                            }
                        } else if !state.input_text.is_empty() {
                            let cmd = state.input_text.clone();
                            state.chat_history.push(format!("🧑‍💻: {}", cmd));

                            bus.emit(SystemEvent::GoalCreated {
                                id: format!("G-{}", chrono::Local::now().format("%H%M%S")),
                                task: cmd.clone(),
                            });

                            state.input_text.clear();
                            state.scroll_offset = 0;
                        }
                    }
                    KeyCode::Char('m') => {
                        if !state.command_palette.visible {
                            state.show_memory_panel = !state.show_memory_panel;
                        }
                    }
                    KeyCode::Char('t') => {
                        if !state.command_palette.visible {
                            // Переключение темы
                            state.theme_manager.toggle();
                        }
                    }
                    KeyCode::Char('r') => {
                        if !state.command_palette.visible {
                            // Refresh memory search
                            state.memory_search_mode = true;
                        }
                    }
                    KeyCode::Char('d') if state.input_text.is_empty() && !state.command_palette.visible => {
                        // DAG editor mode - переключение на Dashboard
                        state.active_tab = Tab::Dashboard;
                    }
                    KeyCode::Delete if state.input_text.is_empty() && !state.command_palette.visible => {
                        // Удаление выбранного узла в DAG редакторе
                        if let Some(ref node_id) = state.dag_editor_state.selected_node {
                            // TODO: Реализовать удаление узла
                            state.dag_editor_state.selected_node = None;
                        }
                    }
                    KeyCode::Up => {
                        if state.command_palette.visible {
                            state.command_palette.move_up();
                        } else if state.memory_search_mode {
                            let selected = state.memory_scroll.selected().unwrap_or(0);
                            if selected > 0 {
                                state.memory_scroll.select(Some(selected - 1));
                            }
                        } else if state.scroll_offset > 0 {
                            state.scroll_offset -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if state.command_palette.visible {
                            state.command_palette.move_down();
                        } else if state.memory_search_mode {
                            let selected = state.memory_scroll.selected().unwrap_or(0);
                            state.memory_scroll.select(Some(selected + 1));
                        } else {
                            state.scroll_offset += 1;
                        }
                    }
                    KeyCode::Esc => {
                        if state.command_palette.visible {
                            state.command_palette.close();
                        } else {
                            state.memory_search_mode = false;
                        }
                    }
                    KeyCode::Tab => {
                        if state.command_palette.visible {
                            // Auto-complete
                            if let Some(cmd) = state.command_palette.selected_command() {
                                state.command_palette.input = format!(":{} ", cmd.name);
                                state.command_palette.filter();
                            }
                        } else if state.input_text.is_empty() && !state.memory_search_mode {
                            let tabs = Tab::all();
                            let current_idx = tabs.iter().position(|t| t == &state.active_tab).unwrap_or(0);
                            state.active_tab = tabs[(current_idx + 1) % tabs.len()];
                        }
                    }
                    KeyCode::BackTab => {
                        if state.input_text.is_empty() && !state.memory_search_mode && !state.command_palette.visible {
                            let tabs = Tab::all();
                            let current_idx = tabs.iter().position(|t| t == &state.active_tab).unwrap_or(0);
                            state.active_tab = tabs[(current_idx + tabs.len() - 1) % tabs.len()];
                        }
                    }
                    KeyCode::Char(c) => {
                        if state.command_palette.visible {
                            state.command_palette.push_char(c);
                        } else if state.memory_search_mode {
                            state.memory_search_query.push(c);
                        } else {
                            state.input_text.push(c);
                        }
                    }
                    KeyCode::Backspace => {
                        if state.command_palette.visible {
                            state.command_palette.pop_char();
                        } else if state.memory_search_mode {
                            state.memory_search_query.pop();
                        } else {
                            state.input_text.pop();
                        }
                    }
                    _ => {}
                }
            }
            
            // Mouse scroll handling (outside command palette)
            if let CEvent::Mouse(mouse) = event::read()? {
                if !state.command_palette.visible {
                    if mouse.kind == MouseEventKind::ScrollDown {
                        state.scroll_offset += 1;
                    } else if mouse.kind == MouseEventKind::ScrollUp {
                        if state.scroll_offset > 0 {
                            state.scroll_offset -= 1;
                        }
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}

}

/// Отрисовка UI
fn draw_ui(f: &mut Frame, state: &mut DashboardState) {
    let area = f.size();
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Status bar
            Constraint::Length(3),  // Tabs
            Constraint::Min(10),    // Main content
            Constraint::Length(3),  // Input
        ])
        .split(area);

    // Status bar
    draw_status_bar(f, main_chunks[0], state);

    // Tabs
    draw_tabs(f, main_chunks[1], state.active_tab, state.theme_manager.get_theme());

    // Main content based on active tab
    match state.active_tab {
        Tab::Dashboard => draw_dashboard(f, main_chunks[2], state),
        Tab::Memory => draw_memory(f, main_chunks[2], state),
        Tab::Graph => draw_graph(f, main_chunks[2], state),
        Tab::Logs => draw_logs(f, main_chunks[2], state),
        Tab::Agents => draw_agents(f, main_chunks[2], state),
        Tab::Git => draw_git(f, main_chunks[2], state),
        Tab::Help => draw_help(f, main_chunks[2], state),
    }

    // Input field
    draw_input(f, main_chunks[3], state);
}

fn draw_status_bar(f: &mut Frame, area: Rect, state: &DashboardState) {
    let theme = state.theme_manager.get_theme();
    
    let ram = *state.ram_mb.lock().unwrap();
    let cpu = *state.cpu_percent.lock().unwrap();
    
    let status_text = format!(
        " 🦀 ANT OS v8.0 | RAM: {} MB | CPU: {:.1}% | Goals: {} | Theme: {:?} ",
        ram,
        cpu,
        state.goals.lock().unwrap().len(),
        state.theme_manager.get_theme().theme_type,
    );

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(theme.colors.foreground).bg(theme.colors.primary))
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_style()));

    f.render_widget(status, area);
}

fn draw_tabs(f: &mut Frame, area: Rect, active_tab: Tab, theme: &Theme) {
    let titles: Vec<Line> = Tab::titles()
        .iter()
        .map(|t| Line::from(*t))
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default()
            .borders(Borders::ALL)
            .title("Navigation")
            .border_style(theme.border_style()))
        .select(Tab::all().iter().position(|t| t == &active_tab).unwrap_or(0))
        .style(Style::default().fg(theme.colors.tab_inactive))
        .highlight_style(Style::default()
            .fg(theme.colors.tab_active)
            .add_modifier(Modifier::BOLD));

    f.render_widget(tabs, area);
}

fn draw_dashboard(f: &mut Frame, area: Rect, state: &mut DashboardState) {
    let theme = state.theme_manager.get_theme();
    
    // Получаем данные о задачах
    let dags_guard = state.dags.lock().unwrap();
    
    // Преобразуем данные в TaskNode для редактора
    let mut tasks = HashMap::new();
    let mut statuses = HashMap::new();
    
    for (id, data) in dags_guard.iter() {
        let tool = data.get(0).cloned().unwrap_or_else(|| "unknown".to_string());
        let input = data.get(2).cloned().unwrap_or_else(|| data.get(1).cloned().unwrap_or_default());
        let status_str = data.get(1).cloned().unwrap_or_default();
        
        let task_node = TaskNode {
            id: id.clone(),
            tool,
            input,
            depends_on: vec![], // TODO: Добавить зависимости
        };
        
        let task_status = match status_str.as_str() {
            "DONE" => TaskStatus::Completed("Completed".to_string()),
            "FAILED" => TaskStatus::Failed("Failed".to_string()),
            "RUNNING" => TaskStatus::Running,
            _ => TaskStatus::Pending,
        };
        
        tasks.insert(id.clone(), task_node);
        statuses.insert(id.clone(), task_status);
    }
    
    drop(dags_guard);
    
    // Рисуем DAG редактор
    draw_dag_editor(f, area, &tasks, &statuses, &mut state.dag_editor_state, theme);
}

fn draw_daemons(f: &mut Frame, area: Rect, state: &DashboardState, theme: &Theme) {
    let daemon_items: Vec<ListItem> = state
        .daemons
        .lock()
        .unwrap()
        .iter()
        .map(|(d, s)| {
            let color = match s.as_str() {
                "STARTING" => Color::Cyan,
                "CRASHED" | "PANICKED" => Color::Red,
                "STOPPED" => Color::Yellow,
                _ => Color::Green,
            };
            ListItem::new(format!("{}: {}", d, s)).style(Style::default().fg(color))
        })
        .collect();

    let daemons_list = List::new(daemon_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title("🛡️ Supervisor Daemons")
            .border_style(theme.border_style()));

    f.render_widget(daemons_list, area);
}

    // Right side - Resource graphs
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // RAM Gauge
    let ram = *state.ram_mb.lock().unwrap();
    let cpu = *state.cpu_percent.lock().unwrap();
    
    let ram_gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Blue))
        .label(format!("{} MB", ram))
        .ratio(ram as f64 / 16384.0) // 16GB max
        .block(Block::default().borders(Borders::ALL).title("💾 RAM Usage"));

    f.render_widget(ram_gauge, right_chunks[0]);

    // CPU Gauge
    let cpu_gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Green))
        .label(format!("{:.1}%", cpu))
        .ratio(cpu as f64 / 100.0)
        .block(Block::default().borders(Borders::ALL).title("⚡ CPU Usage"));

    f.render_widget(cpu_gauge, right_chunks[1]);
}

fn draw_memory(f: &mut Frame, area: Rect, state: &mut DashboardState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Search bar
    let search_text = if state.memory_search_mode {
        format!("🔍 Search: {}_", state.memory_search_query)
    } else {
        "🔍 Press 'r' to search".to_string()
    };

    let search = Paragraph::new(search_text)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Memory Search"));

    f.render_widget(search, chunks[0]);

    // Memory documents (placeholder)
    let memory_items = vec![ListItem::new("Memory module not loaded - enable in Cargo.toml")];

    let memory_list = List::new(memory_items)
        .block(Block::default().borders(Borders::ALL).title("🧠 Memory"))
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(memory_list, chunks[1], &mut state.memory_scroll);
}

fn draw_graph(f: &mut Frame, area: Rect, state: &DashboardState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let history = state.resource_history.lock().unwrap();
    
    // RAM Sparkline
    let ram_data: Vec<u64> = history.iter().map(|(r, _)| *r).collect();
    let ram_data_rev: Vec<u64> = ram_data.iter().rev().copied().collect();
    let ram_spark = Sparkline::default()
        .block(Block::default().borders(Borders::ALL).title("💾 RAM History"))
        .style(Style::default().fg(Color::Blue))
        .data(&ram_data_rev);

    f.render_widget(ram_spark, chunks[0]);

    // CPU Sparkline
    let cpu_data: Vec<u64> = history.iter().map(|(_, c)| (*c * 10.0) as u64).collect();
    let cpu_data_rev: Vec<u64> = cpu_data.iter().rev().copied().collect();
    let cpu_spark = Sparkline::default()
        .block(Block::default().borders(Borders::ALL).title("⚡ CPU History"))
        .style(Style::default().fg(Color::Green))
        .data(&cpu_data_rev);

    f.render_widget(cpu_spark, chunks[1]);
}

fn draw_logs(f: &mut Frame, area: Rect, state: &mut DashboardState) {
    let logs: Vec<ListItem> = state
        .logs
        .lock()
        .unwrap()
        .iter()
        .skip(state.scroll_offset)
        .map(|l| ListItem::new(l.clone()))
        .collect();

    let logs_list = List::new(logs)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "📜 System Logs (scroll: ↑/↓, offset: {})",
            state.scroll_offset
        )));

    f.render_widget(logs_list, area);
}

fn draw_agents(f: &mut Frame, area: Rect, state: &DashboardState) {
    let agent_items: Vec<ListItem> = state
        .agents_status
        .iter()
        .map(|agent| {
            let color = if agent.available { Color::Green } else { Color::Red };
            let version = agent.version.as_deref().unwrap_or("unknown");
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<15} ", agent.name),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("v{}", version)),
            ]))
        })
        .collect();

    let agents_list = List::new(agent_items)
        .block(Block::default().borders(Borders::ALL).title("🤖 AI Agents Status"));

    f.render_widget(agents_list, area);
}

fn draw_git(f: &mut Frame, area: Rect, state: &DashboardState) {
    let git_info = if let Some(status) = &state.git_status {
        let status_color = if status.is_clean { Color::Green } else { Color::Yellow };
        vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("Branch: {}", status.branch),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("Changes: {} ({}clean)", status.changes, if status.is_clean { "" } else { "not " }),
                Style::default().fg(status_color),
            )),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from("No Git repository initialized"),
            Line::from(""),
            Line::from("Use: git clone <url> or git init"),
        ]
    };

    let git_paragraph = Paragraph::new(git_info)
        .block(Block::default().borders(Borders::ALL).title("🔗 Git Status"))
        .wrap(Wrap { trim: true });

    f.render_widget(git_paragraph, area);
}

fn draw_help(f: &mut Frame, area: Rect, state: &DashboardState) {
    let theme = state.theme_manager.get_theme();
    
    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled("📋 Navigation:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("  [Tab]         - Next tab"),
        Line::from("  [Shift+Tab]   - Previous tab"),
        Line::from("  [Esc]/[q]     - Exit"),
        Line::from("  [t]           - Toggle theme (dark/light)"),
        Line::from(""),
        Line::from(Span::styled("🎨 Theme:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(format!("  Current: {:?}", state.theme_manager.get_theme().theme_type)),
        Line::from(""),
        Line::from(Span::styled("🔍 Search:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("  [r]           - Start memory search"),
        Line::from("  [m]           - Toggle memory panel"),
        Line::from("  [↑]/[↓]       - Scroll in search"),
        Line::from("  [Esc]         - Exit search mode"),
        Line::from(""),
        Line::from(Span::styled("📝 Commands:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("  • scrape <url>           - Web scraping"),
        Line::from("  • git <command>          - Git operations"),
        Line::from("  • goose <task>           - Run Goose agent"),
        Line::from("  • memory:search <query>  - Search memory"),
        Line::from("  • run <cmd>              - Execute in sandbox"),
    ];

    let help = Paragraph::new(help_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .title("❓ Help")
            .border_style(theme.border_style()))
        .wrap(Wrap { trim: true });

    f.render_widget(help, area);
}

fn draw_input(f: &mut Frame, area: Rect, state: &DashboardState) {
    let input_text = if state.memory_search_mode {
        format!("🔍 {}", state.memory_search_query)
    } else {
        format!("❯ {}", state.input_text)
    };
    
    let title = if state.memory_search_mode {
        "Memory Search (Esc to cancel)"
    } else {
        "Command (Enter to send, Esc/q to quit)"
    };

    let input = Paragraph::new(input_text)
        .style(Style::default().fg(if state.memory_search_mode { Color::Yellow } else { Color::Green }))
        .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(input, area);

    if !state.memory_search_mode {
        let input_width = state.input_text.len() as u16 + 2;
        let input_height = 1u16;
        f.set_cursor(area.x + input_width, area.y + input_height);
    }
}

