// ── Main service editor card for docker-compose visual editor ──
// Layout: Essentials (always visible) + Quick Actions + Tabs with badges.

import React, { useState, useCallback } from 'react';
import {
  Card,
  Tabs,
  Input,
  InputNumber,
  Select,
  Switch,
  Button,
  Space,
  Typography,
  theme,
  Badge,
  Tooltip,
} from 'antd';
import {
  DeleteOutlined,
  ContainerOutlined,
  ThunderboltOutlined,
  PlusOutlined,
} from '@ant-design/icons';
import type { ServiceDef, LoggingDef } from '../../types/compose';
import EnvVarTable from './EnvVarTable';
import PortList from './PortList';
import VolumeMountList from './VolumeMountList';
import FieldList from './FieldList';
import HealthcheckForm from './HealthcheckForm';
import DeployForm from './DeployForm';
import LabelTemplateModal from './LabelTemplateModal';

const { Text } = Typography;

// ── Props ──

export interface ServiceCardProps {
  name: string;
  value: ServiceDef;
  onChange: (name: string, value: ServiceDef) => void;
  onDelete: (name: string) => void;
  volumeNames: string[];
  networkNames: string[];
}

// ── Helpers ──

function clean(svc: ServiceDef): ServiceDef {
  const r: ServiceDef = {};
  for (const [k, v] of Object.entries(svc)) {
    if (v === undefined || v === null) continue;
    if (typeof v === 'string' && v.trim() === '') continue;
    if (Array.isArray(v) && v.length === 0) continue;
    if (typeof v === 'object' && !Array.isArray(v) && Object.keys(v).length === 0) continue;
    (r as Record<string, unknown>)[k] = v;
  }
  return r;
}

const RESTART_OPTIONS = [
  { value: 'no', label: 'no' },
  { value: 'always', label: 'always' },
  { value: 'on-failure', label: 'on-failure' },
  { value: 'unless-stopped', label: 'unless-stopped' },
];

// ── Count helpers for badges ──

function countNetworking(v: ServiceDef): number {
  let c = 0;
  c += v.ports?.length ?? 0;
  c += v.expose?.length ?? 0;
  c += v.networks?.length ?? 0;
  c += v.dns?.length ?? 0;
  c += v.dns_search?.length ?? 0;
  if (v.network_mode) c++;
  c += v.extra_hosts?.length ?? 0;
  return c;
}

function countStorage(v: ServiceDef): number {
  let c = 0;
  c += v.volumes?.length ?? 0;
  c += v.tmpfs?.length ?? 0;
  c += v.configs?.length ?? 0;
  c += v.secrets?.length ?? 0;
  return c;
}

function countEnv(v: ServiceDef): number {
  let c = Object.keys(v.environment ?? {}).length;
  if (v.env_file) c++;
  return c;
}

function countLabels(v: ServiceDef): number {
  return Object.keys(v.labels ?? {}).length;
}

function countMetadata(v: ServiceDef): number {
  return v.profiles?.length ?? 0;
}

function countResources(v: ServiceDef): number {
  let c = 0;
  if (v.deploy) c++;
  if (v.healthcheck) c++;
  if (v.logging?.driver) c++;
  c += v.cap_add?.length ?? 0;
  c += v.cap_drop?.length ?? 0;
  if (v.privileged) c++;
  if (v.shm_size) c++;
  if (v.mem_limit) c++;
  if (v.cpus) c++;
  if (v.mem_reservation) c++;
  return c;
}

function countAdvanced(v: ServiceDef): number {
  let c = 0;
  c += Object.keys(v.depends_on ?? {}).length;
  c += Object.keys(v.sysctls ?? {}).length;
  c += v.security_opt?.length ?? 0;
  if (v.stop_grace_period) c++;
  if (v.stop_signal) c++;
  if (v.pid) c++;
  if (v.runtime) c++;
  if (v.scale) c++;
  return c;
}

function countGeneral(v: ServiceDef): number {
  let c = 0;
  if (v.image) c++;
  if (v.container_name) c++;
  if (v.restart) c++;
  if (v.command) c++;
  if (v.entrypoint) c++;
  if (v.user) c++;
  if (v.working_dir) c++;
  if (v.hostname) c++;
  if (v.stdin_open) c++;
  if (v.tty) c++;
  if (v.read_only) c++;
  if (v.init) c++;
  return c;
}

// ── Badge helper ──

function tabBadge(count: number): React.ReactNode {
  if (count === 0) return null;
  return <Badge count={count} size="small" style={{ backgroundColor: '#1677ff', fontSize: 10, marginLeft: 4 }} />;
}

// ── Component ──

function ServiceCard({
  name,
  value,
  onChange,
  onDelete,
  volumeNames: _volumeNames,
  networkNames,
}: ServiceCardProps) {
  void _volumeNames;
  const { token } = theme.useToken();

  const [editingName, setEditingName] = useState(false);
  const [nameDraft, setNameDraft] = useState(name);
  const [templateModalOpen, setTemplateModalOpen] = useState(false);
  const [activeTab, setActiveTab] = useState('general');

  // ── Generic update helpers ──

  const handleChange = useCallback(
    (patch: Partial<ServiceDef>) => {
      onChange(name, clean({ ...value, ...patch }));
    },
    [name, value, onChange],
  );

  const handleField = useCallback(
    (field: keyof ServiceDef, val: unknown) => {
      handleChange({ [field]: val });
    },
    [handleChange],
  );

  // ── Name editing ──

  const handleNameConfirm = useCallback(() => {
    const trimmed = nameDraft.trim();
    if (trimmed && trimmed !== name) {
      onChange(trimmed, value);
    } else {
      setNameDraft(name);
    }
    setEditingName(false);
  }, [nameDraft, name, value, onChange]);

  const handleNameKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Enter') handleNameConfirm();
      if (e.key === 'Escape') {
        setNameDraft(name);
        setEditingName(false);
      }
    },
    [handleNameConfirm, name],
  );

  // ── Logging ──

  const handleLoggingDriver = useCallback(
    (driver: string) => {
      const next: LoggingDef = { ...(value.logging ?? {}), driver: driver || undefined };
      if (!next.driver && !next.options) {
        handleChange({ logging: undefined });
      } else {
        handleChange({ logging: next });
      }
    },
    [value.logging, handleChange],
  );

  const handleLoggingOptions = useCallback(
    (options: Record<string, string>) => {
      if (Object.keys(options).length === 0 && !value.logging?.driver) {
        handleChange({ logging: undefined });
      } else {
        handleChange({ logging: { ...(value.logging ?? {}), options } });
      }
    },
    [value.logging, handleChange],
  );

  // ── Depends on ──

  const dependsOnAsRecord = useCallback((): Record<string, string> => {
    const d = value.depends_on;
    if (!d) return {};
    if (Array.isArray(d)) {
      const r: Record<string, string> = {};
      for (const s of d) r[s] = '';
      return r;
    }
    const r: Record<string, string> = {};
    for (const [k, v] of Object.entries(d)) {
      r[k] = v?.condition ?? '';
    }
    return r;
  }, [value.depends_on]);

  const handleDependsOn = useCallback(
    (record: Record<string, string>) => {
      const entries = Object.entries(record).filter(([k]) => k.trim());
      if (entries.length === 0) {
        handleChange({ depends_on: undefined });
      } else {
        const result: Record<string, { condition?: string }> = {};
        for (const [k, v] of entries) {
          result[k] = v.trim() ? { condition: v.trim() } : {};
        }
        handleChange({ depends_on: result as any });
      }
    },
    [handleChange],
  );

  // ── Env file ──

  const envFileValue = useCallback((): string => {
    const ef = value.env_file;
    if (!ef) return '';
    return Array.isArray(ef) ? ef.join(', ') : ef;
  }, [value.env_file]);

  const handleEnvFile = useCallback(
    (val: string) => {
      if (!val.trim()) {
        handleChange({ env_file: undefined });
      } else {
        const parts = val.split(',').map((s) => s.trim()).filter(Boolean);
        handleChange({ env_file: parts.length === 1 ? parts[0] : parts });
      }
    },
    [handleChange],
  );

  // ── Quick Actions ──

  const switchToTab = useCallback((tab: string) => setActiveTab(tab), []);

  const quickAddPort = useCallback(() => {
    switchToTab('networking');
    const ports = [...(value.ports ?? []), ''];
    handleField('ports', ports);
  }, [value.ports, handleField, switchToTab]);

  const quickAddVolume = useCallback(() => {
    switchToTab('storage');
    const volumes = [...(value.volumes ?? []), ''];
    handleField('volumes', volumes);
  }, [value.volumes, handleField, switchToTab]);

  const quickAddEnv = useCallback(() => {
    switchToTab('environment');
    const env = { ...(value.environment ?? {}), '': '' };
    handleField('environment', env);
  }, [value.environment, handleField, switchToTab]);

  const quickAddLabel = useCallback(() => {
    switchToTab('labels');
    const labels = { ...(value.labels ?? {}), '': '' };
    handleField('labels', labels);
  }, [value.labels, handleField, switchToTab]);

  const quickAddNetwork = useCallback(() => {
    switchToTab('networking');
    const networks = [...(value.networks ?? []), ''];
    handleField('networks', networks);
  }, [value.networks, handleField, switchToTab]);

  const quickAddHealthcheck = useCallback(() => {
    switchToTab('resources');
    if (!value.healthcheck) {
      handleField('healthcheck', { test: 'curl -f http://localhost || exit 1', interval: '30s', retries: 3 });
    }
  }, [value.healthcheck, handleField, switchToTab]);

  // ── Label helper ──

  const label = (text: string) => (
    <Text
      style={{
        fontSize: 12,
        fontWeight: 500,
        display: 'block',
        marginBottom: 3,
        color: token.colorTextSecondary,
      }}
    >
      {text}
    </Text>
  );

  // ── Styles ──

  const gridStyle: React.CSSProperties = {
    display: 'grid',
    gridTemplateColumns: '1fr 1fr',
    gap: 10,
  };

  const inputStyle: React.CSSProperties = {
    fontFamily: "'Cascadia Code', 'Fira Code', 'Consolas', monospace",
    fontSize: 13,
  };

  // ── Counts ──

  const netCount = countNetworking(value);
  const stoCount = countStorage(value);
  const envCount = countEnv(value);
  const lblCount = countLabels(value);
  const metCount = countMetadata(value);
  const resCount = countResources(value);
  const advCount = countAdvanced(value);
  const genCount = countGeneral(value);

  // ── Tab items ──

  const tabItems = [
    {
      key: 'general',
      label: <span>📋 General {tabBadge(genCount)}</span>,
      children: (
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <div style={gridStyle}>
            <div>
              {label('image')}
              <Tooltip title="Docker image name (e.g. nginx:latest, postgres:16)">
              <Input placeholder="nginx:latest" value={value.image ?? ''}
                onChange={(e) => handleField('image', e.target.value)} style={inputStyle} />
              </Tooltip>
            </div>
            <div>
              {label('container_name')}
              <Tooltip title="Container name at runtime (supports ${STACK_NAME} variable)">
              <Input placeholder="${STACK_NAME}-app" value={value.container_name ?? ''}
                onChange={(e) => handleField('container_name', e.target.value)} style={inputStyle} />
              </Tooltip>
            </div>
            <div>
              {label('restart')}
              <Tooltip title="Restart policy when container exits">
              <Select allowClear placeholder="Select restart policy"
                value={value.restart ?? undefined}
                onChange={(v) => handleField('restart', v ?? undefined)}
                style={{ width: '100%' }} options={RESTART_OPTIONS} />
              </Tooltip>
            </div>
            <div>
              {label('command')}
              <Tooltip title="Command to run instead of the image's default CMD">
              <Input placeholder="nginx -g 'daemon off;'"
                value={Array.isArray(value.command) ? value.command.join(' ') : (value.command ?? '')}
                onChange={(e) => handleField('command', e.target.value)} style={inputStyle} />
              </Tooltip>
            </div>
            <div>
              {label('entrypoint')}
              <Tooltip title="Entrypoint override for the container">
              <Input placeholder="/docker-entrypoint.sh"
                value={Array.isArray(value.entrypoint) ? value.entrypoint.join(' ') : (value.entrypoint ?? '')}
                onChange={(e) => handleField('entrypoint', e.target.value)} style={inputStyle} />
              </Tooltip>
            </div>
            <div>
              {label('user')}
              <Tooltip title="Username or UID to run the container as">
              <Input placeholder="nginx" value={value.user ?? ''}
                onChange={(e) => handleField('user', e.target.value)} style={inputStyle} />
              </Tooltip>
            </div>
            <div>
              {label('working_dir')}
              <Tooltip title="Working directory inside the container">
              <Input placeholder="/usr/share/nginx/html" value={value.working_dir ?? ''}
                onChange={(e) => handleField('working_dir', e.target.value)} style={inputStyle} />
              </Tooltip>
            </div>
            <div>
              {label('hostname')}
              <Tooltip title="Container hostname">
              <Input placeholder="my-host" value={value.hostname ?? ''}
                onChange={(e) => handleField('hostname', e.target.value)} style={inputStyle} />
              </Tooltip>
            </div>
          </div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 16, paddingTop: 4 }}>
            <Space>
              <Tooltip title="Keep STDIN open (equivalent to -i)">
              <Switch size="small" checked={!!value.stdin_open}
                onChange={(v) => handleField('stdin_open', v || undefined)} />
              </Tooltip>
              {label('stdin_open')}
            </Space>
            <Space>
              <Tooltip title="Allocate a pseudo-TTY (equivalent to -t)">
              <Switch size="small" checked={!!value.tty}
                onChange={(v) => handleField('tty', v || undefined)} />
              </Tooltip>
              {label('tty')}
            </Space>
            <Space>
              <Tooltip title="Mount container filesystem as read-only">
              <Switch size="small" checked={!!value.read_only}
                onChange={(v) => handleField('read_only', v || undefined)} />
              </Tooltip>
              {label('read_only')}
            </Space>
            <Space>
              <Tooltip title="Run an init process inside the container (tini)">
              <Switch size="small" checked={!!value.init}
                onChange={(v) => handleField('init', v || undefined)} />
              </Tooltip>
              {label('init')}
            </Space>
          </div>
        </Space>
      ),
    },
    {
      key: 'networking',
      label: <span>🌐 Networking {tabBadge(netCount)}</span>,
      children: (
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <Tooltip title="Container port mappings (host:container)">
          <PortList
            value={value.ports ?? []}
            onChange={(v) => handleField('ports', v.length > 0 ? v : undefined)}
          />
          </Tooltip>
          <Tooltip title="Expose ports without publishing them">
          <FieldList
            title="expose"
            placeholder="e.g. 80/tcp"
            value={value.expose ?? []}
            onChange={(v) => handleField('expose', v.length > 0 ? v : undefined)}
          />
          </Tooltip>
          <div>
            {label('networks')}
            <Tooltip title="Attach to Docker networks">
            <Select
              mode="tags"
              placeholder="Select or type network name"
              value={value.networks ?? []}
              onChange={(v) => handleField('networks', v.length > 0 ? v : undefined)}
              style={{ width: '100%' }}
              options={networkNames.map((n) => ({ value: n, label: n }))}
            />
            </Tooltip>
          </div>
          <Tooltip title="Custom DNS server addresses">
          <FieldList
            title="dns"
            placeholder="e.g. 8.8.8.8"
            value={value.dns ?? []}
            onChange={(v) => handleField('dns', v.length > 0 ? v : undefined)}
          />
          </Tooltip>
          <Tooltip title="DNS search domains">
          <FieldList
            title="dns_search"
            placeholder="e.g. example.com"
            value={value.dns_search ?? []}
            onChange={(v) => handleField('dns_search', v.length > 0 ? v : undefined)}
          />
          </Tooltip>
          <div>
            {label('network_mode')}
            <Tooltip title="Network mode (bridge, host, none, service:...)">
            <Input

              placeholder="bridge, host, none, service:..."
              value={value.network_mode ?? ''}
              onChange={(e) => handleField('network_mode', e.target.value)}
              style={inputStyle}
            />
            </Tooltip>
          </div>
          <Tooltip title="Hostname-to-IP mappings (extra_hosts)">
          <FieldList
            title="extra_hosts"
            placeholder="e.g. host.docker.internal:host-gateway"
            value={value.extra_hosts ?? []}
            onChange={(v) => handleField('extra_hosts', v.length > 0 ? v : undefined)}
          />
          </Tooltip>
        </Space>
      ),
    },
    {
      key: 'storage',
      label: <span>💾 Storage {tabBadge(stoCount)}</span>,
      children: (
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <div>
            <Text strong style={{ fontSize: 13, display: 'block', marginBottom: 6 }}>volumes</Text>
            <Tooltip title="Mount host paths or named volumes into the container">
            <VolumeMountList
              value={value.volumes ?? []}
              onChange={(v) => handleField('volumes', v.length > 0 ? v : undefined)}
              volumeNames={_volumeNames.length > 0 ? _volumeNames : undefined}
            />
            </Tooltip>
          </div>
          <Tooltip title="Mount a temporary filesystem in RAM">
          <FieldList
            title="tmpfs"
            placeholder="e.g. /run:size=100M"
            value={value.tmpfs ?? []}
            onChange={(v) => handleField('tmpfs', v.length > 0 ? v : undefined)}
          />
          </Tooltip>
          <Tooltip title="Use Docker configs (Swarm mode)">
          <FieldList
            title="configs"
            placeholder="e.g. my_config"
            value={value.configs ?? []}
            onChange={(v) => handleField('configs', v.length > 0 ? v : undefined)}
          />
          </Tooltip>
          <Tooltip title="Use Docker secrets (Swarm mode)">
          <FieldList
            title="secrets"
            placeholder="e.g. db_password"
            value={value.secrets ?? []}
            onChange={(v) => handleField('secrets', v.length > 0 ? v : undefined)}
          />
          </Tooltip>
        </Space>
      ),
    },
    {
      key: 'environment',
      label: <span>🔤 Environment {tabBadge(envCount)}</span>,
      children: (
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <Tooltip title="Set environment variables (KEY=VALUE)">
          <EnvVarTable
            title="environment"
            keyPlaceholder="VAR_NAME"
            valuePlaceholder="VALUE"
            value={value.environment ?? {}}
            onChange={(v) => handleField('environment', Object.keys(v).length > 0 ? v : undefined)}
          />
          </Tooltip>
          <div>
            {label('env_file')}
            <Tooltip title="Load environment variables from a file (.env)">
            <Input

              placeholder=".env, .env.production (comma-separated)"
              value={envFileValue()}
              onChange={(e) => handleEnvFile(e.target.value)}
              style={inputStyle}
            />
            </Tooltip>
          </div>
        </Space>
      ),
    },
    {
      key: 'labels',
      label: <span>🏷️ Labels {tabBadge(lblCount)}</span>,
      children: (
        <Space direction="vertical" size="small" style={{ width: '100%' }}>
          <Tooltip title="Metadata labels for the container (used by Traefik, etc.)">
          <EnvVarTable
            title="Service Labels"
            keyPlaceholder="KEY"
            valuePlaceholder="VALUE"
            value={value.labels ?? {}}
            onChange={(v) => handleField('labels', Object.keys(v).length > 0 ? v : undefined)}
          />
          </Tooltip>
          <Tooltip title="Quick-add preconfigured label sets (Traefik, Caddy, etc.)">
          <Button
            size="small"
            icon={<ThunderboltOutlined />}
            onClick={() => setTemplateModalOpen(true)}
            block
          >
            ⚡ Templates
          </Button>
          </Tooltip>
          <LabelTemplateModal
            open={templateModalOpen}
            onCancel={() => setTemplateModalOpen(false)}
            onApply={(newLabels) => {
              handleField('labels', newLabels);
              setTemplateModalOpen(false);
            }}
            serviceName={name}
            existingLabels={value.labels ?? {}}
          />
        </Space>
      ),
    },
    {
      key: 'metadata',
      label: <span>📋 Metadata {tabBadge(metCount)}</span>,
      children: (
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <Tooltip title="Activate this service only with matching profiles">
          <FieldList
            title="profiles"
            placeholder="e.g. production"
            value={value.profiles ?? []}
            onChange={(v) => handleField('profiles', v.length > 0 ? v : undefined)}
          />
          </Tooltip>
        </Space>
      ),
    },
    {
      key: 'resources',
      label: <span>⚙️ Resources {tabBadge(resCount)}</span>,
      children: (
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <Tooltip title="Swarm deployment configuration (mode, replicas, resources)">
          <DeployForm
            value={value.deploy}
            onChange={(v) => handleField('deploy', v)}
          />
          </Tooltip>
          <Tooltip title="Container health check command and settings">
          <HealthcheckForm
            value={value.healthcheck}
            onChange={(v) => handleField('healthcheck', v)}
          />
          </Tooltip>
          <div>
            <Text strong style={{ fontSize: 13, display: 'block', marginBottom: 6 }}>logging</Text>
            <Space direction="vertical" size="small" style={{ width: '100%' }}>
              <div>
                {label('driver')}
                <Tooltip title="Log driver (json-file, journald, syslog, etc.)">
                <Input

                  placeholder="json-file, syslog, journald, gelf..."
                  value={value.logging?.driver ?? ''}
                  onChange={(e) => handleLoggingDriver(e.target.value)}
                  style={inputStyle}
                />
                </Tooltip>
              </div>
              <Tooltip title="Driver-specific log options (max-size, max-file, etc.)">
              <EnvVarTable
                title="options"
                keyPlaceholder="max-size"
                valuePlaceholder="10m"
                value={value.logging?.options ?? {}}
                onChange={handleLoggingOptions}
              />
              </Tooltip>
            </Space>
          </div>
          <div style={gridStyle}>
            <Tooltip title="Add Linux capabilities to the container">
            <FieldList
              title="cap_add"
              placeholder="e.g. NET_ADMIN"
              value={value.cap_add ?? []}
              onChange={(v) => handleField('cap_add', v.length > 0 ? v : undefined)}
            />
            </Tooltip>
            <Tooltip title="Drop Linux capabilities from the container">
            <FieldList
              title="cap_drop"
              placeholder="e.g. ALL"
              value={value.cap_drop ?? []}
              onChange={(v) => handleField('cap_drop', v.length > 0 ? v : undefined)}
            />
            </Tooltip>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Tooltip title="Give extended privileges to the container">
            <Switch size="small" checked={!!value.privileged}
              onChange={(v) => handleField('privileged', v || undefined)} />
            </Tooltip>
            {label('privileged')}
          </div>
          <div style={gridStyle}>
            <div>
              {label('shm_size')}
              <Tooltip title="Size of /dev/shm (e.g. 64m, 256m)">
              <Input placeholder="64m" value={value.shm_size ?? ''}
                onChange={(e) => handleField('shm_size', e.target.value)} style={inputStyle} />
              </Tooltip>
            </div>
            <div>
              {label('mem_limit')}
              <Tooltip title="Memory limit (e.g. 512M, 2G)">
              <Input placeholder="512M" value={value.mem_limit ?? ''}
                onChange={(e) => handleField('mem_limit', e.target.value)} style={inputStyle} />
              </Tooltip>
            </div>
            <div>
              {label('cpus')}
              <Tooltip title="CPU limit (e.g. 0.5, 2.0)">
              <Input placeholder="0.5" value={value.cpus ?? ''}
                onChange={(e) => handleField('cpus', e.target.value)} style={inputStyle} />
              </Tooltip>
            </div>
            <div>
              {label('mem_reservation')}
              <Tooltip title="Soft memory reservation (e.g. 256M)">
              <Input placeholder="256M" value={value.mem_reservation ?? ''}
                onChange={(e) => handleField('mem_reservation', e.target.value)} style={inputStyle} />
              </Tooltip>
            </div>
          </div>
        </Space>
      ),
    },
    {
      key: 'advanced',
      label: <span>🔧 Advanced {tabBadge(advCount)}</span>,
      children: (
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <Tooltip title="Services this service depends on, with optional condition">
          <EnvVarTable
            title="depends_on"
            keyPlaceholder="Service name"
            valuePlaceholder="condition (e.g. service_healthy)"
            value={dependsOnAsRecord()}
            onChange={handleDependsOn}
          />
          </Tooltip>
          <Tooltip title="Kernel parameters to set in the container">
          <EnvVarTable
            title="sysctls"
            keyPlaceholder="net.core.somaxconn"
            valuePlaceholder="1024"
            value={value.sysctls ?? {}}
            onChange={(v) => handleField('sysctls', Object.keys(v).length > 0 ? v : undefined)}
          />
          </Tooltip>
          <Tooltip title="Security options (e.g. no-new-privileges)">
          <FieldList
            title="security_opt"
            placeholder="e.g. no-new-privileges:true"
            value={value.security_opt ?? []}
            onChange={(v) => handleField('security_opt', v.length > 0 ? v : undefined)}
          />
          </Tooltip>
          <div style={gridStyle}>
            <div>
              {label('stop_grace_period')}
              <Tooltip title="Time to wait before force-killing (e.g. 10s)">
              <Input placeholder="10s" value={value.stop_grace_period ?? ''}
                onChange={(e) => handleField('stop_grace_period', e.target.value)} style={inputStyle} />
              </Tooltip>
            </div>
            <div>
              {label('stop_signal')}
              <Tooltip title="Signal to stop the container (e.g. SIGTERM, SIGINT)">
              <Input placeholder="SIGTERM" value={value.stop_signal ?? ''}
                onChange={(e) => handleField('stop_signal', e.target.value)} style={inputStyle} />
              </Tooltip>
            </div>
            <div>
              {label('pid')}
              <Tooltip title="PID namespace mode (host for sharing host PID namespace)">
              <Input placeholder="host" value={value.pid ?? ''}
                onChange={(e) => handleField('pid', e.target.value)} style={inputStyle} />
              </Tooltip>
            </div>
            <div>
              {label('runtime')}
              <Tooltip title="Runtime to use (e.g. runc, nvidia)">
              <Input placeholder="runc" value={value.runtime ?? ''}
                onChange={(e) => handleField('runtime', e.target.value)} style={inputStyle} />
              </Tooltip>
            </div>
          </div>
          <div>
            {label('scale')}
            <Tooltip title="Number of replicas (Swarm mode)">
            <InputNumber min={1} placeholder="1" value={value.scale ?? null}
              onChange={(v) => handleField('scale', v ?? undefined)} style={{ width: '100%' }} />
            </Tooltip>
          </div>
        </Space>
      ),
    },
  ];

  // ── Render ──

  return (
    <Card
      size="small"
      style={{ marginBottom: 12 }}
      styles={{
        header: { padding: '8px 12px', minHeight: 40 },
        body: { padding: 0 },
      }}
      title={
        editingName ? (
          <Input

            value={nameDraft}
            onChange={(e) => setNameDraft(e.target.value)}
            onBlur={handleNameConfirm}
            onKeyDown={handleNameKeyDown}
            autoFocus
            style={{ width: 240, ...inputStyle }}
          />
        ) : (
          <Space size={4} style={{ cursor: 'pointer' }}
            onClick={() => { setNameDraft(name); setEditingName(true); }}>
            <ContainerOutlined style={{ fontSize: 14, color: token.colorPrimary }} />
            <Text strong style={{ fontSize: 14 }}>{name}</Text>
            <Text type="secondary" style={{ fontSize: 11 }}>(click to rename)</Text>
          </Space>
        )
      }
      extra={
        <Button size="small" danger icon={<DeleteOutlined />} onClick={() => onDelete(name)}>
          Delete
        </Button>
      }
    >

      {/* Quick Actions */}
      <div style={{
        padding: '6px 12px',
        borderBottom: `1px solid ${token.colorBorderSecondary}`,
        display: 'flex', flexWrap: 'wrap', gap: 4,
      }}>
        <Tooltip title="Add a port mapping (host:container)">
          <Button size="small" icon={<PlusOutlined />} onClick={quickAddPort}>Port</Button>
        </Tooltip>
        <Tooltip title="Add a volume mount (source:target)">
          <Button size="small" icon={<PlusOutlined />} onClick={quickAddVolume}>Volume</Button>
        </Tooltip>
        <Tooltip title="Add an environment variable">
          <Button size="small" icon={<PlusOutlined />} onClick={quickAddEnv}>Env</Button>
        </Tooltip>
        <Tooltip title="Add a metadata label">
          <Button size="small" icon={<PlusOutlined />} onClick={quickAddLabel}>Label</Button>
        </Tooltip>
        <Tooltip title="Attach to a Docker network">
          <Button size="small" icon={<PlusOutlined />} onClick={quickAddNetwork}>Network</Button>
        </Tooltip>
        <Tooltip title="Add a container health check">
          <Button size="small" icon={<PlusOutlined />} onClick={quickAddHealthcheck}>Healthcheck</Button>
        </Tooltip>
      </div>

      {/* Tabs */}
      <Tabs
        size="small"
        type="card"
        activeKey={activeTab}
        onChange={setActiveTab}
        items={tabItems}
        style={{ margin: '0 4px' }}
      />
    </Card>
  );
}

export default ServiceCard;
export { ServiceCard };