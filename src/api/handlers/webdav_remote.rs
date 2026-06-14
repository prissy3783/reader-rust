use crate::api::auth::AuthContext;
use crate::api::handlers::webdav::WebdavPathRequest;
use crate::api::AppState;
use crate::error::error::{ApiResponse, AppError};
use crate::util::time::now_ts;
use axum::{
    extract::{Query, State},
    Json,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::PathBuf;
use tokio::fs;
use zip::read::ZipArchive;
use zip::write::{FileOptions, ZipWriter};

// ==================== 远程 WebDAV 客户端功能 ====================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebdavRemoteConfig {
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct SaveWebdavConfigRequest {
    pub server_url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct WebdavConfigResponse {
    pub server_url: String,
    pub username: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct BackupRequest {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct BackupResult {
    pub file_name: String,
    pub size: u64,
}

#[derive(Debug, Deserialize)]
pub struct RestoreRequest {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct RestoreResult {
    pub restored: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct TestResult {
    pub connected: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RemoteWebdavFileEntry {
    pub name: String,
    pub size: u64,
    pub path: String,
    pub last_modified: i64,
    pub is_directory: bool,
}

// ==================== Legado 备份格式支持 ====================

#[derive(Debug, Deserialize)]
struct LegadoBackup {
    #[serde(default)]
    bookSource: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    bookshelf: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    replaceRule: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    rssSource: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    myBookProgress: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    myBookshelf: Option<Vec<serde_json::Value>>,
}

// ==================== 辅助函数 ====================

fn basic_auth_header(username: &str, password: &str) -> String {
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", username, password))
    )
}

fn storage_dir_for_user(state: &AppState, user_ns: &str) -> PathBuf {
    PathBuf::from(&state.config.storage_dir)
        .join("data")
        .join(user_ns)
}

// ==================== WebDAV 配置管理 ====================

pub async fn save_webdav_config(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<SaveWebdavConfigRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if !req.server_url.starts_with("http://") && !req.server_url.starts_with("https://") {
        return Ok(Json(ApiResponse::err("URL 格式不正确")));
    }
    let user_ns = auth
        .access_token()
        .map(|t| t.to_string())
        .unwrap_or_else(|| "default".to_string());
    let config = WebdavRemoteConfig {
        server_url: req.server_url,
        username: req.username,
        password: req.password,
        enabled: true,
    };
    state.webdav_config.lock().unwrap().insert(
        user_ns.clone(),
        config.clone(),
    );
    let config_dir = PathBuf::from(&state.config.storage_dir)
        .join("data")
        .join(&user_ns);
    fs::create_dir_all(&config_dir)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    let config_path = config_dir.join("webdav_remote_config.json");
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| AppError::Internal(e.into()))?;
    fs::write(&config_path, json)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    Ok(Json(ApiResponse::ok(())))
}

pub async fn get_webdav_config(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<WebdavConfigResponse>>, AppError> {
    let user_ns = auth
        .access_token()
        .map(|t| t.to_string())
        .unwrap_or_else(|| "default".to_string());
    if let Some(c) = state.webdav_config.lock().unwrap().get(&user_ns) {
        return Ok(Json(ApiResponse::ok(WebdavConfigResponse {
            server_url: c.server_url.clone(),
            username: c.username.clone(),
            enabled: c.enabled,
        })));
    }
    let config_path = PathBuf::from(&state.config.storage_dir)
        .join("data")
        .join(&user_ns)
        .join("webdav_remote_config.json");
    if config_path.exists() {
        let data = fs::read(&config_path)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        let config: WebdavRemoteConfig =
            serde_json::from_slice(&data).map_err(|e| AppError::Internal(e.into()))?;
        state.webdav_config.lock().unwrap().insert(user_ns, config.clone());
        return Ok(Json(ApiResponse::ok(WebdavConfigResponse {
            server_url: config.server_url,
            username: config.username,
            enabled: config.enabled,
        })));
    }
    Ok(Json(ApiResponse::ok(WebdavConfigResponse {
        server_url: String::new(),
        username: String::new(),
        enabled: false,
    })))
}

pub async fn test_webdav_connection(
    Json(req): Json<SaveWebdavConfigRequest>,
) -> Result<Json<ApiResponse<TestResult>>, AppError> {
    let client = reqwest::Client::new();
    let auth_header = basic_auth_header(&req.username, &req.password);
    let response = client
        .request(
            reqwest::Method::from_bytes(b"PROPFIND").unwrap(),
            &req.server_url,
        )
        .header("Authorization", auth_header)
        .header("Depth", "0")
        .body("")
        .send()
        .await;
    match response {
        Ok(resp) if resp.status().as_u16() == 207 => {
            Ok(Json(ApiResponse::ok(TestResult {
                connected: true,
                message: "连接成功".to_string(),
            })))
        }
        Ok(resp) => Ok(Json(ApiResponse::ok(TestResult {
            connected: false,
            message: format!("HTTP {}", resp.status()),
        }))),
        Err(e) => Ok(Json(ApiResponse::ok(TestResult {
            connected: false,
            message: format!("连接失败: {}", e),
        }))),
    }
}

// ==================== 备份到远程 WebDAV ====================

pub async fn backup_to_remote_webdav(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<BackupRequest>,
) -> Result<Json<ApiResponse<BackupResult>>, AppError> {
    let user_ns = auth
        .access_token()
        .map(|t| t.to_string())
        .unwrap_or_else(|| "default".to_string());
    let config = state.webdav_config.lock().unwrap().get(&user_ns).cloned().ok_or_else(|| {
        AppError::BadRequest("未配置远程 WebDAV".to_string())
    })?;
    let user_dir = storage_dir_for_user(&state, &user_ns);
    let mut backup_json = serde_json::Map::new();
    if let Ok(data) = fs::read(user_dir.join("bookSource.json")).await {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&data) {
            backup_json.insert("bookSource".to_string(), val);
        }
    }
    if let Ok(data) = fs::read(user_dir.join("bookshelf.json")).await {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&data) {
            backup_json.insert("bookshelf".to_string(), val);
        }
    }
    if let Ok(data) = fs::read(user_dir.join("replaceRule.json")).await {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&data) {
            backup_json.insert("replaceRule".to_string(), val);
        }
    }
    if let Ok(data) = fs::read(user_dir.join("rssSource.json")).await {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&data) {
            backup_json.insert("rssSource".to_string(), val);
        }
    }
    let filename = format!(
        "reader-backup-{}.zip",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    let remote_path = format!("{}/{}", req.path.trim_end_matches('/'), filename);
    let client = reqwest::Client::new();
    let auth_header = basic_auth_header(&config.username, &config.password);
    let backup_value = serde_json::Value::Object(backup_json);
    let json_str = serde_json::to_string_pretty(&backup_value)
        .map_err(|e| AppError::Internal(e.into()))?;
    let mut zip_buf = Vec::new();
    {
        let mut archive = ZipWriter::new(&mut zip_buf);
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        archive
            .start_file("backup.json", options)
            .map_err(|e| AppError::Internal(e.into()))?;
        archive
            .write_all(json_str.as_bytes())
            .map_err(|e| AppError::Internal(e.into()))?;
        archive
            .finish()
            .map_err(|e| AppError::Internal(e.into()))?;
    }
    let response = client
        .put(format!("{}{}", config.server_url, remote_path))
        .header("Authorization", auth_header)
        .header("Content-Type", "application/octet-stream")
        .body(zip_buf.clone())
        .send()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    if !response.status().is_success() {
        return Ok(Json(ApiResponse::err(format!(
            "上传失败: HTTP {}",
            response.status()
        ))));
    }
    Ok(Json(ApiResponse::ok(BackupResult {
        file_name: filename,
        size: zip_buf.len() as u64,
    })))
}

// ==================== 远程文件列表 ====================

pub async fn get_remote_webdav_file_list(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(req): Query<WebdavPathRequest>,
) -> Result<Json<ApiResponse<Vec<RemoteWebdavFileEntry>>>, AppError> {
    let user_ns = auth
        .access_token()
        .map(|t| t.to_string())
        .unwrap_or_else(|| "default".to_string());
    let config = state.webdav_config.lock().unwrap().get(&user_ns).cloned().ok_or_else(|| {
        AppError::BadRequest("未配置远程 WebDAV".to_string())
    })?;
    let client = reqwest::Client::new();
    let auth_header = basic_auth_header(&config.username, &config.password);
    let path = req.path.unwrap_or_else(|| "/".to_string());
    let url = format!("{}{}", config.server_url, path);
    let response = client
        .request(
            reqwest::Method::from_bytes(b"PROPFIND").unwrap(),
            &url,
        )
        .header("Authorization", auth_header)
        .header("Depth", "1")
        .body("")
        .send()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    if response.status().as_u16() != 207 {
        return Ok(Json(ApiResponse::err("PROPFIND 失败")));
    }
    let body = response.text().await.map_err(|e| AppError::Internal(e.into()))?;
    let files = parse_webdav_response(&body, &config.server_url, &path);
    Ok(Json(ApiResponse::ok(files)))
}

// ==================== 从远程 WebDAV 恢复 ====================

pub async fn restore_from_remote_webdav(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<RestoreRequest>,
) -> Result<Json<ApiResponse<RestoreResult>>, AppError> {
    let user_ns = auth
        .access_token()
        .map(|t| t.to_string())
        .unwrap_or_else(|| "default".to_string());
    let config = state.webdav_config.lock().unwrap().get(&user_ns).cloned().ok_or_else(|| {
        AppError::BadRequest("未配置远程 WebDAV".to_string())
    })?;
    let client = reqwest::Client::new();
    let auth_header = basic_auth_header(&config.username, &config.password);
    let url = format!("{}{}", config.server_url, req.path);
    let response = client
        .get(&url)
        .header("Authorization", auth_header)
        .send()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    if !response.status().is_success() {
        return Ok(Json(ApiResponse::err(format!(
            "下载失败: HTTP {}",
            response.status()
        ))));
    }
    let zip_data = response.bytes().await.map_err(|e| AppError::Internal(e.into()))?;
    let mut archive =
        ZipArchive::new(std::io::Cursor::new(zip_data)).map_err(|e| AppError::Internal(e.into()))?;
    let mut backup_json_str = String::new();
    let mut found = false;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| AppError::Internal(e.into()))?;
        if file.name() == "backup.json" || file.name().ends_with(".json") {
            std::io::Read::read_to_string(&mut file, &mut backup_json_str)
                .map_err(|e| AppError::Internal(e.into()))?;
            found = true;
            break;
        }
    }
    if !found {
        return Ok(Json(ApiResponse::err("备份文件中未找到数据文件")));
    }
    let backup: LegadoBackup =
        serde_json::from_str(&backup_json_str).map_err(|e| AppError::Internal(e.into()))?;
    let user_dir = storage_dir_for_user(&state, &user_ns);
    fs::create_dir_all(&user_dir)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    if let Some(sources) = &backup.bookSource {
        if !sources.is_empty() {
            let data = serde_json::to_string_pretty(sources)
                .map_err(|e| AppError::Internal(e.into()))?;
            fs::write(user_dir.join("bookSource.json"), data)
                .await
                .map_err(|e| AppError::Internal(e.into()))?;
        }
    }
    if let Some(shelf) = &backup.bookshelf {
        if !shelf.is_empty() {
            let data = serde_json::to_string_pretty(shelf)
                .map_err(|e| AppError::Internal(e.into()))?;
            fs::write(user_dir.join("bookshelf.json"), data)
                .await
                .map_err(|e| AppError::Internal(e.into()))?;
        }
    }
    if let Some(shelf) = &backup.myBookshelf {
        if !shelf.is_empty() {
            let data = serde_json::to_string_pretty(shelf)
                .map_err(|e| AppError::Internal(e.into()))?;
            fs::write(user_dir.join("bookshelf.json"), data)
                .await
                .map_err(|e| AppError::Internal(e.into()))?;
        }
    }
    if let Some(rules) = &backup.replaceRule {
        if !rules.is_empty() {
            let data = serde_json::to_string_pretty(rules)
                .map_err(|e| AppError::Internal(e.into()))?;
            fs::write(user_dir.join("replaceRule.json"), data)
                .await
                .map_err(|e| AppError::Internal(e.into()))?;
        }
    }
    if let Some(rss) = &backup.rssSource {
        if !rss.is_empty() {
            let data = serde_json::to_string_pretty(rss)
                .map_err(|e| AppError::Internal(e.into()))?;
            fs::write(user_dir.join("rssSource.json"), data)
                .await
                .map_err(|e| AppError::Internal(e.into()))?;
        }
    }
    Ok(Json(ApiResponse::ok(RestoreResult {
        restored: true,
        message: "恢复完成".to_string(),
    })))
}

// ==================== XML 解析 ====================

fn parse_webdav_response(
    xml: &str,
    server_url: &str,
    base_path: &str,
) -> Vec<RemoteWebdavFileEntry> {
    let mut files = Vec::new();
    for response in xml.split("<D:response>") {
        if !response.contains("<D:href>") {
            continue;
        }
        let href = response
            .split("<D:href>")
            .nth(1)
            .and_then(|s| s.split("</D:href>").next())
            .unwrap_or("");
        if href.is_empty() || href == base_path || href == format!("{}/", base_path) {
            continue;
        }
        let name = href.split('/').last().unwrap_or("");
        let is_dir = response.contains("<D:collection />");
        let size_str = response
            .split("<D:getcontentlength>")
            .nth(1)
            .and_then(|s| s.split("</D:getcontentlength>").next())
            .unwrap_or("0");
        let size: u64 = size_str.parse().unwrap_or(0);
        let modified_str = response
            .split("<D:getlastmodified>")
            .nth(1)
            .and_then(|s| s.split("</D:getlastmodified>").next())
            .unwrap_or("");
        let last_modified: i64 = if !modified_str.is_empty() {
            chrono::DateTime::parse_from_rfc2822(modified_str)
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(now_ts())
        } else {
            now_ts()
        };
        files.push(RemoteWebdavFileEntry {
            name: name.to_string(),
            size,
            path: href.to_string(),
            last_modified,
            is_directory: is_dir,
        });
    }
    files
}
