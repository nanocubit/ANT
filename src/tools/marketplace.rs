#![cfg(feature = "with-wasm")]

//! WASM Skills Marketplace
//! HTTP API для загрузки и управления навыками

use crate::tools::wasm_runtime::{WasmRuntime, SkillManifest};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

/// Информация о навыке для API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub installed: bool,
    pub manifest: Option<SkillManifest>,
}

/// Запрос на установку навыка
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSkillRequest {
    pub name: String,
    pub version: Option<String>,
    pub wasm_url: Option<String>,
}

/// Ответ API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Marketplace сервер
pub struct MarketplaceServer {
    runtime: Arc<Mutex<WasmRuntime>>,
    port: u16,
}

impl MarketplaceServer {
    pub fn new(runtime: Arc<Mutex<WasmRuntime>>, port: u16) -> Self {
        Self { runtime, port }
    }

    /// Запустить HTTP сервер
    pub async fn run(&self) -> Result<()> {
        let runtime = self.runtime.clone();

        // GET /api/skills - список всех навыков
        let list_skills = warp::path!("api" / "skills")
            .and(warp::get())
            .and_then(move || {
                let runtime = runtime.clone();
                async move {
                    let rt = runtime.lock().await;
                    let skills = rt.list_skills();
                    let info: Vec<SkillInfo> = skills.into_iter().map(|s| SkillInfo {
                        id: s.name.clone(),
                        name: s.name,
                        version: s.version,
                        description: s.description,
                        author: s.author,
                        installed: true,
                        manifest: None,
                    }).collect();
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse {
                        success: true,
                        message: "Skills listed successfully".into(),
                        data: Some(serde_json::to_value(info).unwrap()),
                    }))
                }
            });

        // GET /api/skills/:name - информация о навыке
        let get_skill = warp::path!("api" / "skills" / String)
            .and(warp::get())
            .and_then(move |name: String| {
                let runtime = runtime.clone();
                async move {
                    let rt = runtime.lock().await;
                    if let Some(info) = rt.get_skill_info(&name) {
                        Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse {
                            success: true,
                            message: "Skill found".into(),
                            data: Some(serde_json::to_value(info).unwrap()),
                        }))
                    } else {
                        Ok(warp::reply::json(&ApiResponse {
                            success: false,
                            message: "Skill not found".into(),
                            data: None,
                        }))
                    }
                }
            });

        // POST /api/skills/install - установить навык
        let install_skill = warp::path!("api" / "skills" / "install")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |req: InstallSkillRequest| {
                let runtime = runtime.clone();
                async move {
                    let mut rt = runtime.lock().await;
                    
                    // Пытаемся загрузить навык
                    match rt.load_skill(&req.name) {
                        Ok(_) => Ok(warp::reply::json(&ApiResponse {
                            success: true,
                            message: format!("Skill '{}' installed successfully", req.name),
                            data: None,
                        })),
                        Err(e) => Ok(warp::reply::json(&ApiResponse {
                            success: false,
                            message: format!("Failed to install skill: {}", e),
                            data: None,
                        })),
                    }
                }
            });

        // DELETE /api/skills/:name - удалить навык
        let delete_skill = warp::path!("api" / "skills" / String)
            .and(warp::delete())
            .and_then(move |name: String| {
                let runtime = runtime.clone();
                async move {
                    let mut rt = runtime.lock().await;
                    
                    if rt.unload_skill(&name) {
                        Ok(warp::reply::json(&ApiResponse {
                            success: true,
                            message: format!("Skill '{}' removed", name),
                            data: None,
                        }))
                    } else {
                        Ok(warp::reply::json(&ApiResponse {
                            success: false,
                            message: "Skill not found".into(),
                            data: None,
                        }))
                    }
                }
            });

        // POST /api/skills/:name/execute - выполнить навык
        let execute_skill = warp::path!("api" / "skills" / String / "execute")
            .and(warp::post())
            .and(warp::body::bytes())
            .and_then(move |name: String, body: bytes::Bytes| {
                let runtime = runtime.clone();
                async move {
                    let mut rt = runtime.lock().await;
                    
                    match rt.execute_skill(&name, &body) {
                        Ok(output) => Ok(warp::reply::json(&ApiResponse {
                            success: true,
                            message: "Skill executed successfully".into(),
                            data: Some(serde_json::to_value(String::from_utf8_lossy(&output).to_string()).unwrap()),
                        })),
                        Err(e) => Ok(warp::reply::json(&ApiResponse {
                            success: false,
                            message: format!("Execution failed: {}", e),
                            data: None,
                        })),
                    }
                }
            });

        // GET /health - проверка здоровья
        let health = warp::path!("health")
            .and(warp::get())
            .map(|| {
                warp::reply::json(&ApiResponse {
                    success: true,
                    message: "Marketplace server is running".into(),
                    data: None,
                })
            });

        let routes = list_skills
            .or(get_skill)
            .or(install_skill)
            .or(delete_skill)
            .or(execute_skill)
            .or(health);

        println!("🚀 Marketplace server starting on port {}", self.port);
        
        warp::serve(routes)
            .run(([127, 0, 0, 1], self.port))
            .await;

        Ok(())
    }
}

/// Marketplace Actor для интеграции с EventBus
pub struct MarketplaceActor;

impl MarketplaceActor {
    pub async fn run_daemon(
        runtime: Arc<Mutex<WasmRuntime>>,
        port: u16,
    ) -> Result<()> {
        let server = MarketplaceServer::new(runtime, port);
        server.run().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_marketplace_server() {
        // Тест требует запущенный сервер
        let client = reqwest::Client::new();
        let response = client
            .get("http://127.0.0.1:8080/health")
            .send()
            .await
            .unwrap();
        
        assert!(response.status().is_success());
    }
}

