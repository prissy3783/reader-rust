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
use std::io::{Cursor, Write};
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
#[allow(dead_code)]
struct LegadoBackup {
    #[serde(default)]
    book_source: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    bookshelf: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    replace_rule: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    rss_source: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    my_book_progress: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    my_bookshelf: Option<Vec<serde_json::Value>>,
}

// ==================== 辅助函数 ====================

fn basic_auth_header(username: &str, password: &str) -> String {
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", username, password))
    )
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

    // 读取所有数据类型
    let bookshelf = state.book_service.get_bookshelf(&user_ns).await.unwrap_or_default();
    let book_sources = state.book_source_service.list(&user_ns).await.unwrap_or_default();
    let book_groups: Vec<crate::model::book_group::BookGroup> = state.json_document_service.read_list(&user_ns, "book_groups.json").await.unwrap_or_default();
    let bookmarks: Vec<crate::model::bookmark::Bookmark> = state.json_document_service.read_list(&user_ns, "bookmark.json").await.unwrap_or_default();
    let replace_rules: Vec<crate::model::replace_rule::ReplaceRule> = state.json_document_service.read_list(&user_ns, "replaceRule.json").await.unwrap_or_default();
    let rss_sources: Vec<crate::model::rss::RssSource> = state.json_document_service.read_list(&user_ns, "rssSources.json").await.unwrap_or_default();

    // 构建 ZIP 文件名 (兼容 hectorqin/reader 格式)
    let filename = format!(
        "backup{}.zip",
        chrono::Utc::now().format("%Y-%m-%d")
    );
    let remote_path = format!("{}/legado/{}", req.path.trim_end_matches('/'), filename);
    let client = reqwest::Client::new();
    let auth_header = basic_auth_header(&config.username, &config.password);

    // 打包为 ZIP (每个数据类型一个 JSON 文件)
    let mut zip_buf = Cursor::new(Vec::new());
    {
        let mut archive = ZipWriter::new(&mut zip_buf);
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // 书源
        let book_source_json = serde_json::to_string_pretty(&book_sources)
            .map_err(|e| AppError::Internal(e.into()))?;
        archive.start_file("bookSource.json", options.clone()).map_err(|e| AppError::Internal(e.into()))?;
        archive.write_all(book_source_json.as_bytes()).map_err(|e| AppError::Internal(e.into()))?;

        // 书架
        let bookshelf_json = serde_json::to_string_pretty(&bookshelf)
            .map_err(|e| AppError::Internal(e.into()))?;
        archive.start_file("bookshelf.json", options.clone()).map_err(|e| AppError::Internal(e.into()))?;
        archive.write_all(bookshelf_json.as_bytes()).map_err(|e| AppError::Internal(e.into()))?;

        // 替换规则
        let replace_rule_json = serde_json::to_string_pretty(&replace_rules)
            .map_err(|e| AppError::Internal(e.into()))?;
        archive.start_file("replaceRule.json", options.clone()).map_err(|e| AppError::Internal(e.into()))?;
        archive.write_all(replace_rule_json.as_bytes()).map_err(|e| AppError::Internal(e.into()))?;

        // RSS 源
        let rss_json = serde_json::to_string_pretty(&rss_sources)
            .map_err(|e| AppError::Internal(e.into()))?;
        archive.start_file("rssSources.json", options.clone()).map_err(|e| AppError::Internal(e.into()))?;
        archive.write_all(rss_json.as_bytes()).map_err(|e| AppError::Internal(e.into()))?;

        // 书签
        let bookmark_json = serde_json::to_string_pretty(&bookmarks)
            .map_err(|e| AppError::Internal(e.into()))?;
        archive.start_file("bookmark.json", options.clone()).map_err(|e| AppError::Internal(e.into()))?;
        archive.write_all(bookmark_json.as_bytes()).map_err(|e| AppError::Internal(e.into()))?;

        // 书籍分组
        let group_json = serde_json::to_string_pretty(&book_groups)
            .map_err(|e| AppError::Internal(e.into()))?;
        archive.start_file("bookGroup.json", options).map_err(|e| AppError::Internal(e.into()))?;
        archive.write_all(group_json.as_bytes()).map_err(|e| AppError::Internal(e.into()))?;

        archive.finish().map_err(|e| AppError::Internal(e.into()))?;
    }
    let zip_bytes = zip_buf.into_inner();
    let response = client
        .put(format!("{}{}", config.server_url, remote_path))
        .header("Authorization", auth_header)
        .header("Content-Type", "application/octet-stream")
        .body(zip_bytes.clone())
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
        size: zip_bytes.len() as u64,
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
    let files = parse_webdav_response(&body, &path);
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

    // 逐个读取 ZIP 中的 JSON 文件并恢复数据
    for i in 0..archive.len() {
        let file_content = {
            let mut file = archive.by_index(i).map_err(|e| AppError::Internal(e.into()))?;
            let name = file.name().to_string();
            let mut content = String::new();
            std::io::Read::read_to_string(&mut file, &mut content)
                .map_err(|e| AppError::Internal(e.into()))?;
            (name, content)
        }; // file dropped here, before any await
        let (name, content) = file_content;

        match name.as_str() {
            "bookshelf.json" => {
                if let Ok(books) = serde_json::from_str::<Vec<crate::model::book::Book>>(&content) {
                    if !books.is_empty() {
                        let _ = state.book_service.save_books(&user_ns, books).await;
                    }
                }
            }
            "bookSource.json" => {
                if let Ok(sources) = serde_json::from_str::<Vec<crate::model::book_source::BookSource>>(&content) {
                    if !sources.is_empty() {
                        let _ = state.book_source_service.save_many(&user_ns, sources).await;
                    }
                }
            }
            "replaceRule.json" => {
                if let Ok(rules) = serde_json::from_str::<Vec<crate::model::replace_rule::ReplaceRule>>(&content) {
                    if !rules.is_empty() {
                        let _ = state.json_document_service.write_list(&user_ns, "replaceRule.json", &rules).await;
                    }
                }
            }
            "rssSources.json" => {
                if let Ok(rss) = serde_json::from_str::<Vec<crate::model::rss::RssSource>>(&content) {
                    if !rss.is_empty() {
                        let _ = state.json_document_service.write_list(&user_ns, "rssSources.json", &rss).await;
                    }
                }
            }
            "bookmark.json" => {
                if let Ok(bookmarks) = serde_json::from_str::<Vec<crate::model::bookmark::Bookmark>>(&content) {
                    if !bookmarks.is_empty() {
                        let _ = state.json_document_service.write_list(&user_ns, "bookmark.json", &bookmarks).await;
                    }
                }
            }
            "bookGroup.json" => {
                if let Ok(groups) = serde_json::from_str::<Vec<crate::model::book_group::BookGroup>>(&content) {
                    if !groups.is_empty() {
                        let _ = state.json_document_service.write_list(&user_ns, "book_groups.json", &groups).await;
                    }
                }
            }
            _ => {} // 忽略其他文件
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
