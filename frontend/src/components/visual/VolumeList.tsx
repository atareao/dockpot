import { useCallback, useState } from 'react';
import { Tabs, Input, Select, Switch, Button, Typography, theme, Space, Modal, Tooltip } from 'antd';
import { PlusOutlined, LinkOutlined } from '@ant-design/icons';
import { VolumeDef } from '../../types/compose';
import EnvVarTable from './EnvVarTable';
import AddExternalModal from './AddExternalModal';

const VOLUME_DRIVERS = [
  { value: 'local', label: 'local (default)' },
  { value: 'nfs', label: 'nfs' },
  { value: 'tmpfs', label: 'tmpfs' },
  { value: 'cifs', label: 'cifs' },
  { value: 'azure', label: 'azure' },
  { value: 'gcs', label: 'gcs' },
  { value: 's3', label: 's3' },
  { value: 'rclone', label: 'rclone' },
  { value: 'vieux/sshfs', label: 'vieux/sshfs' },
];

const { Text } = Typography;
const { confirm } = Modal;

interface VolumeListProps {
  value: Record<string, VolumeDef>;
  onChange: (v: Record<string, VolumeDef>) => void;
}

/**
 * Editable list of docker-compose top-level volume definitions.
 * Each volume appears as an editable-card tab.
 */
export default function VolumeList({ value, onChange }: VolumeListProps) {
  const { token } = theme.useToken();
  const entries = Object.entries(value);
  const [activeKey, setActiveKey] = useState<string | undefined>(
    entries.length > 0 ? entries[0][0] : undefined,
  );
  const [externalModalOpen, setExternalModalOpen] = useState(false);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [renameDraft, setRenameDraft] = useState('');

  const addVolume = useCallback(() => {
    const baseName = 'volume';
    let idx = 1;
    while (value[`${baseName}${idx}`]) {
      idx++;
    }
    const key = `${baseName}${idx}`;
    const next = { ...value, [key]: {} };
    onChange(next);
    setActiveKey(key);
  }, [value, onChange]);

  const addExternalVolume = useCallback(
    (name: string) => {
      const key = name.replace(/[^a-zA-Z0-9_-]/g, '_');
      // Use the original name as external name if it differs from key
      const vol: VolumeDef = key === name ? { external: true } : { external: { name } };
      const next = { ...value, [key]: vol };
      onChange(next);
      setActiveKey(key);
      setExternalModalOpen(false);
    },
    [value, onChange],
  );

  const removeVolume = useCallback(
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
        title: `Delete volume '${targetKey}'?`,
        okText: 'Delete',
        okType: 'danger',
        onOk: () => removeVolume(targetKey),
      });
    },
    [removeVolume],
  );

  const updateVolume = useCallback(
    (name: string, patch: Partial<VolumeDef>) => {
      onChange({ ...value, [name]: { ...value[name], ...patch } });
    },
    [value, onChange],
  );

  const renameVolume = useCallback(
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
          No volumes defined.
        </Text>
        <div style={{ marginTop: 12, display: 'flex', gap: 8, justifyContent: 'center' }}>
          <Button size="small" type="primary" icon={<PlusOutlined />} onClick={addVolume}>
            Add Volume
          </Button>
          <Button size="small" icon={<LinkOutlined />} onClick={() => setExternalModalOpen(true)}>
            Add External
          </Button>
        </div>
        <AddExternalModal
          open={externalModalOpen}
          onCancel={() => setExternalModalOpen(false)}
          onConfirm={addExternalVolume}
          title="Add External Volume"
          fetchUrl="/api/docker/volumes"
          existingNames={Object.keys(value)}
          labelKey="Volume"
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
            <Button size="small" icon={<PlusOutlined />} onClick={addVolume}>
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
                renameVolume(name, renameDraft);
                setRenaming(null);
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  renameVolume(name, renameDraft);
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
          children: <VolumeForm name={name} value={value[name]} onChange={updateVolume} />,
        }))}
        style={{ marginTop: -4 }}
      />
      <AddExternalModal
        open={externalModalOpen}
        onCancel={() => setExternalModalOpen(false)}
        onConfirm={addExternalVolume}
        title="Add External Volume"
        fetchUrl="/api/docker/volumes"
        existingNames={Object.keys(value)}
        labelKey="Volume"
      />
    </div>
  );
}

// ── Volume form (contents of a single tab) ──

function VolumeForm({
  name,
  value: vol,
  onChange,
}: {
  name: string;
  value: VolumeDef;
  onChange: (name: string, patch: Partial<VolumeDef>) => void;
}) {

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10, paddingTop: 8 }}>
      {/* driver */}
      <div>
        <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>driver</Text>
        <Tooltip title="Volume driver (local is the default, nfs for network storage)">
          <Select
            size="small"
            allowClear
            placeholder="local"
            value={vol.driver ?? undefined}
            onChange={(v) => onChange(name, { driver: v || undefined })}
            style={{ width: '100%' }}
            options={VOLUME_DRIVERS}
          />
        </Tooltip>
      </div>

      {/* driver_opts */}
      <EnvVarTable
        value={vol.driver_opts ?? {}}
        onChange={(v) => onChange(name, { driver_opts: Object.keys(v).length > 0 ? v : undefined })}
        title="driver_opts"
        keyPlaceholder="OPTION"
        valuePlaceholder="VALUE"
      />

      {/* external */}
      <div>
        <Space>
          <Tooltip title="Use an externally-created volume">
            <Switch
              size="small"
              checked={!!vol.external}
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
        {vol.external && typeof vol.external === 'object' && (
          <div style={{ marginTop: 6 }}>
            <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>external name</Text>
            <Tooltip title="Name of the external Docker volume">
              <Input
                size="small"
                placeholder="external-volume-name"
                value={vol.external.name ?? ''}
                onChange={(e) =>
                  onChange(name, {
                    external: { name: e.target.value || '' },
                  })
                }
              />
            </Tooltip>
          </div>
        )}
        {vol.external === true && (
          <Button
            size="small"
            type="link"
            style={{ fontSize: 12, padding: 0, marginTop: 4 }}
            onClick={() => onChange(name, { external: { name: name } })}
          >
            Customize external name
          </Button>
        )}
        {vol.external && typeof vol.external === 'object' && !vol.external.name && (
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
        value={vol.labels ?? {}}
        onChange={(v) => onChange(name, { labels: Object.keys(v).length > 0 ? v : undefined })}
        title="labels"
        keyPlaceholder="LABEL"
        valuePlaceholder="VALUE"
      />

      {/* name */}
      <div>
        <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>name</Text>
        <Tooltip title="Custom volume name in Docker (defaults to the key)">
          <Input
            size="small"
            placeholder="docker-volume-name"
            value={vol.name ?? ''}
            onChange={(e) => onChange(name, { name: e.target.value || undefined })}
          />
        </Tooltip>
      </div>
    </div>
  );
}