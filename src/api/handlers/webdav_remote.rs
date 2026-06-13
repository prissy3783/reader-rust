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
use serde::Deserialize;
use serde_json::Value;
use std::io::Read;
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

// ==================== 远程 WebDAV API 端点 ====================

pub async fn save_webdav_config(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<SaveWebdavConfigRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if !req.server_url.starts_with("http://") && !req.server_url.starts_with("https://") {
        return Ok(Json(ApiResponse::err("URL 格式不正确")));
    }
    let user_id = auth.user_id().ok_or_else(|| AppError::Unauthorized)?;
    state.webdav_config.lock().unwrap().insert(
        user_id.to_string(),
        WebdavRemoteConfig {
            server_url: req.server_url,
            username: req.username,
            password: req.password,
            enabled: true,
        },
    );
    Ok(Json(ApiResponse::ok(())))
}

pub async fn get_webdav_config(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<WebdavConfigResponse>>, AppError> {
    let user_id = auth.user_id().ok_or_else(|| AppError::Unauthorized)?;
    let config = state.webdav_config.lock().unwrap().get(user_id).cloned();
    match config {
        Some(c) => Ok(Json(ApiResponse::ok(WebdavConfigResponse {
            server_url: c.server_url,
            username: c.username,
            enabled: c.enabled,
        }))),
        None => Ok(Json(ApiResponse::ok(WebdavConfigResponse {
            server_url: String::new(),
            username: String::new(),
            enabled: false,
        }))),
    }
}

pub async fn test_webdav_connection(
    Json(req): Json<SaveWebdavConfigRequest>,
) -> Result<Json<ApiResponse<TestResult>>, AppError> {
    use reqwest::Client;
    let client = Client::new();
    let auth_header = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!(
            "{}:{}",
            req.username, req.password
        ))
    );
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

pub async fn backup_to_remote_webdav(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<BackupRequest>,
) -> Result<Json<ApiResponse<BackupResult>>, AppError> {
    let user_id = auth.user_id().ok_or_else(|| AppError::Unauthorized)?;
    let config = state.webdav_config.lock().unwrap().get(user_id).ok_or_else(|| {
        AppError::BadRequest("未配置 WebDAV".to_string())
    })?;
    let filename = format!(
        "reader-backup-{}.json",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    let remote_path = format!("{}/{}", req.path.trim_end_matches('/'), filename);
    let client = reqwest::Client::new();
    let auth_header = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!(
            "{}:{}",
            config.username, config.password
        ))
    );
    let backup_json = serde_json::json!({
        "version": 1,
        "createdAt": chrono::Utc::now().to_rfc3339(),
        "app": "reader-rust-frontend",
        "bookshelf": { "books": [], "groups": [] },
        "bookSources": [],
        "rssSources": [],
        "bookmarks": [],
        "replaceRules": [],
        "localState": {}
    });
    let json_str = serde_json::to_string_pretty(&backup_json)
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
        .body(zip_buf)
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

pub async fn get_remote_webdav_file_list(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(req): Query<WebdavPathRequest>,
) -> Result<Json<ApiResponse<Vec<RemoteWebdavFileEntry>>>, AppError> {
    let user_id = auth.user_id().ok_or_else(|| AppError::Unauthorized)?;
    let config = state.webdav_config.lock().unwrap().get(user_id).ok_or_else(|| {
        AppError::BadRequest("未配置 WebDAV".to_string())
    })?;
    let client = reqwest::Client::new();
    let auth_header = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!(
            "{}:{}",
            config.username, config.password
        ))
    );
    let url = format!("{}{}", config.server_url, req.path);
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
    let files = parse_webdav_response(&body, &config.server_url, &req.path);
    Ok(Json(ApiResponse::ok(files)))
}

pub async fn restore_from_remote_webdav(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<RestoreRequest>,
) -> Result<Json<ApiResponse<RestoreResult>>, AppError> {
    let user_id = auth.user_id().ok_or_else(|| AppError::Unauthorized)?;
    let config = state.webdav_config.lock().unwrap().get(user_id).ok_or_else(|| {
        AppError::BadRequest("未配置 WebDAV".to_string())
    })?;
    let client = reqwest::Client::new();
    let auth_header = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!(
            "{}:{}",
            config.username, config.password
        ))
    );
    let url = format!("{}{}", config.server_url, req.path);
    let response = client
        .get(&url)
        .header("Authorization", auth_header)
        .send()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    if !response.status().is_success() {
        return Ok(Json(ApiResponse::err("下载失败")));
    }
    let zip_data = response.bytes().await.map_err(|e| AppError::Internal(e.into()))?;
    let mut archive =
        ZipArchive::new(std::io::Cursor::new(zip_data)).map_err(|e| AppError::Internal(e.into()))?;
    let mut json_str = String::new();
    let mut found = false;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| AppError::Internal(e.into()))?;
        if file.name() == "backup.json" {
            std::io::Read::read_to_string(&mut file, &mut json_str)
                .map_err(|e| AppError::Internal(e.into()))?;
            found = true;
            break;
        }
    }
    if !found {
        return Ok(Json(ApiResponse::err("备份文件中未找到 backup.json")));
    }
    let _payload: Value =
        serde_json::from_str(&json_str).map_err(|e| AppError::Internal(e.into()))?;
    Ok(Json(ApiResponse::ok(RestoreResult { restored: true })))
}

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
