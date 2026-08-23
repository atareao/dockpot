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

  // Me
  me: () => request<{ sub: string; email?: string; name?: string }>('/api/me'),
};