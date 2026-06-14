<template>
  <div class="remote-webdav-panel">
    <h3>远程 WebDAV 备份</h3>
    
    <div class="form-group">
      <label>服务器地址</label>
      <input 
        v-model="config.server_url" 
        placeholder="https://dav.jianguoyun.com/dav/"
        :disabled="!editable"
      />
    </div>
    
    <div class="form-group">
      <label>用户名</label>
      <input 
        v-model="config.username" 
        placeholder="用户名"
        :disabled="!editable"
      />
    </div>
    
    <div class="form-group">
      <label>密码</label>
      <input 
        v-model="password" 
        type="password" 
        placeholder="密码"
        :disabled="!editable"
      />
    </div>
    
    <div class="actions">
      <button 
        v-if="!editable" 
        class="btn btn-primary" 
        @click="editable = true"
      >
        编辑配置
      </button>
      
      <button 
        v-else 
        class="btn btn-secondary" 
        @click="cancelEdit"
      >
        取消
      </button>
      
      <button 
        class="btn btn-success" 
        @click="saveConfig"
        :disabled="!editable"
      >
        保存
      </button>
      
      <button 
        class="btn btn-info" 
        @click="testConnection"
        :disabled="!config.server_url"
      >
        测试连接
      </button>
    </div>
    
    <div v-if="testResult" class="test-result" :class="testResult.connected ? 'success' : 'error'">
      {{ testResult.message }}
    </div>
    
    <div v-if="editable" class="remote-actions">
      <button 
        class="btn btn-primary" 
        @click="backupNow"
        :disabled="!config.enabled"
      >
        立即备份
      </button>
      
      <button 
        class="btn btn-warning" 
        @click="showRestoreDialog = true"
        :disabled="!config.enabled"
      >
        从远程恢复
      </button>
    </div>
    
    <!-- 恢复确认对话框 -->
    <div v-if="showRestoreDialog" class="modal-overlay" @click="showRestoreDialog = false">
      <div class="modal-content" @click.stop>
        <h3>确认恢复</h3>
        <p>从远程 WebDAV 恢复数据将覆盖当前所有数据。</p>
        <p>请选择要恢复的备份文件：</p>
        
        <select v-model="selectedBackup" class="backup-select">
          <option value="">加载中...</option>
        </select>
        
        <div class="modal-actions">
          <button class="btn btn-secondary" @click="showRestoreDialog = false">取消</button>
          <button 
            class="btn btn-danger" 
            @click="restoreFromRemote"
            :disabled="!selectedBackup"
          >
            确认恢复
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import {
  saveWebdavConfig,
  getWebdavConfig,
  testWebdavConnection,
  backupToRemoteWebdav,
  restoreFromRemoteWebdav,
  type RemoteWebdavConfig,
  type TestResult,
} from '../api/webdav'

const config = ref<RemoteWebdavConfig>({
  server_url: '',
  username: '',
  enabled: false,
})

const password = ref('')
const editable = ref(false)
const testResult = ref<TestResult | null>(null)
const showRestoreDialog = ref(false)
const selectedBackup = ref('')

onMounted(async () => {
  try {
    const data = await getWebdavConfig()
    config.value = data
  } catch (error) {
    console.error('Failed to load webdav config:', error)
  }
})

function cancelEdit() {
  editable.value = false
  password.value = ''
}

async function saveConfig() {
  try {
    await saveWebdavConfig({
      server_url: config.value.server_url,
      username: config.value.username,
      password: password.value,
    })
    config.value.enabled = true
    editable.value = false
    testResult.value = { connected: true, message: '配置已保存' }
  } catch (error) {
    testResult.value = { connected: false, message: '保存失败' }
  }
}

async function testConnection() {
  try {
    const result = await testWebdavConnection({
      server_url: config.value.server_url,
      username: config.value.username,
      password: password.value,
    })
    testResult.value = result
  } catch (error) {
    testResult.value = { connected: false, message: '测试失败' }
  }
}

async function backupNow() {
  try {
    await backupToRemoteWebdav('/backups')
    testResult.value = { connected: true, message: '备份成功' }
  } catch (error) {
    testResult.value = { connected: false, message: '备份失败' }
  }
}

async function restoreFromRemote() {
  if (!selectedBackup.value) return
  
  try {
    await restoreFromRemoteWebdav(selectedBackup.value)
    testResult.value = { connected: true, message: '恢复成功' }
    setTimeout(() => {
      window.location.reload()
    }, 1000)
  } catch (error) {
    testResult.value = { connected: false, message: '恢复失败' }
  }
}
</script>

<style scoped>
.remote-webdav-panel {
  padding: 20px;
  border: 1px solid var(--color-border-light);
  border-radius: 8px;
  background: var(--color-bg-sunken);
}

.form-group {
  margin-bottom: 16px;
}

.form-group label {
  display: block;
  margin-bottom: 4px;
  font-weight: 500;
  color: var(--color-text);
}

.form-group input {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--color-border-light);
  border-radius: 4px;
  background: var(--color-bg-elevated);
  color: var(--color-text);
  font-size: 14px;
}

.actions {
  display: flex;
  gap: 8px;
  margin: 16px 0;
}

.btn {
  padding: 8px 16px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.2s;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  background: var(--color-primary);
  color: white;
}

.btn-secondary {
  background: var(--color-bg-sunken);
  color: var(--color-text);
}

.btn-success {
  background: #22c55e;
  color: white;
}

.btn-info {
  background: #3b82f6;
  color: white;
}

.btn-warning {
  background: #f59e0b;
  color: white;
}

.btn-danger {
  background: #ef4444;
  color: white;
}

.test-result {
  padding: 12px;
  border-radius: 4px;
  margin: 16px 0;
}

.test-result.success {
  background: rgba(34, 197, 94, 0.1);
  border: 1px solid rgba(34, 197, 94, 0.3);
  color: #22c55e;
}

.test-result.error {
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.3);
  color: #ef4444;
}

.remote-actions {
  display: flex;
  gap: 8px;
  margin-top: 16px;
}

.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: var(--color-bg-elevated);
  padding: 24px;
  border-radius: 8px;
  min-width: 400px;
  max-width: 600px;
}

.modal-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  margin-top: 16px;
}

.backup-select {
  width: 100%;
  padding: 8px;
  margin: 8px 0;
  border: 1px solid var(--color-border-light);
  border-radius: 4px;
  background: var(--color-bg-elevated);
  color: var(--color-text);
}
</style>
