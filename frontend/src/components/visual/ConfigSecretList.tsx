import { useCallback, useState } from 'react';
import { Tabs, Input, Switch, Button, Typography, theme, Space, Modal, Tooltip } from 'antd';
import { PlusOutlined, LinkOutlined } from '@ant-design/icons';
import { ConfigDef } from '../../types/compose';
import AddExternalModal from './AddExternalModal';

const { Text } = Typography;
const { confirm } = Modal;

interface ConfigSecretListProps {
  /** Title shown — "Configs" or "Secrets" */
  title: string;
  value: Record<string, ConfigDef>;
  onChange: (v: Record<string, ConfigDef>) => void;
}

/**
 * Editable list of docker-compose top-level config or secret definitions.
 * Each item appears as an editable-card tab.
 */
export default function ConfigSecretList({ title, value, onChange }: ConfigSecretListProps) {
  const { token } = theme.useToken();
  const entries = Object.entries(value);
  const singular = title.endsWith('s') ? title.slice(0, -1) : title;
  const [activeKey, setActiveKey] = useState<string | undefined>(
    entries.length > 0 ? entries[0][0] : undefined,
  );
  const [externalModalOpen, setExternalModalOpen] = useState(false);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [renameDraft, setRenameDraft] = useState('');

  const addItem = useCallback(() => {
    const baseName = singular.toLowerCase();
    let idx = 1;
    while (value[`${baseName}${idx}`]) {
      idx++;
    }
    const key = `${baseName}${idx}`;
    const next = { ...value, [key]: {} };
    onChange(next);
    setActiveKey(key);
  }, [value, onChange, singular]);

  const addExternalItem = useCallback(
    (name: string) => {
      const key = name.replace(/[^a-zA-Z0-9_-]/g, '_');
      const item: ConfigDef = key === name ? { external: true } : { external: { name } };
      const next = { ...value, [key]: item };
      onChange(next);
      setActiveKey(key);
      setExternalModalOpen(false);
    },
    [value, onChange],
  );

  const removeItem = useCallback(
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
        title: `Delete ${singular.toLowerCase()} '${targetKey}'?`,
        okText: 'Delete',
        okType: 'danger',
        onOk: () => removeItem(targetKey),
      });
    },
    [removeItem, singular],
  );

  const updateItem = useCallback(
    (name: string, patch: Partial<ConfigDef>) => {
      onChange({ ...value, [name]: { ...value[name], ...patch } });
    },
    [value, onChange],
  );

  const renameItem = useCallback(
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

  const fetchUrl = title === 'Configs' ? '/api/docker/configs' : '/api/docker/secrets';

  // Empty state
  if (entries.length === 0) {
    return (
      <div style={{ textAlign: 'center', padding: '40px 0' }}>
        <Text style={{ color: token.colorTextQuaternary, fontStyle: 'italic' }}>
          No {title.toLowerCase()} defined.
        </Text>
        <div style={{ marginTop: 12, display: 'flex', gap: 8, justifyContent: 'center' }}>
          <Button size="small" type="primary" icon={<PlusOutlined />} onClick={addItem}>
            Add {singular}
          </Button>
          <Button size="small" icon={<LinkOutlined />} onClick={() => setExternalModalOpen(true)}>
            Add External
          </Button>
        </div>
        <AddExternalModal
          open={externalModalOpen}
          onCancel={() => setExternalModalOpen(false)}
          onConfirm={addExternalItem}
          title={`Add External ${singular}`}
          fetchUrl={fetchUrl}
          existingNames={Object.keys(value)}
          labelKey={singular}
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
            <Button size="small" icon={<PlusOutlined />} onClick={addItem}>
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
                renameItem(name, renameDraft);
                setRenaming(null);
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  renameItem(name, renameDraft);
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
          children: (
            <ConfigSecretForm
              name={name}
              value={value[name]}
              onChange={updateItem}
              singular={singular}
            />
          ),
        }))}
        style={{ marginTop: -4 }}
      />
      <AddExternalModal
        open={externalModalOpen}
        onCancel={() => setExternalModalOpen(false)}
        onConfirm={addExternalItem}
        title={`Add External ${singular}`}
        fetchUrl={fetchUrl}
        existingNames={Object.keys(value)}
        labelKey={singular}
      />
    </div>
  );
}

// ── Config/Secret form (contents of a single tab) ──

function ConfigSecretForm({
  name,
  value: item,
  onChange,
  singular,
}: {
  name: string;
  value: ConfigDef;
  onChange: (name: string, patch: Partial<ConfigDef>) => void;
  singular: string;
}) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10, paddingTop: 8 }}>
      {/* file */}
      <div>
        <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>file</Text>
        <Tooltip title="Path to the file on the host filesystem">
          <Input
            size="small"
            placeholder="./path/to/file"
            value={item.file ?? ''}
            onChange={(e) => onChange(name, { file: e.target.value || undefined })}
          />
        </Tooltip>
      </div>

      {/* external */}
      <div>
        <Space>
          <Tooltip title="Use an externally-created config/secret">
            <Switch
              size="small"
              checked={!!item.external}
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
        {item.external && typeof item.external === 'object' && (
          <div style={{ marginTop: 6 }}>
            <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>external name</Text>
            <Input
              size="small"
              placeholder={`external-${singular.toLowerCase()}-name`}
              value={item.external.name ?? ''}
              onChange={(e) =>
                onChange(name, {
                  external: { name: e.target.value || '' },
                })
              }
            />
          </div>
        )}
        {item.external === true && (
          <Button
            size="small"
            type="link"
            style={{ fontSize: 12, padding: 0, marginTop: 4 }}
            onClick={() => onChange(name, { external: { name } })}
          >
            Customize external name
          </Button>
        )}
        {item.external && typeof item.external === 'object' && !item.external.name && (
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

      {/* name */}
      <div>
        <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>
          {singular} name
        </Text>
        <Tooltip title="Custom name in Docker">
          <Input
            size="small"
            placeholder={`docker-${singular.toLowerCase()}-name`}
            value={item.name ?? ''}
            onChange={(e) => onChange(name, { name: e.target.value || undefined })}
          />
        </Tooltip>
      </div>

      {/* template_driver */}
      <div>
        <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>template_driver</Text>
        <Tooltip title="Template driver (e.g. golang) for Go templates">
          <Input
            size="small"
            placeholder="golang"
            value={item.template_driver ?? ''}
            onChange={(e) => onChange(name, { template_driver: e.target.value || undefined })}
          />
        </Tooltip>
      </div>
    </div>
  );
}