import { useCallback, useState } from 'react';
import { Tabs, Input, Select, Switch, Button, Typography, theme, Space, Modal, Tooltip } from 'antd';
import { PlusOutlined, LinkOutlined, DeleteOutlined } from '@ant-design/icons';
import { NetworkDef, IpamConfigDef } from '../../types/compose';
import EnvVarTable from './EnvVarTable';
import AddExternalModal from './AddExternalModal';

const NETWORK_DRIVERS = [
  { value: 'bridge', label: 'bridge (default)' },
  { value: 'host', label: 'host' },
  { value: 'overlay', label: 'overlay' },
  { value: 'macvlan', label: 'macvlan' },
  { value: 'ipvlan', label: 'ipvlan' },
  { value: 'none', label: 'none' },
];

const { Text } = Typography;
const { confirm } = Modal;

interface NetworkListProps {
  value: Record<string, NetworkDef>;
  onChange: (v: Record<string, NetworkDef>) => void;
}

/**
 * Editable list of docker-compose top-level network definitions.
 * Each network appears as an editable-card tab.
 */
export default function NetworkList({ value, onChange }: NetworkListProps) {
  const { token } = theme.useToken();
  const entries = Object.entries(value);
  const [activeKey, setActiveKey] = useState<string | undefined>(
    entries.length > 0 ? entries[0][0] : undefined,
  );
  const [externalModalOpen, setExternalModalOpen] = useState(false);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [renameDraft, setRenameDraft] = useState('');

  const addNetwork = useCallback(() => {
    const baseName = 'network';
    let idx = 1;
    while (value[`${baseName}${idx}`]) {
      idx++;
    }
    const key = `${baseName}${idx}`;
    const next = { ...value, [key]: {} };
    onChange(next);
    setActiveKey(key);
  }, [value, onChange]);

  const addExternalNetwork = useCallback(
    (name: string) => {
      const key = name.replace(/[^a-zA-Z0-9_-]/g, '_');
      const net: NetworkDef = key === name ? { external: true } : { external: { name } };
      const next = { ...value, [key]: net };
      onChange(next);
      setActiveKey(key);
      setExternalModalOpen(false);
    },
    [value, onChange],
  );

  const removeNetwork = useCallback(
    (name: string) => {
      const next = { ...value };
      delete next[name];
      onChange(next);
      if (activeKey === name) {
        const remaining = Object.keys(next);
        setActiveKey(remaining.length > 0 ? remaining[0] : undefined);
      }
    },
    [value, onChange, activeKey],
  );

  const handleTabClose = useCallback(
    (targetKey: string) => {
      confirm({
        title: `Delete network '${targetKey}'?`,
        okText: 'Delete',
        okType: 'danger',
        onOk: () => removeNetwork(targetKey),
      });
    },
    [removeNetwork],
  );

  const updateNetwork = useCallback(
    (name: string, patch: Partial<NetworkDef>) => {
      onChange({ ...value, [name]: { ...value[name], ...patch } });
    },
    [value, onChange],
  );

  const renameNetwork = useCallback(
    (oldName: string, newName: string) => {
      if (!newName.trim() || newName === oldName) return;
      const next = { ...value };
      next[newName] = next[oldName];
      delete next[oldName];
      onChange(next);
      if (activeKey === oldName) setActiveKey(newName);
    },
    [value, onChange, activeKey],
  );

  // Empty state
  if (entries.length === 0) {
    return (
      <div style={{ textAlign: 'center', padding: '40px 0' }}>
        <Text style={{ color: token.colorTextQuaternary, fontStyle: 'italic' }}>
          No networks defined.
        </Text>
        <div style={{ marginTop: 12, display: 'flex', gap: 8, justifyContent: 'center' }}>
          <Button size="small" type="primary" icon={<PlusOutlined />} onClick={addNetwork}>
            Add Network
          </Button>
          <Button size="small" icon={<LinkOutlined />} onClick={() => setExternalModalOpen(true)}>
            Add External
          </Button>
        </div>
        <AddExternalModal
          open={externalModalOpen}
          onCancel={() => setExternalModalOpen(false)}
          onConfirm={addExternalNetwork}
          title="Add External Network"
          fetchUrl="/api/docker/networks"
          existingNames={Object.keys(value)}
          labelKey="Network"
        />
      </div>
    );
  }

  return (
    <div>
      <Tabs
        type="editable-card"
        size="small"
        activeKey={activeKey}
        onChange={setActiveKey}
        onEdit={(targetKey, action) => {
          if (action === 'remove' && typeof targetKey === 'string') {
            handleTabClose(targetKey);
          }
        }}
        tabBarExtraContent={
          <Space size={4}>
            <Button size="small" icon={<LinkOutlined />} onClick={() => setExternalModalOpen(true)}>
              External
            </Button>
            <Button size="small" icon={<PlusOutlined />} onClick={addNetwork}>
              Add
            </Button>
          </Space>
        }
        items={entries.map(([name]) => ({
          key: name,
          label: renaming === name ? (
            <Input
              size="small"
              value={renameDraft}
              onChange={(e) => setRenameDraft(e.target.value)}
              onBlur={() => {
                renameNetwork(name, renameDraft);
                setRenaming(null);
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  renameNetwork(name, renameDraft);
                  setRenaming(null);
                }
                if (e.key === 'Escape') setRenaming(null);
              }}
              onClick={(e) => e.stopPropagation()}
              style={{ width: 120 }}
              autoFocus
            />
          ) : (
            <span
              onClick={(e) => {
                e.stopPropagation();
                setRenameDraft(name);
                setRenaming(name);
              }}
              style={{ cursor: 'pointer' }}
            >
              {name}
            </span>
          ),
          closable: true,
          children: <NetworkForm name={name} value={value[name]} onChange={updateNetwork} />,
        }))}
        style={{ marginTop: -4 }}
      />
      <AddExternalModal
        open={externalModalOpen}
        onCancel={() => setExternalModalOpen(false)}
        onConfirm={addExternalNetwork}
        title="Add External Network"
        fetchUrl="/api/docker/networks"
        existingNames={Object.keys(value)}
        labelKey="Network"
      />
    </div>
  );
}

// ── Network form (contents of a single tab) ──

function NetworkForm({
  name,
  value: net,
  onChange,
}: {
  name: string;
  value: NetworkDef;
  onChange: (name: string, patch: Partial<NetworkDef>) => void;
}) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10, paddingTop: 8 }}>
      {/* driver */}
      <div>
        <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>driver</Text>
        <Tooltip title="Network driver (bridge is default, host shares host network)">
          <Select
            size="small"
            allowClear
            placeholder="bridge"
            value={net.driver ?? undefined}
            onChange={(v) => onChange(name, { driver: v || undefined })}
            style={{ width: '100%' }}
            options={NETWORK_DRIVERS}
          />
        </Tooltip>
      </div>

      {/* driver_opts */}
      <EnvVarTable
        value={net.driver_opts ?? {}}
        onChange={(v) =>
          onChange(name, {
            driver_opts: Object.keys(v).length > 0 ? v : undefined,
          })
        }
        title="driver_opts"
        keyPlaceholder="OPTION"
        valuePlaceholder="VALUE"
      />

      {/* ipam */}
      <IpamSection
        value={net.ipam}
        onChange={(ipam) => onChange(name, { ipam })}
      />

      {/* external */}
      <div>
        <Space>
          <Tooltip title="Use an externally-created network">
            <Switch
              size="small"
              checked={!!net.external}
              onChange={(checked) => {
                if (!checked) {
                  onChange(name, { external: undefined });
                } else {
                  onChange(name, { external: true });
                }
              }}
            />
          </Tooltip>
          <Text style={{ fontSize: 12 }}>external</Text>
        </Space>
        {net.external && typeof net.external === 'object' && (
          <div style={{ marginTop: 6 }}>
            <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>external name</Text>
            <Tooltip title="Name of the external Docker network">
              <Input
                size="small"
                placeholder="external-network-name"
                value={net.external.name ?? ''}
                onChange={(e) =>
                  onChange(name, {
                    external: { name: e.target.value || '' },
                  })
                }
              />
            </Tooltip>
          </div>
        )}
        {net.external === true && (
          <Button
            size="small"
            type="link"
            style={{ fontSize: 12, padding: 0, marginTop: 4 }}
            onClick={() => onChange(name, { external: { name } })}
          >
            Customize external name
          </Button>
        )}
        {net.external && typeof net.external === 'object' && !net.external.name && (
          <Button
            size="small"
            type="link"
            style={{ fontSize: 12, padding: 0, marginTop: 4 }}
            onClick={() => onChange(name, { external: true })}
          >
            Use simple external
          </Button>
        )}
      </div>

      {/* labels */}
      <EnvVarTable
        value={net.labels ?? {}}
        onChange={(v) =>
          onChange(name, {
            labels: Object.keys(v).length > 0 ? v : undefined,
          })
        }
        title="labels"
        keyPlaceholder="LABEL"
        valuePlaceholder="VALUE"
      />

      {/* name */}
      <div>
        <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>name</Text>
        <Tooltip title="Custom network name in Docker">
          <Input
            size="small"
            placeholder="docker-network-name"
            value={net.name ?? ''}
            onChange={(e) => onChange(name, { name: e.target.value || undefined })}
          />
        </Tooltip>
      </div>

      {/* Toggle switches row */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
        <Space>
          <Tooltip title="Enable IPv6 on this network">
            <Switch
              size="small"
              checked={!!net.enable_ipv6}
              onChange={(checked) => onChange(name, { enable_ipv6: checked || undefined })}
            />
          </Tooltip>
          <Text style={{ fontSize: 12 }}>enable_ipv6</Text>
        </Space>
        <Space>
          <Tooltip title="Restrict external access to the network">
            <Switch
              size="small"
              checked={!!net.internal}
              onChange={(checked) => onChange(name, { internal: checked || undefined })}
            />
          </Tooltip>
          <Text style={{ fontSize: 12 }}>internal</Text>
        </Space>
        <Space>
          <Tooltip title="Allow standalone containers to attach (Swarm)">
            <Switch
              size="small"
              checked={!!net.attachable}
              onChange={(checked) => onChange(name, { attachable: checked || undefined })}
            />
          </Tooltip>
          <Text style={{ fontSize: 12 }}>attachable</Text>
        </Space>
      </div>
    </div>
  );
}

// ── IpamSection sub-component ──

interface IpamSectionProps {
  value?: NetworkDef['ipam'];
  onChange: (v: NetworkDef['ipam'] | undefined) => void;
}

function IpamSection({ value, onChange }: IpamSectionProps) {
  const { token } = theme.useToken();

  const hasContent =
    !!value?.driver ||
    (value?.config && value.config.length > 0) ||
    (value?.options && Object.keys(value.options).length > 0);

  if (!hasContent) {
    return (
      <Tooltip title="IP Address Management configuration">
        <Button
          size="small"
          type="dashed"
          icon={<PlusOutlined />}
          onClick={() => onChange({ driver: '', config: [], options: {} })}
          block
        >
          Add IPAM Configuration
        </Button>
      </Tooltip>
    );
  }

  const updateIpam = (patch: Partial<NonNullable<NetworkDef['ipam']>>) => {
    onChange({ ...(value ?? { driver: '', config: [], options: {} }), ...patch });
  };

  const addConfig = () => {
    const current = value?.config ?? [];
    updateIpam({ config: [...current, {}] });
  };

  const updateIpamConfig = (index: number, patch: Partial<IpamConfigDef>) => {
    const current = [...(value?.config ?? [])];
    current[index] = { ...current[index], ...patch };
    updateIpam({ config: current });
  };

  const removeConfig = (index: number) => {
    const current = [...(value?.config ?? [])];
    current.splice(index, 1);
    updateIpam({ config: current.length > 0 ? current : undefined });
  };

  return (
    <div
      style={{
        padding: 10,
        border: `1px solid ${token.colorBorderSecondary}`,
        borderRadius: 6,
        display: 'flex',
        flexDirection: 'column',
        gap: 8,
      }}
    >
      <Space style={{ justifyContent: 'space-between', width: '100%' }}>
        <Text strong style={{ fontSize: 12 }}>
          IPAM
        </Text>
        <Button
          size="small"
          danger
          type="text"
          icon={<DeleteOutlined />}
          onClick={() => onChange(undefined)}
        >
          Remove IPAM
        </Button>
      </Space>

      {/* ipam driver */}
      <div>
        <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>driver</Text>
        <Tooltip title="IPAM driver (default is the Docker default)">
          <Input
            size="small"
            placeholder="default"
            value={value?.driver ?? ''}
            onChange={(e) => updateIpam({ driver: e.target.value || undefined })}
          />
        </Tooltip>
      </div>

      {/* ipam config list */}
      <div>
        <Text strong style={{ fontSize: 12, display: 'block', marginBottom: 4 }}>
          IPAM Config
        </Text>
        {(value?.config ?? []).length === 0 ? (
          <Button size="small" type="dashed" icon={<PlusOutlined />} onClick={addConfig} block>
            Add Config
          </Button>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {(value?.config ?? []).map((cfg, idx) => (
              <div
                key={idx}
                style={{
                  padding: 8,
                  border: `1px solid ${token.colorBorderSecondary}`,
                  borderRadius: 4,
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 6,
                }}
              >
                <Space style={{ justifyContent: 'space-between', width: '100%' }}>
                  <Text style={{ fontSize: 11, color: token.colorTextSecondary }}>
                    Config #{idx + 1}
                  </Text>
                  <Button
                    size="small"
                    danger
                    type="text"
                    icon={<DeleteOutlined />}
                    onClick={() => removeConfig(idx)}
                  />
                </Space>
                <div>
                  <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>subnet</Text>
                  <Tooltip title="Subnet CIDR (e.g. 172.20.0.0/16)">
                    <Input
                      size="small"
                      placeholder="172.20.0.0/16"
                      value={cfg.subnet ?? ''}
                      onChange={(e) => updateIpamConfig(idx, { subnet: e.target.value || undefined })}
                    />
                  </Tooltip>
                </div>
                <div>
                  <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>gateway</Text>
                  <Tooltip title="Gateway IP for the subnet">
                    <Input
                      size="small"
                      placeholder="172.20.0.1"
                      value={cfg.gateway ?? ''}
                      onChange={(e) => updateIpamConfig(idx, { gateway: e.target.value || undefined })}
                    />
                  </Tooltip>
                </div>
                <div>
                  <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>ip_range</Text>
                  <Tooltip title="IP range for containers (subset of subnet)">
                    <Input
                      size="small"
                      placeholder="172.20.0.0/24"
                      value={cfg.ip_range ?? ''}
                      onChange={(e) => updateIpamConfig(idx, { ip_range: e.target.value || undefined })}
                    />
                  </Tooltip>
                </div>
              </div>
            ))}
            <Button size="small" type="dashed" icon={<PlusOutlined />} onClick={addConfig} block>
              Add Config
            </Button>
          </div>
        )}
      </div>

      {/* ipam options */}
      <EnvVarTable
        value={value?.options ?? {}}
        onChange={(v) => updateIpam({ options: Object.keys(v).length > 0 ? v : undefined })}
        title="IPAM options"
        keyPlaceholder="OPTION"
        valuePlaceholder="VALUE"
      />
    </div>
  );
}