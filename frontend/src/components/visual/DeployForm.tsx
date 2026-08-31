// ── Deploy configuration sub-form for the Docker Compose visual editor ──

import { useCallback, useMemo } from 'react';
import {
  Input,
  InputNumber,
  Select,
  Button,
  Space,
  Collapse,
  Typography,
  Tooltip,
} from 'antd';
import { DeleteOutlined, PlusOutlined, MinusCircleOutlined } from '@ant-design/icons';
import type { DeployDef, DeployResourcesDef, DeployRestartPolicyDef, DeployPlacementDef } from '../../types/compose';

const { Text } = Typography;
const { Panel } = Collapse;

export interface DeployFormProps {
  value?: DeployDef;
  onChange: (v: DeployDef | undefined) => void;
}

// ── Helpers ──

/** Deep-merge a partial into the current deploy value. */
function mergeDeploy(current: DeployDef | undefined, patch: Partial<DeployDef>): DeployDef {
  return { ...(current ?? {}), ...patch };
}

/** Merge into a nested object (resources, restart_policy, etc.). */
function mergeNested<T>(current: T | undefined, patch: Partial<T>): T {
  return { ...(current ?? ({} as T)), ...patch } as T;
}

// ── Labels key-value editor ──

interface LabelsEditorProps {
  value?: Record<string, string>;
  onChange: (v: Record<string, string> | undefined) => void;
}

function LabelsEditor({ value, onChange }: LabelsEditorProps) {
  const entries = useMemo(() => Object.entries(value ?? {}), [value]);

  const setEntries = useCallback(
    (next: [string, string][]) => {
      const filtered = next.filter(([k]) => k.trim().length > 0);
      if (filtered.length === 0) {
        onChange(undefined);
      } else {
        onChange(Object.fromEntries(filtered));
      }
    },
    [onChange],
  );

  const updateRow = useCallback(
    (index: number, key: string, val: string) => {
      const next = entries.map(([k, v], i) => (i === index ? [key, val] : [k, v])) as [string, string][];
      setEntries(next);
    },
    [entries, setEntries],
  );

  const removeRow = useCallback(
    (index: number) => {
      setEntries(entries.filter((_, i) => i !== index));
    },
    [entries, setEntries],
  );

  const addRow = useCallback(() => {
    setEntries([...entries, ['', '']]);
  }, [entries, setEntries]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
      {entries.length === 0 && (
        <Text type="secondary" style={{ fontSize: 12 }}>No labels defined</Text>
      )}
      {entries.map(([key, val], i) => (
        <Space key={i} style={{ display: 'flex' }} align="start">
          <Input
            size="small"
            placeholder="Key"
            value={key}
            onChange={(e) => updateRow(i, e.target.value, val)}
            style={{ width: 180 }}
          />
          <Input
            size="small"
            placeholder="Value"
            value={val}
            onChange={(e) => updateRow(i, key, e.target.value)}
            style={{ width: 180 }}
          />
          <Button
            size="small"
            type="text"
            danger
            icon={<MinusCircleOutlined />}
            onClick={() => removeRow(i)}
          />
        </Space>
      ))}
      <Button size="small" type="dashed" icon={<PlusOutlined />} onClick={addRow} style={{ alignSelf: 'flex-start' }}>
        Add Label
      </Button>
    </div>
  );
}

// ── Placement constraints editor (list of strings) ──

interface StringListEditorProps {
  value?: string[];
  onChange: (v: string[] | undefined) => void;
  placeholder?: string;
}

function StringListEditor({ value, onChange, placeholder = 'constraint' }: StringListEditorProps) {
  const items = value ?? [];

  const setItems = useCallback(
    (next: string[]) => {
      const filtered = next.filter((s) => s.trim().length > 0);
      onChange(filtered.length > 0 ? filtered : undefined);
    },
    [onChange],
  );

  const updateItem = useCallback(
    (index: number, val: string) => {
      const next = items.map((s, i) => (i === index ? val : s));
      setItems(next);
    },
    [items, setItems],
  );

  const removeItem = useCallback(
    (index: number) => {
      setItems(items.filter((_, i) => i !== index));
    },
    [items, setItems],
  );

  const addItem = useCallback(() => {
    setItems([...items, '']);
  }, [items, setItems]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
      {items.length === 0 && (
        <Text type="secondary" style={{ fontSize: 12 }}>No constraints defined</Text>
      )}
      {items.map((s, i) => (
        <Space key={i} style={{ display: 'flex' }} align="start">
          <Tooltip title="Node placement constraints (e.g. node.labels.role==worker)">
            <Input
              size="small"
              placeholder={placeholder}
              value={s}
              onChange={(e) => updateItem(i, e.target.value)}
              style={{ width: 360 }}
            />
          </Tooltip>
          <Button
            size="small"
            type="text"
            danger
            icon={<MinusCircleOutlined />}
            onClick={() => removeItem(i)}
          />
        </Space>
      ))}
      <Button size="small" type="dashed" icon={<PlusOutlined />} onClick={addItem} style={{ alignSelf: 'flex-start' }}>
        Add Constraint
      </Button>
    </div>
  );
}

// ── Resource block (limits / reservations) ──

interface ResourceBlockProps {
  label: string;
  value?: { cpus?: string; memory?: string; pids?: number };
  onChange: (v: { cpus?: string; memory?: string; pids?: number } | undefined) => void;
  showPids?: boolean;
}

function ResourceBlock({ label, value, onChange, showPids = false }: ResourceBlockProps) {
  const isEmpty = !value?.cpus && !value?.memory && !(showPids && value?.pids);

  const isLimits = label === 'Limits';

  const updateField = useCallback(
    (field: string, val: string | number | undefined) => {
      const next = { ...(value ?? {}), [field]: val };
      // If everything is empty, emit undefined to keep the tree clean
      if (!next.cpus && !next.memory && !(showPids && next.pids)) {
        onChange(undefined);
      } else {
        onChange(next as { cpus?: string; memory?: string; pids?: number });
      }
    },
    [value, onChange, showPids],
  );

  if (isEmpty) {
    return (
      <div style={{ marginBottom: 4 }}>
        <Text type="secondary" style={{ fontSize: 12 }}>{label}: not set</Text>
        <Button
          size="small"
          type="link"
          onClick={() => onChange({})}
          style={{ padding: 0, marginLeft: 8, fontSize: 12 }}
        >
          Configure
        </Button>
      </div>
    );
  }

  return (
    <div style={{ marginBottom: 8 }}>
      <Text strong style={{ fontSize: 12, display: 'block', marginBottom: 4 }}>{label}</Text>
      <Space size="small" wrap>
        <div>
          <Text type="secondary" style={{ fontSize: 11, display: 'block' }}>cpus</Text>
          <Tooltip title={isLimits ? "CPU limit (e.g. '0.5' for half a core)" : 'CPU reservation (guaranteed minimum)'}>
            <Input
              size="small"
              placeholder="e.g. 0.5"
              value={value?.cpus ?? ''}
              onChange={(e) => updateField('cpus', e.target.value || undefined)}
              style={{ width: 100 }}
            />
          </Tooltip>
        </div>
        <div>
          <Text type="secondary" style={{ fontSize: 11, display: 'block' }}>memory</Text>
          <Tooltip title={isLimits ? "Memory limit (e.g. '512M')" : 'Memory reservation (guaranteed minimum)'}>
            <Input
              size="small"
              placeholder="e.g. 512M"
              value={value?.memory ?? ''}
              onChange={(e) => updateField('memory', e.target.value || undefined)}
              style={{ width: 110 }}
            />
          </Tooltip>
        </div>
        {showPids && (
          <div>
            <Text type="secondary" style={{ fontSize: 11, display: 'block' }}>pids</Text>
            <InputNumber
              size="small"
              placeholder="pids"
              min={0}
              value={value?.pids}
              onChange={(v) => updateField('pids', v ?? undefined)}
              style={{ width: 90 }}
            />
          </div>
        )}
      </Space>
      <Button
        size="small"
        type="link"
        danger
        icon={<DeleteOutlined />}
        onClick={() => onChange(undefined)}
        style={{ padding: 0, fontSize: 11, marginTop: 2 }}
      >
        Clear
      </Button>
    </div>
  );
}

// ── Main DeployForm ──

function DeployForm({ value, onChange }: DeployFormProps) {
  const deploy = value;

  // ── Top-level fields ──

  const handleModeChange = useCallback(
    (mode: string) => {
      if (mode === 'replicated' || mode === 'global') {
        onChange(mergeDeploy(deploy, { mode }));
      } else {
        const { mode: _, ...rest } = deploy ?? {};
        onChange(Object.keys(rest).length > 0 ? (rest as DeployDef) : undefined);
      }
    },
    [deploy, onChange],
  );

  const handleReplicasChange = useCallback(
    (val: number | null) => {
      if (val != null && val > 0) {
        onChange(mergeDeploy(deploy, { replicas: val }));
      } else {
        const { replicas: _, ...rest } = deploy ?? {};
        onChange(Object.keys(rest).length > 0 ? (rest as DeployDef) : undefined);
      }
    },
    [deploy, onChange],
  );

  const handleEndpointModeChange = useCallback(
    (mode: string) => {
      if (mode === 'vip' || mode === 'dnsrr') {
        onChange(mergeDeploy(deploy, { endpoint_mode: mode }));
      } else {
        const { endpoint_mode: _, ...rest } = deploy ?? {};
        onChange(Object.keys(rest).length > 0 ? (rest as DeployDef) : undefined);
      }
    },
    [deploy, onChange],
  );

  // ── Resources ──

  const handleResourcesLimitsChange = useCallback(
    (limits: { cpus?: string; memory?: string; pids?: number } | undefined) => {
      const resources: DeployResourcesDef = { ...(deploy?.resources ?? {}), limits };
      if (!resources.limits && !resources.reservations) {
        const { resources: _, ...rest } = deploy ?? {};
        onChange(Object.keys(rest).length > 0 ? (rest as DeployDef) : undefined);
      } else {
        onChange(mergeDeploy(deploy, { resources }));
      }
    },
    [deploy, onChange],
  );

  const handleResourcesReservationsChange = useCallback(
    (reservations: { cpus?: string; memory?: string } | undefined) => {
      const resources: DeployResourcesDef = { ...(deploy?.resources ?? {}), reservations };
      if (!resources.limits && !resources.reservations) {
        const { resources: _, ...rest } = deploy ?? {};
        onChange(Object.keys(rest).length > 0 ? (rest as DeployDef) : undefined);
      } else {
        onChange(mergeDeploy(deploy, { resources }));
      }
    },
    [deploy, onChange],
  );

  // ── Restart Policy ──

  const handleRestartPolicyChange = useCallback(
    (patch: Partial<DeployRestartPolicyDef>) => {
      const next = mergeNested(deploy?.restart_policy, patch) as DeployRestartPolicyDef;
      const hasAny = next.condition || next.delay || next.max_attempts != null || next.window;
      if (!hasAny) {
        const { restart_policy: _, ...rest } = deploy ?? {};
        onChange(Object.keys(rest).length > 0 ? (rest as DeployDef) : undefined);
      } else {
        onChange(mergeDeploy(deploy, { restart_policy: next }));
      }
    },
    [deploy, onChange],
  );

  // ── Placement ──

  const handlePlacementConstraintsChange = useCallback(
    (constraints: string[] | undefined) => {
      const placement: DeployPlacementDef = { ...(deploy?.placement ?? {}), constraints };
      if (!placement.constraints || placement.constraints.length === 0) {
        const { placement: _, ...rest } = deploy ?? {};
        onChange(Object.keys(rest).length > 0 ? (rest as DeployDef) : undefined);
      } else {
        onChange(mergeDeploy(deploy, { placement }));
      }
    },
    [deploy, onChange],
  );

  // ── Labels ──

  const handleLabelsChange = useCallback(
    (labels: Record<string, string> | undefined) => {
      if (!labels || Object.keys(labels).length === 0) {
        const { labels: _, ...rest } = deploy ?? {};
        onChange(Object.keys(rest).length > 0 ? (rest as DeployDef) : undefined);
      } else {
        onChange(mergeDeploy(deploy, { labels }));
      }
    },
    [deploy, onChange],
  );

  // ── Update Config ──

  const handleUpdateConfigChange = useCallback(
    (patch: Partial<NonNullable<DeployDef['update_config']>>) => {
      const next = { ...(deploy?.update_config ?? {}), ...patch } as NonNullable<DeployDef['update_config']>;
      const hasAny = next.parallelism != null || next.delay || next.failure_action || next.monitor || next.order;
      if (!hasAny) {
        const { update_config: _, ...rest } = deploy ?? {};
        onChange(Object.keys(rest).length > 0 ? (rest as DeployDef) : undefined);
      } else {
        onChange(mergeDeploy(deploy, { update_config: next }));
      }
    },
    [deploy, onChange],
  );

  // ── Remove all ──

  const handleRemove = useCallback(() => {
    onChange(undefined);
  }, [onChange]);

  // ── Collapse default active keys ──

  const defaultActiveKeys = useMemo(() => {
    const keys: string[] = [];
    if (deploy?.mode || deploy?.replicas != null) keys.push('mode');
    if (deploy?.resources) keys.push('resources');
    if (deploy?.restart_policy) keys.push('restart-policy');
    if (deploy?.placement) keys.push('placement');
    if (deploy?.labels && Object.keys(deploy.labels).length > 0) keys.push('labels');
    if (deploy?.update_config) keys.push('update-config');
    if (deploy?.endpoint_mode) keys.push('endpoint-mode');
    return keys;
  }, [deploy]);

  // ── Render ──

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {/* Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Text strong style={{ fontSize: 13 }}>Deploy Configuration</Text>
        {deploy && (
          <Button size="small" danger icon={<DeleteOutlined />} onClick={handleRemove}>
            Remove Deploy
          </Button>
        )}
      </div>

      {!deploy ? (
        <Button
          size="small"
          type="dashed"
          icon={<PlusOutlined />}
          onClick={() => onChange({})}
          style={{ alignSelf: 'flex-start' }}
        >
          Add Deploy Configuration
        </Button>
      ) : (
        <Collapse
          size="small"
          ghost
          defaultActiveKey={defaultActiveKeys}
          style={{ background: 'transparent' }}
        >
          {/* Mode & Replicas */}
          <Panel
            header={<Text style={{ fontSize: 12 }}>Mode &amp; Replicas</Text>}
            key="mode"
            style={{ marginBottom: 0 }}
          >
            <Space direction="vertical" size="small" style={{ width: '100%' }}>
              <Space size="small">
                <div>
                  <Text type="secondary" style={{ fontSize: 11, display: 'block' }}>Mode</Text>
                  <Tooltip title="Replicated (multiple instances) or global (one per node)">
                    <Select
                      size="small"
                      value={deploy.mode ?? undefined}
                      placeholder="Select mode"
                      allowClear
                      onChange={handleModeChange}
                      style={{ width: 140 }}
                      options={[
                        { value: 'replicated', label: 'replicated' },
                        { value: 'global', label: 'global' },
                      ]}
                    />
                  </Tooltip>
                </div>
                {deploy.mode !== 'global' && (
                  <div>
                    <Text type="secondary" style={{ fontSize: 11, display: 'block' }}>Replicas</Text>
                    <Tooltip title="Number of container replicas">
                      <InputNumber
                        size="small"
                        min={1}
                        value={deploy.replicas}
                        onChange={handleReplicasChange}
                        style={{ width: 100 }}
                      />
                    </Tooltip>
                  </div>
                )}
              </Space>
            </Space>
          </Panel>

          {/* Resources */}
          <Panel
            header={<Text style={{ fontSize: 12 }}>Resources</Text>}
            key="resources"
            style={{ marginBottom: 0 }}
          >
            <Space direction="vertical" size="small" style={{ width: '100%' }}>
              <ResourceBlock
                label="Limits"
                value={deploy.resources?.limits}
                onChange={handleResourcesLimitsChange}
                showPids
              />
              <ResourceBlock
                label="Reservations"
                value={deploy.resources?.reservations}
                onChange={handleResourcesReservationsChange}
              />
            </Space>
          </Panel>

          {/* Restart Policy */}
          <Panel
            header={<Text style={{ fontSize: 12 }}>Restart Policy</Text>}
            key="restart-policy"
            style={{ marginBottom: 0 }}
          >
            <Space direction="vertical" size="small" style={{ width: '100%' }}>
              <Space size="small" wrap>
                <div>
                  <Text type="secondary" style={{ fontSize: 11, display: 'block' }}>Condition</Text>
                  <Tooltip title="When to restart (any, on-failure, none)">
                    <Select
                      size="small"
                      value={deploy.restart_policy?.condition}
                      placeholder="Select"
                      allowClear
                      onChange={(v) => handleRestartPolicyChange({ condition: v ?? undefined })}
                      style={{ width: 130 }}
                      options={[
                        { value: 'any', label: 'any' },
                        { value: 'on-failure', label: 'on-failure' },
                        { value: 'none', label: 'none' },
                      ]}
                    />
                  </Tooltip>
                </div>
                <div>
                  <Text type="secondary" style={{ fontSize: 11, display: 'block' }}>Delay</Text>
                  <Tooltip title="Delay between restart attempts (e.g. 5s)">
                    <Input
                      size="small"
                      placeholder="e.g. 5s"
                      value={deploy.restart_policy?.delay ?? ''}
                      onChange={(e) => handleRestartPolicyChange({ delay: e.target.value || undefined })}
                      style={{ width: 100 }}
                    />
                  </Tooltip>
                </div>
                <div>
                  <Text type="secondary" style={{ fontSize: 11, display: 'block' }}>Max Attempts</Text>
                  <Tooltip title="Max restart attempts before giving up">
                    <InputNumber
                      size="small"
                      min={0}
                      value={deploy.restart_policy?.max_attempts}
                      onChange={(v) => handleRestartPolicyChange({ max_attempts: v ?? undefined })}
                      style={{ width: 100 }}
                    />
                  </Tooltip>
                </div>
                <div>
                  <Text type="secondary" style={{ fontSize: 11, display: 'block' }}>Window</Text>
                  <Tooltip title="Time window to evaluate restart (e.g. 120s)">
                    <Input
                      size="small"
                      placeholder="e.g. 30s"
                      value={deploy.restart_policy?.window ?? ''}
                      onChange={(e) => handleRestartPolicyChange({ window: e.target.value || undefined })}
                      style={{ width: 100 }}
                    />
                  </Tooltip>
                </div>
              </Space>
              <Button
                size="small"
                type="link"
                danger
                icon={<DeleteOutlined />}
                onClick={() => {
                  const { restart_policy: _, ...rest } = deploy;
                  onChange(Object.keys(rest).length > 0 ? (rest as DeployDef) : undefined);
                }}
                style={{ padding: 0, fontSize: 11 }}
              >
                Clear Restart Policy
              </Button>
            </Space>
          </Panel>

          {/* Placement */}
          <Panel
            header={<Text style={{ fontSize: 12 }}>Placement</Text>}
            key="placement"
            style={{ marginBottom: 0 }}
          >
            <Space direction="vertical" size="small" style={{ width: '100%' }}>
              <StringListEditor
                value={deploy.placement?.constraints}
                onChange={handlePlacementConstraintsChange}
                placeholder="e.g. node.labels.role==manager"
              />
            </Space>
          </Panel>

          {/* Labels */}
          <Panel
            header={<Text style={{ fontSize: 12 }}>Labels</Text>}
            key="labels"
            style={{ marginBottom: 0 }}
          >
            <LabelsEditor value={deploy.labels} onChange={handleLabelsChange} />
          </Panel>

          {/* Update Config */}
          <Panel
            header={<Text style={{ fontSize: 12 }}>Update Config</Text>}
            key="update-config"
            style={{ marginBottom: 0 }}
          >
            <Space direction="vertical" size="small" style={{ width: '100%' }}>
              <Space size="small" wrap>
                <div>
                  <Text type="secondary" style={{ fontSize: 11, display: 'block' }}>Parallelism</Text>
                  <Tooltip title="Number of containers to update at once">
                    <InputNumber
                      size="small"
                      min={0}
                      value={deploy.update_config?.parallelism}
                      onChange={(v) => handleUpdateConfigChange({ parallelism: v ?? undefined })}
                      style={{ width: 90 }}
                    />
                  </Tooltip>
                </div>
                <div>
                  <Text type="secondary" style={{ fontSize: 11, display: 'block' }}>Delay</Text>
                  <Tooltip title="Delay between updates (e.g. 10s)">
                    <Input
                      size="small"
                      placeholder="e.g. 10s"
                      value={deploy.update_config?.delay ?? ''}
                      onChange={(e) => handleUpdateConfigChange({ delay: e.target.value || undefined })}
                      style={{ width: 100 }}
                    />
                  </Tooltip>
                </div>
                <div>
                  <Text type="secondary" style={{ fontSize: 11, display: 'block' }}>Failure Action</Text>
                  <Tooltip title="Action on update failure (pause, continue, rollback)">
                    <Select
                      size="small"
                      value={deploy.update_config?.failure_action}
                      placeholder="Select"
                      allowClear
                      onChange={(v) => handleUpdateConfigChange({ failure_action: v ?? undefined })}
                      style={{ width: 120 }}
                      options={[
                        { value: 'continue', label: 'continue' },
                        { value: 'pause', label: 'pause' },
                        { value: 'rollback', label: 'rollback' },
                      ]}
                    />
                  </Tooltip>
                </div>
                <div>
                  <Text type="secondary" style={{ fontSize: 11, display: 'block' }}>Monitor</Text>
                  <Input
                    size="small"
                    placeholder="e.g. 10s"
                    value={deploy.update_config?.monitor ?? ''}
                    onChange={(e) => handleUpdateConfigChange({ monitor: e.target.value || undefined })}
                    style={{ width: 100 }}
                  />
                </div>
                <div>
                  <Text type="secondary" style={{ fontSize: 11, display: 'block' }}>Order</Text>
                  <Tooltip title="Update order (start-first or stop-first)">
                    <Select
                      size="small"
                      value={deploy.update_config?.order}
                      placeholder="Select"
                      allowClear
                      onChange={(v) => handleUpdateConfigChange({ order: v ?? undefined })}
                      style={{ width: 130 }}
                      options={[
                        { value: 'start-first', label: 'start-first' },
                        { value: 'stop-first', label: 'stop-first' },
                      ]}
                    />
                  </Tooltip>
                </div>
              </Space>
              <Button
                size="small"
                type="link"
                danger
                icon={<DeleteOutlined />}
                onClick={() => {
                  const { update_config: _, ...rest } = deploy;
                  onChange(Object.keys(rest).length > 0 ? (rest as DeployDef) : undefined);
                }}
                style={{ padding: 0, fontSize: 11 }}
              >
                Clear Update Config
              </Button>
            </Space>
          </Panel>

          {/* Endpoint Mode */}
          <Panel
            header={<Text style={{ fontSize: 12 }}>Endpoint Mode</Text>}
            key="endpoint-mode"
            style={{ marginBottom: 0 }}
          >
            <Space size="small">
              <div>
                <Text type="secondary" style={{ fontSize: 11, display: 'block' }}>Mode</Text>
                <Tooltip title="Service discovery mode (vip or dnsrr)">
                  <Select
                    size="small"
                    value={deploy.endpoint_mode ?? undefined}
                    placeholder="Select mode"
                    allowClear
                    onChange={handleEndpointModeChange}
                    style={{ width: 140 }}
                    options={[
                      { value: 'vip', label: 'vip' },
                      { value: 'dnsrr', label: 'dnsrr' },
                    ]}
                  />
                </Tooltip>
              </div>
            </Space>
          </Panel>
        </Collapse>
      )}
    </div>
  );
}

export default DeployForm;