import http from './http'

export interface WebdavFileEntry {
  name: string
  size: number
  path: string
  lastModified: number
  isDirectory: boolean
}

export function getWebdavFileList(path = '/') {
  return http.get<WebdavFileEntry[]>('/getWebdavFileList', {
    params: { path },
  }).then((r) => r.data)
}

export function getWebdavFileText(path: string) {
  return http.get<string>('/getWebdavFile', {
    params: { path },
    responseType: 'text',
    transformResponse: [(value) => value],
  }).then((r) => r.data as unknown as string)
}

export function getWebdavFileBlob(path: string) {
  return http.get<Blob>('/getWebdavFile', {
    params: { path },
    responseType: 'blob',
  }).then((r) => r.data)
}

export function uploadFilesToWebdav(files: Array<{ file: Blob; name: string }>, path = '/') {
  const formData = new FormData()
  formData.append('path', path)
  files.forEach((item, index) => {
    formData.append(`file${index}`, item.file, item.name)
  })
  return http.post<WebdavFileEntry[]>('/uploadFileToWebdav', formData, {
    headers: {
      'Content-Type': 'multipart/form-data',
    },
  }).then((r) => r.data)
}

export function uploadTextToWebdav(content: string, filename: string, path = '/') {
  const blob = new Blob([content], { type: 'application/json;charset=utf-8' })
  return uploadFilesToWebdav([{ file: blob, name: filename }], path)
}

export function deleteWebdavFile(path: string) {
  return http.post<string>('/deleteWebdavFile', { path }).then((r) => r.data)
}

export function deleteWebdavFileList(paths: string[]) {
  return http.post<string>('/deleteWebdavFileList', { path: paths }).then((r) => r.data)
}

// ==================== 远程 WebDAV 客户端 ====================

export interface RemoteWebdavConfig {
  server_url: string
  username: string
  enabled: boolean
}

export interface RemoteWebdavFileEntry {
  name: string
  size: number
  path: string
  lastModified: number
  isDirectory: boolean
}

export interface TestResult {
  connected: boolean
  message: string
}

export function saveWebdavConfig(config: { server_url: string; username: string; password: string }) {
  return http.post('/saveWebdavConfig', config).then((r) => r.data)
}

export function getWebdavConfig(): Promise<RemoteWebdavConfig> {
  return http.get('/getWebdavConfig').then((r) => r.data)
}

export function testWebdavConnection(config: { server_url: string; username: string; password: string }): Promise<TestResult> {
  return http.post('/testWebdavConnection', config).then((r) => r.data)
}

export function backupToRemoteWebdav(path: string): Promise<{ file_name: string; size: number }> {
  return http.post('/backupToRemoteWebdav', { path }).then((r) => r.data)
}

export function getRemoteWebdavFileList(path: string): Promise<RemoteWebdavFileEntry[]> {
  return http.get('/getRemoteWebdavFileList', { params: { path } }).then((r) => r.data)
}

export function restoreFromRemoteWebdav(path: string): Promise<{ restored: boolean }> {
  return http.post('/restoreFromRemoteWebdav', { path }).then((r) => r.data)
}
