const API_BASE = '';

export function connectSSE(path: string): EventSource {
  return new EventSource(path, { withCredentials: true });
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
    credentials: 'include',
    ...options,
  });

  if (!res.ok) {
    if (res.status === 401) {
      window.location.href = '/api/auth/login';
      throw new Error('Unauthorized');
    }
    const error = await res.text();
    throw new Error(error || res.statusText);
  }

  if (res.status === 204) return undefined as T;
  return res.json();
}

export interface Stack {
  id: string;
  name: string;
  description: string | null;
  compose: string;
  status: string;
  path: string;
  created_at: string;
  updated_at: string;
}

export interface DashboardStatus {
  total_stacks: number;
  running_stacks: number;
  stopped_stacks: number;
}

export interface DockerInfo {
  version: string;
  engine: string;
  containers_total: number;
  containers_running: number;
  images: number;
  disk_usage: number;
}

export interface LogEntry {
  content: string;
  level: string;
  created_at: string;
}

export interface EnvFile {
  id: string;
  stack_id: string;
  filename: string;
  content: string;
  created_at: string;
  updated_at: string;
}

export interface Notifier {
  id: string;
  name: string;
  notifier_type: string;
  config_json: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface StackStats {
  stack_id: string;
  last_started_at: string | null;
  total_running_seconds: number;
}

export interface StackSync {
  stack_id: string;
  sync_type: string;
  remote_url: string | null;
  remote_branch: string;
  auth_token: string | null;
  last_commit: string | null;
  last_synced_at: string | null;
  status: string;
}

export interface SyncStatus {
  stack_id: string;
  status: string;
}

export interface GitDiff {
  files_changed: string[];
  additions: number;
  deletions: number;
  diff_text: string;
}

export const api = {
  // Health
  health: () => request<{ status: string }>('/health'),

  // Stacks
  listStacks: () => request<Stack[]>('/api/stacks'),
  getStack: (id: string) => request<Stack>(`/api/stacks/${id}`),
  createStack: (data: { name: string; description?: string; compose?: string }) =>
    request<Stack>('/api/stacks', { method: 'POST', body: JSON.stringify(data) }),
  updateStack: (id: string, data: { name?: string; description?: string; compose?: string }) =>
    request<Stack>(`/api/stacks/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  deleteStack: (id: string) =>
    request<void>(`/api/stacks/${id}`, { method: 'DELETE' }),
  startStack: (id: string) =>
    request<Stack>(`/api/stacks/${id}/start`, { method: 'POST' }),
  stopStack: (id: string) =>
    request<Stack>(`/api/stacks/${id}/stop`, { method: 'POST' }),
  restartStack: (id: string) =>
    request<Stack>(`/api/stacks/${id}/restart`, { method: 'POST' }),

  // Compose
  getCompose: (id: string) =>
    request<{ id: string; name: string; compose: string }>(`/api/stacks/${id}/compose`),
  updateCompose: (id: string, compose: string) =>
    request<Stack>(`/api/stacks/${id}/compose`, { method: 'PUT', body: JSON.stringify({ compose }) }),
  validateCompose: (compose: string) =>
    request<{ valid: boolean; error?: string }>('/api/stacks/validate', {
      method: 'POST',
      body: JSON.stringify({ compose }),
    }),

  // Pull images
  pullStack: (id: string) =>
    request<Stack>(`/api/stacks/${id}/pull`, { method: 'POST' }),

  // Status
  getStatus: () => request<DashboardStatus>('/api/status'),

  // Sync
  getSyncConfig: (id: string) => request<StackSync | null>(`/api/stacks/${id}/sync`),
  setSyncConfig: (id: string, data: { sync_type?: string; remote_url?: string; remote_branch?: string; auth_token?: string }) =>
    request<StackSync>(`/api/stacks/${id}/sync`, { method: 'PUT', body: JSON.stringify(data) }),
  syncPull: (id: string) => request<{ message: string; commit?: string }>(`/api/stacks/${id}/sync/pull`, { method: 'POST' }),
  syncPush: (id: string) => request<{ message: string; commit?: string }>(`/api/stacks/${id}/sync/push`, { method: 'POST' }),
  syncDiff: (id: string) => request<GitDiff>(`/api/stacks/${id}/sync/diff`),
  syncStatus: (id: string) => request<SyncStatus>(`/api/stacks/${id}/sync/status`),

  // Backup
  getBackupConfig: () => request<any>('/api/backup/config'),
  setBackupConfig: (data: { enabled?: boolean; cron_expression?: string; retention_days?: number }) =>
    request<any>('/api/backup/config', { method: 'POST', body: JSON.stringify(data) }),
  runBackup: () => request<{ status: string; path: string }>('/api/backup/run', { method: 'POST' }),

  // Me
  me: () => request<{ sub: string; email?: string; name?: string }>('/api/me'),

  // Convert
  convertDockerRun: (command: string, serviceName?: string) =>
    request<{ valid: boolean; compose?: string; error?: string; service_name?: string }>('/api/convert/docker-run', {
      method: 'POST',
      body: JSON.stringify({ command, service_name: serviceName }),
    }),

  // Docker Info
  getDockerInfo: () => request<DockerInfo>('/api/docker/info'),

  // Stack Logs
  getStackLogs: (id: string, limit?: number, offset?: number) => {
    const params = new URLSearchParams();
    if (limit !== undefined) params.set('limit', String(limit));
    if (offset !== undefined) params.set('offset', String(offset));
    const qs = params.toString();
    return request<LogEntry[]>(`/api/stacks/${id}/logs${qs ? '?' + qs : ''}`);
  },

  // Env Files
  listEnvFiles: (id: string) => request<EnvFile[]>(`/api/stacks/${id}/env`),
  upsertEnvFile: (id: string, filename: string, content: string) =>
    request<EnvFile>(`/api/stacks/${id}/env`, { method: 'PUT', body: JSON.stringify({ filename, content }) }),
  deleteEnvFile: (id: string, filename: string) =>
    request<void>(`/api/stacks/${id}/env/${encodeURIComponent(filename)}`, { method: 'DELETE' }),

  // Notifiers
  listNotifiers: () => request<Notifier[]>('/api/notifiers'),
  createNotifier: (data: { name: string; notifier_type: string; config_json: string; enabled?: boolean }) =>
    request<Notifier>('/api/notifiers', { method: 'POST', body: JSON.stringify(data) }),
  updateNotifier: (id: string, data: { name?: string; notifier_type?: string; config_json?: string; enabled?: boolean }) =>
    request<Notifier>(`/api/notifiers/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  deleteNotifier: (id: string) => request<void>(`/api/notifiers/${id}`, { method: 'DELETE' }),
  testNotifier: (id: string) => request<{ status: string; message?: string }>(`/api/notifiers/${id}/test`, { method: 'POST' }),

  // Stack Notifier Assignments
  getStackNotifiers: (id: string) => request<string[]>(`/api/stacks/${id}/notifiers`),
  setStackNotifiers: (id: string, notifierIds: string[]) =>
    request<void>(`/api/stacks/${id}/notifiers`, { method: 'PUT', body: JSON.stringify({ notifier_ids: notifierIds }) }),

  // Export
  exportStack: (id: string) => {
    window.open(`/api/stacks/${id}/export`, '_blank');
  },

  // Stats
  getStackStats: (id: string) => request<StackStats>(`/api/stacks/${id}/stats`),
};