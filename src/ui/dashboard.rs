use crate::bus::{EventBus, SystemEvent};
use crate::ui::theme::{Theme, ThemeManager, ThemeType};
use crate::ui::dag_editor::{DagEditorState, DagEditorCommand, draw_dag_editor};
use crate::ui::command_palette::{CommandPalette, CommandAction};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyModifiers, MouseEventKind, MouseButton},
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
    pub resource_history: Arc<Mutex<Vec<(u64, f32)>>>,
    pub show_memory_panel: bool,
    pub memory_search_mode: bool,
    pub theme_manager: ThemeManager,
    pub dag_editor_state: DagEditorState,
    pub command_palette: CommandPalette,
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
            chat_history: vec!["🦀 ANT OS v0.9.0: Система готова. Введите команду.".to_string()],
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
            command_palette: CommandPalette::new(),
        }
    }
}

impl Default for DashboardState {
    fn default() -> Self {
        Self::new()
    }
}

/// Обработка действий команд палитры
fn handle_command_action(action: CommandAction, bus: &Arc<EventBus>, state: &mut DashboardState) {
    match action {
        CommandAction::SubmitGoal { task } => {
            state.chat_history.push(format!("⚡ Submit: {}", task));
            bus.emit(SystemEvent::GoalCreated {
                id: format!("G-{}", chrono::Local::now().format("%H%M%S")),
                task,
            });
        }
        CommandAction::KillGoal { goal_id } => {
            state.chat_history.push(format!("🛑 Kill: {}", goal_id));
            bus.emit(SystemEvent::GoalFailed {
                id: goal_id,
                reason: "Killed by user".to_string(),
            });
        }
        CommandAction::RetryTask { task_id } => {
            state.chat_history.push(format!("🔄 Retry: {}", task_id));
            if let Some(task) = state.dags.lock().unwrap().get_mut(&task_id) {
                task[1] = "PENDING".to_string();
                bus.emit(SystemEvent::TaskDispatched {
                    task_id: task_id.clone(),
                    tool: task[0].clone(),
                    input: task[2].clone(),
                });
            }
        }
        CommandAction::ShowTaskLogs { task_id } => {
            state.chat_history.push(format!("📜 Logs for: {}", task_id));
            state.active_tab = Tab::Logs;
            state.scroll_offset = 0;
        }
        CommandAction::ToggleTheme => {
            state.theme_manager.toggle();
            state.chat_history.push(format!("🎨 Theme: {:?}", state.theme_manager.get_theme().theme_type));
        }
        CommandAction::ShowHelp => {
            state.active_tab = Tab::Help;
        }
        CommandAction::Quit => {
            state.chat_history.push("👋 Goodbye!".to_string());
            bus.emit(SystemEvent::SystemBoot("Shutdown requested".to_string()));
        }
        CommandAction::SwitchTab { tab } => {
            let tabs = Tab::all();
            if tab < tabs.len() {
                state.active_tab = tabs[tab];
                state.chat_history.push(format!("📑 Tab: {:?}", tabs[tab]));
            }
        }
        CommandAction::ClearLogs => {
            state.logs.lock().unwrap().clear();
            state.chat_history.push("🗑️ Logs cleared".to_string());
        }
        CommandAction::ShowStatus => {
            let goals = state.goals.lock().unwrap();
            let active = goals.iter().filter(|g| g.status != "COMPLETED" && g.status != "FAILED").count();
            let completed = goals.iter().filter(|g| g.status == "COMPLETED").count();
            let failed = goals.iter().filter(|g| g.status == "FAILED").count();
            state.chat_history.push(format!(
                "📊 Status: Active: {}, Completed: {}, Failed: {}",
                active, completed, failed
            ));
        }
        CommandAction::RestartDaemon { daemon } => {
            state.chat_history.push(format!("🔄 Restart daemon: {}", daemon));
            bus.emit(SystemEvent::DaemonStatus {
                daemon: daemon.clone(),
                status: "RESTARTING".to_string(),
            });
        }
        CommandAction::SplitHorizontal { tab } => {
            let tabs = Tab::all();
            if tab < tabs.len() {
                if state.split_root.split_focused(crate::ui::dashboard::SplitDirection::Horizontal, tabs[tab]) {
                    state.chat_history.push(format!("⬅️➡️ Split horizontal: {:?}", tabs[tab]));
                }
            }
        }
        CommandAction::SplitVertical { tab } => {
            let tabs = Tab::all();
            if tab < tabs.len() {
                if state.split_root.split_focused(crate::ui::dashboard::SplitDirection::Vertical, tabs[tab]) {
                    state.chat_history.push(format!("⬆️⬇️ Split vertical: {:?}", tabs[tab]));
                }
            }
        }
        CommandAction::ClosePane => {
            if state.split_root.close_focused() {
                state.chat_history.push("❌ Pane closed".to_string());
            }
        }
        CommandAction::FocusNextPane => {
            state.split_root.focus_next();
            state.chat_history.push("➡️ Next pane".to_string());
        }
        CommandAction::FocusPrevPane => {
            state.split_root.focus_prev();
            state.chat_history.push("⬅️ Prev pane".to_string());
        }
        CommandAction::EqualizePanes => {
            fn equalize(node: &mut crate::ui::dashboard::SplitNode) {
                if let crate::ui::dashboard::SplitNode::Split { children, sizes, .. } = node {
                    let size = 100 / children.len() as u16;
                    *sizes = vec![size; children.len()];
                    for child in children {
                        equalize(child);
                    }
                }
            }
            equalize(&mut state.split_root);
            state.chat_history.push("⚖️ Panes equalized".to_string());
        }
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
                            state.theme_manager.toggle();
                        }
                    }
                    KeyCode::Char('r') => {
                        if !state.command_palette.visible {
                            state.memory_search_mode = true;
                        }
                    }
                    KeyCode::Char('d') if state.input_text.is_empty() && !state.command_palette.visible => {
                        state.active_tab = Tab::Dashboard;
                    }
                    KeyCode::Delete if state.input_text.is_empty() && !state.command_palette.visible => {
                        if let Some(ref node_id) = state.dag_editor_state.selected_node {
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

    draw_status_bar(f, main_chunks[0], state);
    draw_tabs(f, main_chunks[1], state.active_tab, state.theme_manager.get_theme());
    
    match state.active_tab {
        Tab::Dashboard => draw_dashboard(f, main_chunks[2], state),
        Tab::Memory => draw_memory(f, main_chunks[2], state),
        Tab::Graph => draw_graph(f, main_chunks[2], state),
        Tab::Logs => draw_logs(f, main_chunks[2], state),
        Tab::Agents => draw_agents(f, main_chunks[2], state),
        Tab::Git => draw_git(f, main_chunks[2], state),
        Tab::Help => draw_help(f, main_chunks[2], state),
    }
    
    draw_input(f, main_chunks[3], state);
    
    if state.command_palette.visible {
        state.command_palette.draw(f, area, state.theme_manager.get_theme());
    }
}

fn draw_status_bar(f: &mut Frame, area: Rect, state: &DashboardState) {
    let theme = state.theme_manager.get_theme();
    
    let ram = *state.ram_mb.lock().unwrap();
    let cpu = *state.cpu_percent.lock().unwrap();
    
    let daemons = state.daemons.lock().unwrap();
    let running = daemons.values().filter(|s| *s == "RUNNING").count();
    let total = daemons.len();
    
    let goals = state.goals.lock().unwrap();
    let active_goals = goals.iter().filter(|g| g.status != "COMPLETED" && g.status != "FAILED").count();
    
    let status = Line::from(vec![
        Span::styled("🦀 ANT OS v0.9.0", Style::default().fg(theme.colors.accent).add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled(format!("💾 RAM: {} MB", ram), Style::default().fg(Color::Blue)),
        Span::raw(" | "),
        Span::styled(format!("⚡ CPU: {:.1}%", cpu), Style::default().fg(Color::Green)),
        Span::raw(" | "),
        Span::styled(format!("🛡️ Daemons: {}/{}", running, total), Style::default().fg(Color::Cyan)),
        Span::raw(" | "),
        Span::styled(format!("🎯 Goals: {}", active_goals), Style::default().fg(Color::Yellow)),
    ]);
    
    let bar = Paragraph::new(status)
        .block(Block::default().borders(Borders::ALL).border_style(theme.border_style()));
    
    f.render_widget(bar, area);
}

fn draw_tabs(f: &mut Frame, area: Rect, active: Tab, theme: &Theme) {
    let titles = Tab::titles();
    let tabs = Tabs::new(titles)
        .select(active as usize)
        .block(Block::default().borders(Borders::ALL).border_style(theme.border_style()))
        .style(Style::default().fg(theme.colors.foreground))
        .highlight_style(Style::default().fg(theme.colors.accent).add_modifier(Modifier::BOLD))
        .divider(Span::raw(" | "));
    
    f.render_widget(tabs, area);
}

fn draw_dashboard(f: &mut Frame, area: Rect, state: &mut DashboardState) {
    let theme = state.theme_manager.get_theme();
    
    let dags_guard = state.dags.lock().unwrap();
    
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
            depends_on: vec![],
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
    
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    draw_dag_editor(f, chunks[0], &tasks, &statuses, &mut state.dag_editor_state, theme);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let ram = *state.ram_mb.lock().unwrap();
    let cpu = *state.cpu_percent.lock().unwrap();
    
    let ram_gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Blue))
        .label(format!("{} MB", ram))
        .ratio(ram as f64 / 16384.0)
        .block(Block::default().borders(Borders::ALL).title("💾 RAM Usage"));

    f.render_widget(ram_gauge, right_chunks[0]);

    let cpu_gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Green))
        .label(format!("{:.1}%", cpu))
        .ratio(cpu as f64 / 100.0)
        .block(Block::default().borders(Borders::ALL).title("⚡ CPU Usage"));

    f.render_widget(cpu_gauge, right_chunks[1]);
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

fn draw_memory(f: &mut Frame, area: Rect, state: &mut DashboardState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let search_text = if state.memory_search_mode {
        format!("🔍 Search: {}_", state.memory_search_query)
    } else {
        "🔍 Press 'r' to search".to_string()
    };
    
    let search = Paragraph::new(search_text)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Memory Search"));
    
    f.render_widget(search, chunks[0]);
    
    let items: Vec<ListItem> = state
        .logs
        .lock()
        .unwrap()
        .iter()
        .skip(state.scroll_offset)
        .map(|l| ListItem::new(l.clone()))
        .collect();
    
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Memory/Logs"));
    
    f.render_widget(list, chunks[1]);
}

fn draw_graph(f: &mut Frame, area: Rect, state: &mut DashboardState) {
    let theme = state.theme_manager.get_theme();
    let history = state.resource_history.lock().unwrap();
    
    let ram_data: Vec<u64> = history.iter().map(|(r, _)| *r).collect();
    let cpu_data: Vec<f32> = history.iter().map(|(_, c)| *c).collect();
    
    let ram_spark = Sparkline::default()
        .data(&ram_data)
        .style(Style::default().fg(Color::Blue))
        .block(Block::default().borders(Borders::ALL).title("RAM History (MB)").border_style(theme.border_style()));
    
    let cpu_spark = Sparkline::default()
        .data(&cpu_data)
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL).title("CPU History (%)").border_style(theme.border_style()));
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    
    f.render_widget(ram_spark, chunks[0]);
    f.render_widget(cpu_spark, chunks[1]);
}

fn draw_logs(f: &mut Frame, area: Rect, state: &mut DashboardState) {
    let logs: Vec<ListItem> = state
        .logs
        .lock()
        .unwrap()
        .iter()
        .rev()
        .skip(state.scroll_offset)
        .map(|l| ListItem::new(l.clone()))
        .collect();
    
    let list = List::new(logs)
        .block(Block::default().borders(Borders::ALL).title("📜 System Logs"));
    
    f.render_widget(list, area);
}

fn draw_agents(f: &mut Frame, area: Rect, state: &mut DashboardState) {
    let theme = state.theme_manager.get_theme();
    
    let items: Vec<ListItem> = state
        .agents_status
        .iter()
        .map(|a| {
            let status = if a.available { "✅" } else { "❌" };
            let version = a.version.as_deref().unwrap_or("unknown");
            ListItem::new(format!("{} {} ({})", status, a.name, version))
        })
        .collect();
    
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("🤖 Agents").border_style(theme.border_style()));
    
    f.render_widget(list, area);
}

fn draw_git(f: &mut Frame, area: Rect, state: &mut DashboardState) {
    let theme = state.theme_manager.get_theme();
    
    if let Some(git) = &state.git_status {
        let text = vec![
            Line::from(vec![Span::styled("Branch: ", Style::default().fg(theme.colors.accent)), Span::raw(&git.branch)]),
            Line::from(vec![Span::styled("Changes: ", Style::default().fg(theme.colors.accent)), Span::raw(git.changes.to_string())]),
            Line::from(vec![Span::styled("Status: ", Style::default().fg(theme.colors.accent)), Span::raw(if git.is_clean { "Clean" } else { "Dirty" })]),
        ];
        
        let para = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("🔗 Git Status").border_style(theme.border_style()));
        
        f.render_widget(para, area);
    } else {
        let para = Paragraph::new("Git not available")
            .block(Block::default().borders(Borders::ALL).title("🔗 Git Status").border_style(theme.border_style()));
        
        f.render_widget(para, area);
    }
}

fn draw_help(f: &mut Frame, area: Rect, state: &DashboardState) {
    let theme = state.theme_manager.get_theme();
    
    let help = Paragraph::new(vec![
        Line::from(vec![Span::styled("ANT OS v0.9.0 - Help", Style::default().fg(theme.colors.accent).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled("Navigation:", Style::default().fg(theme.colors.accent))]),
        Line::from("  Tab / Shift+Tab  - Switch tabs"),
        Line::from("  :                - Command palette"),
        Line::from("  ↑/↓              - Scroll / Navigate palette"),
        Line::from("  Enter            - Execute command / Send input"),
        Line::from("  Esc              - Close palette / Exit search"),
        Line::from(""),
        Line::from(vec![Span::styled("Commands:", Style::default().fg(theme.colors.accent))]),
        Line::from("  :submit \"task\"   - Submit new goal"),
        Line::from("  :kill <id>       - Kill goal/task"),
        Line::from("  :retry <id>      - Retry failed task"),
        Line::from("  :logs <id>       - Show task logs"),
        Line::from("  :split [tab]     - Horizontal split"),
        Line::from("  :vsplit [tab]    - Vertical split"),
        Line::from("  :close-pane      - Close current pane"),
        Line::from("  :next-pane       - Focus next pane"),
        Line::from("  :theme           - Toggle theme"),
        Line::from("  :help            - This help"),
        Line::from("  :quit            - Exit ANT OS"),
        Line::from(""),
        Line::from(vec![Span::styled("Shortcuts:", Style::default().fg(theme.colors.accent))]),
        Line::from("  m                - Toggle memory panel"),
        Line::from("  r                - Memory search mode"),
        Line::from("  t                - Toggle theme"),
        Line::from("  d                - Dashboard tab"),
        Line::from(""),
        Line::from(vec![Span::styled("Tabs:", Style::default().fg(theme.colors.accent))]),
        Line::from("  1 📊 Dashboard   - DAG editor + resource graphs"),
        Line::from("  2 🧠 Memory      - Search & view memory"),
        Line::from("  3 📈 Graph       - Resource sparklines"),
        Line::from("  4 📜 Logs        - System logs"),
        Line::from("  5 🤖 Agents      - External agent status"),
        Line::from("  6 🔗 Git         - Git status"),
        Line::from("  7 ❓ Help        - This help"),
    ])
    .block(Block::default().borders(Borders::ALL).title("❓ Help").border_style(theme.border_style()))
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