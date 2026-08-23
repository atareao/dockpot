const API_BASE = '';

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
      window.location.href = '/auth/login';
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

  // Me
  me: () => request<{ sub: string; email?: string; name?: string }>('/api/me'),
};