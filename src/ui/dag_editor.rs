//! Интерактивный DAG редактор
//! Поддержка мыши, добавление/удаление узлов, изменение зависимостей

use crate::core::dag::{TaskNode, TaskStatus};
use ratatui::{
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::collections::HashMap;

/// Позиция узла на экране
#[derive(Debug, Clone)]
pub struct DagNodePosition {
    pub id: String,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// Состояние DAG редактора
#[derive(Debug, Clone)]
pub struct DagEditorState {
    pub nodes: HashMap<String, DagNodePosition>,
    pub selected_node: Option<String>,
    pub dragging_node: Option<String>,
    pub drag_offset_x: u16,
    pub drag_offset_y: u16,
    pub adding_dependency: Option<String>, // ID узла от которого тянем связь
    pub zoom: f32,
    pub pan_x: i32,
    pub pan_y: i32,
}

impl DagEditorState {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            selected_node: None,
            dragging_node: None,
            drag_offset_x: 0,
            drag_offset_y: 0,
            adding_dependency: None,
            zoom: 1.0,
            pan_x: 0,
            pan_y: 0,
        }
    }

    /// Инициализировать позиции узлов
    pub fn initialize_positions(&mut self, tasks: &HashMap<String, TaskNode>) {
        let mut x = 5u16;
        let mut y = 3u16;

        for (id, task) in tasks {
            if !self.nodes.contains_key(id) {
                self.nodes.insert(
                    id.clone(),
                    DagNodePosition {
                        id: id.clone(),
                        x,
                        y,
                        width: 30,
                        height: 5,
                    },
                );

                // Смещение для следующего узла
                x += 35;
                if x > 100 {
                    x = 5;
                    y += 10;
                }
            }
        }
    }

    /// Обработка нажатия мыши
    pub fn handle_mouse_press(&mut self, row: u16, col: u16) -> Option<String> {
        // Проверяем клик по узлу
        for (id, pos) in &self.nodes {
            if col >= pos.x && col <= pos.x + pos.width &&
               row >= pos.y && row <= pos.y + pos.height {
                self.selected_node = Some(id.clone());
                self.dragging_node = Some(id.clone());
                self.drag_offset_x = col - pos.x;
                self.drag_offset_y = row - pos.y;
                return Some(id.clone());
            }
        }

        self.selected_node = None;
        self.dragging_node = None;
        None
    }

    /// Обработка перетаскивания
    pub fn handle_mouse_drag(&mut self, row: u16, col: u16) {
        if let Some(ref id) = self.dragging_node {
            if let Some(pos) = self.nodes.get_mut(id) {
                pos.x = col.saturating_sub(self.drag_offset_x);
                pos.y = row.saturating_sub(self.drag_offset_y);
            }
        }
    }

    /// Обработка отпускания мыши
    pub fn handle_mouse_release(&mut self) {
        self.dragging_node = None;
    }

    /// Начать добавление зависимости
    pub fn start_adding_dependency(&mut self, node_id: String) {
        self.adding_dependency = Some(node_id);
    }

    /// Завершить добавление зависимости
    pub fn finish_adding_dependency(&mut self, target_id: String) -> Option<(String, String)> {
        if let Some(ref source_id) = self.adding_dependency {
            if source_id != &target_id {
                let result = (source_id.clone(), target_id);
                self.adding_dependency = None;
                return Some(result);
            }
        }
        self.adding_dependency = None;
        None
    }

    /// Получить узел под курсором
    pub fn get_node_at(&self, row: u16, col: u16) -> Option<&String> {
        for (id, pos) in &self.nodes {
            if col >= pos.x && col <= pos.x + pos.width &&
               row >= pos.y && row <= pos.y + pos.height {
                return Some(id);
            }
        }
        None
    }
}

impl Default for DagEditorState {
    fn default() -> Self {
        Self::new()
    }
}

/// Отрисовка DAG редактора
pub fn draw_dag_editor(
    f: &mut Frame,
    area: Rect,
    tasks: &HashMap<String, TaskNode>,
    statuses: &HashMap<String, TaskStatus>,
    editor_state: &mut DagEditorState,
    theme: &crate::ui::theme::Theme,
) {
    // Инициализация позиций если нужно
    editor_state.initialize_positions(tasks);

    // Рисуем связи между узлами
    for (id, task) in tasks {
        if let Some(pos) = editor_state.nodes.get(id) {
            // Рисуем связи от зависимостей
            for dep_id in &task.depends_on {
                if let Some(dep_pos) = editor_state.nodes.get(dep_id) {
                    draw_connection(
                        f,
                        dep_pos.x + dep_pos.width / 2,
                        dep_pos.y + dep_pos.height,
                        pos.x + pos.width / 2,
                        pos.y,
                        theme.colors.border,
                    );
                }
            }
        }
    }

    // Рисуем узлы
    for (id, task) in tasks {
        if let Some(pos) = editor_state.nodes.get(id) {
            let status_color = match statuses.get(id) {
                Some(TaskStatus::Completed(_)) => theme.colors.success,
                Some(TaskStatus::Failed(_)) => theme.colors.error,
                Some(TaskStatus::Running) => theme.colors.warning,
                _ => theme.colors.primary,
            };

            let is_selected = editor_state.selected_node.as_ref() == Some(id);
            let is_dragging = editor_state.dragging_node.as_ref() == Some(id);
            let is_adding_dep = editor_state.adding_dependency.as_ref() == Some(id);

            let border_color = if is_adding_dep {
                theme.colors.accent
            } else if is_selected || is_dragging {
                theme.colors.tab_active
            } else {
                status_color
            };

            // Заголовок узла
            let title = format!("{} [{}]", id, task.tool);
            
            // Превью input
            let input_preview = if task.input.len() > 25 {
                format!("{}...", &task.input[..25])
            } else {
                task.input.clone()
            };

            let status_text = match statuses.get(id) {
                Some(TaskStatus::Completed(_)) => "✓",
                Some(TaskStatus::Failed(_)) => "✗",
                Some(TaskStatus::Running) => "⟳",
                Some(TaskStatus::Pending) => "○",
                None => "?",
            };

            let content = format!(
                "{} {}\n{}\n{}",
                status_text, title, input_preview,
                if task.depends_on.is_empty() {
                    "Нет зависимостей".to_string()
                } else {
                    format!("Зависит от: {}", task.depends_on.join(", "))
                }
            );

            let mut paragraph = Paragraph::new(content)
                .block(
                    Block::default()
                        .title(format!(" {} ", id))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(border_color))
                )
                .style(Style::default().fg(theme.colors.foreground));

            if is_selected {
                paragraph = paragraph.style(
                    Style::default()
                        .bg(theme.colors.selected)
                        .add_modifier(Modifier::BOLD)
                );
            }

            let node_area = Rect::new(pos.x, pos.y, pos.width, pos.height);
            f.render_widget(paragraph, node_area);
        }
    }

    // Рисуем линию при добавлении зависимости
    if let Some(ref source_id) = editor_state.adding_dependency {
        if let Some(source_pos) = editor_state.nodes.get(source_id) {
            // Будет нарисована при следующем рендере
        }
    }

    // Подсказки
    let hints = vec![
        "ЛКМ: Выбрать/Перетащить",
        "ПКМ: Добавить зависимость",
        "Del: Удалить узел",
        "Esc: Отмена",
    ];

    let hints_paragraph = Paragraph::new(hints.join(" | "))
        .style(Style::default().fg(theme.colors.disabled))
        .block(Block::default().borders(Borders::TOP));

    let hints_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
    f.render_widget(hints_paragraph, hints_area);
}

/// Рисование линии связи между двумя точками
fn draw_connection(f: &mut Frame, x1: u16, y1: u16, x2: u16, y2: u16, color: Color) {
    // Простая реализация через символы
    let dx = (x2 as i32 - x1 as i32).abs();
    let dy = (y2 as i32 - y1 as i32).abs();
    
    let sx = if x1 < x2 { 1 } else { -1 };
    let sy = if y1 < y2 { 1 } else { -1 };
    
    let mut err = (if dx > dy { dx } else { dy }) / 2;
    let mut x = x1 as i32;
    let mut y = y1 as i32;
    
    loop {
        // Проверяем что координаты в пределах экрана
        if x >= 0 && y >= 0 {
            let symbol = if x == x1 as i32 && y == y1 as i32 {
                "●" // Начало
            } else if x == x2 as i32 && y == y2 as i32 {
                "▶" // Конец
            } else {
                "─" // Линия
            };

            let span = Span::styled(symbol, Style::default().fg(color));
            f.render_widget(
                Paragraph::new(Line::from(vec![span.clone()]))
                    .style(Style::default().fg(color)),
                Rect::new(x as u16, y as u16, 1, 1),
            );
        }

        if x == x2 as i32 && y == y2 as i32 {
            break;
        }

        let e2 = err;
        if e2 > dx {
            err -= dy;
            x += sx;
        }
        if e2 < dy {
            err += dx;
            y += sy;
        }
    }
}

/// Команды DAG редактора
#[derive(Debug, Clone)]
pub enum DagEditorCommand {
    SelectNode(String),
    MoveNode(String, u16, u16),
    AddDependency { from: String, to: String },
    RemoveDependency { from: String, to: String },
    DeleteNode(String),
    AddNode(TaskNode),
    None,
}
