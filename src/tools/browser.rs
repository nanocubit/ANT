#![cfg(feature = "with-browser")]

use anyhow::{Context, Result};
use headless_chrome::{Browser, protocol::cdp::Page, types::PrintToPdfOptions};
use std::time::Duration;

/// Результат скрапинга веб-страницы
#[derive(Debug, Clone)]
pub struct ScrapedContent {
    pub url: String,
    pub title: String,
    pub text: String,
    pub html: String,
    pub screenshot: Option<Vec<u8>>,
    pub links: Vec<String>,
}

/// Headless браузер для агентов
pub struct HeadlessBrowser {
    browser: Browser,
}

impl HeadlessBrowser {
    /// Инициализация браузера
    pub fn new() -> Result<Self> {
        let browser = Browser::new(
            headless_chrome::LaunchOptions {
                headless: true,
                sandbox: true,
                enable_logging: false,
                ..Default::default()
            }
        )?;

        Ok(Self { browser })
    }

    /// Скрапинг страницы с извлечением текста
    pub fn scrape(&self, url: &str) -> Result<ScrapedContent> {
        let tab = self.browser.new_tab()?;

        // Устанавливаем таймаут
        tab.set_default_timeout(Duration::from_secs(30));

        // Переходим на страницу и ждем загрузки
        tab.navigate_to(url)?;
        tab.wait_until_navigated()?;

        // Получаем заголовок
        let title = tab.get_title().unwrap_or_default();

        // Получаем HTML
        let html = tab
            .get_content()
            .context("Failed to get page content")?;

        // Извлекаем текст (удаляя HTML теги)
        let text = self.extract_text(&html);

        // Извлекаем ссылки
        let links = self.extract_links(&tab)?;

        // Делаем скриншот (опционально)
        let screenshot = tab
            .capture_screenshot(Page::CaptureScreenshotFormat::Png, None, None, true)
            .ok();

        Ok(ScrapedContent {
            url: url.to_string(),
            title,
            text,
            html,
            screenshot,
            links,
        })
    }

    /// Скрапинг с автоскроллом (для длинных страниц)
    pub fn scrape_with_scroll(&self, url: &str) -> Result<ScrapedContent> {
        let tab = self.browser.new_tab()?;
        tab.set_default_timeout(Duration::from_secs(60));

        tab.navigate_to(url)?;
        tab.wait_until_navigated()?;

        // Автоскролл вниз
        self.scroll_to_bottom(&tab)?;

        let title = tab.get_title().unwrap_or_default();
        let html = tab.get_content().context("Failed to get content")?;
        let text = self.extract_text(&html);
        let links = self.extract_links(&tab)?;

        Ok(ScrapedContent {
            url: url.to_string(),
            title,
            text,
            html,
            screenshot: None,
            links,
        })
    }

    /// Сохранение страницы в PDF
    pub fn save_pdf(&self, url: &str, output_path: &str) -> Result<()> {
        let tab = self.browser.new_tab()?;
        tab.navigate_to(url)?;
        tab.wait_until_navigated()?;

        let pdf_data = tab.print_to_pdf(Some(PrintToPdfOptions {
            paper_width: Some(8.5),
            paper_height: Some(11.0),
            margin_top: Some(0.5),
            margin_bottom: Some(0.5),
            margin_left: Some(0.5),
            margin_right: Some(0.5),
            print_background: Some(true),
            ..Default::default()
        }))?;

        std::fs::write(output_path, pdf_data)?;
        Ok(())
    }

    /// Выполнение JavaScript на странице
    pub fn evaluate_js(&self, url: &str, js_code: &str) -> Result<serde_json::Value> {
        let tab = self.browser.new_tab()?;
        tab.navigate_to(url)?;
        tab.wait_until_navigated()?;

        let result = tab.evaluate(js_code, false)?;
        Ok(result)
    }

    /// Клик по элементу
    pub fn click_element(&self, url: &str, selector: &str) -> Result<()> {
        let tab = self.browser.new_tab()?;
        tab.navigate_to(url)?;
        tab.wait_until_navigated()?;

        let element = tab.wait_for_element(selector)?;
        element.click()?;

        Ok(())
    }

    /// Ввод текста в поле
    pub fn type_text(&self, url: &str, selector: &str, text: &str) -> Result<()> {
        let tab = self.browser.new_tab()?;
        tab.navigate_to(url)?;
        tab.wait_until_navigated()?;

        let element = tab.wait_for_element(selector)?;
        element.type_str(text)?;
        element.press_enter()?;

        Ok(())
    }

    // Приватные методы

    fn extract_text(&self, html: &str) -> String {
        // Простая экстракция текста без тегов
        let mut text = html.to_string();

        // Удаляем скрипты и стили
        text = regex_replace(&text, r"(?s)<script[^>]*>.*?</script>", "");
        text = regex_replace(&text, r"(?s)<style[^>]*>.*?</style>", "");

        // Удаляем HTML теги
        text = regex_replace(&text, r"<[^>]*>", " ");

        // Декодируем HTML entities
        text = html_escape(&text);

        // Нормализуем пробелы
        let mut result = String::new();
        let mut last_was_space = false;
        for c in text.chars() {
            if c.is_whitespace() {
                if !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            } else {
                result.push(c);
                last_was_space = false;
            }
        }

        text.trim().to_string()
    }

    fn extract_links(&self, tab: &headless_chrome::Tab) -> Result<Vec<String>> {
        let js = r#"
            Array.from(document.querySelectorAll('a[href]'))
                .map(a => a.href)
                .filter(href => href.startsWith('http'))
        "#;

        let result = tab.evaluate(js, false)?;
        let links: Vec<String> = serde_json::from_value(result).unwrap_or_default();
        Ok(links)
    }

    fn scroll_to_bottom(&self, tab: &headless_chrome::Tab) -> Result<()> {
        let js = r#"
            new Promise((resolve) => {
                let totalHeight = document.body.scrollHeight;
                let scrollStep = 500;
                let scrollInterval = 100;
                let currentPosition = 0;

                const scroll = () => {
                    if (currentPosition < totalHeight) {
                        window.scrollBy(0, scrollStep);
                        currentPosition += scrollStep;
                        setTimeout(scroll, scrollInterval);
                    } else {
                        resolve();
                    }
                };

                scroll();
            })
        "#;

        tab.evaluate(js, true)?;
        std::thread::sleep(Duration::from_secs(2)); // Ждем завершения скролла

        Ok(())
    }
}

// Вспомогательные функции

fn regex_replace(text: &str, pattern: &str, replacement: &str) -> String {
    let re = regex::Regex::new(pattern).unwrap();
    re.replace_all(text, replacement).to_string()
}

fn html_escape(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

impl Default for HeadlessBrowser {
    fn default() -> Self {
        Self::new().expect("Failed to initialize browser")
    }
}

// Actor для интеграции с EventBus
use crate::bus::{EventBus, SystemEvent};
use std::sync::Arc;
use tokio::time::{sleep, Duration as TokioDuration};

pub struct BrowserActor;

impl BrowserActor {
    pub async fn run_daemon(bus: Arc<EventBus>) -> Result<()> {
        let mut rx = bus.subscribe();

        // Инициализируем браузер один раз
        let browser = match HeadlessBrowser::new() {
            Ok(b) => b,
            Err(e) => {
                bus.emit(SystemEvent::Log {
                    level: "ERROR".into(),
                    source: "Browser".into(),
                    message: format!("Failed to init browser: {}", e),
                });
                return Ok(());
            }
        };

        bus.emit(SystemEvent::Log {
            level: "INFO".into(),
            source: "Browser".into(),
            message: "Headless Chrome initialized".into(),
        });

        while let Ok(SystemEvent::TaskDispatched { task_id, tool, input }) = rx.recv().await {
            if tool == "browser" || tool.starts_with("scrape:") {
                let bus_clone = bus.clone();
                let browser_ref = &browser;
                let url = input.clone();
                let task_id_clone = task_id.clone();

                tokio::spawn(async move {
                    bus_clone.emit(SystemEvent::Log {
                        level: "INFO".into(),
                        source: "Browser".into(),
                        message: format!("Scraping: {}", url),
                    });

                    match browser_ref.scrape(&url) {
                        Ok(content) => {
                            bus_clone.emit(SystemEvent::Log {
                                level: "INFO".into(),
                                source: "Browser".into(),
                                message: format!(
                                    "Scraped {} bytes from {}",
                                    content.text.len(),
                                    url
                                ),
                            });

                            bus_clone.emit(SystemEvent::TaskCompleted {
                                task_id: task_id_clone,
                                result: format!(
                                    "Title: {}\n\nContent:\n{}",
                                    content.title,
                                    content.text.chars().take(2000).collect::<String>()
                                ),
                            });
                        }
                        Err(e) => {
                            bus_clone.emit(SystemEvent::TaskFailed {
                                task_id: task_id_clone,
                                error: e.to_string(),
                            });
                        }
                    }
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Требуется установленный Chrome
    fn test_browser_scrape() {
        let browser = HeadlessBrowser::new().unwrap();
        let result = browser.scrape("https://www.rust-lang.org").unwrap();

        assert!(!result.title.is_empty());
        assert!(!result.text.is_empty());
        println!("Title: {}", result.title);
        println!("Text preview: {}", &result.text[..200.min(result.text.len())]);
    }
}

