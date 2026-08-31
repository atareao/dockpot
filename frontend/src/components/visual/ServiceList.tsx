// ── List of all services with add/delete capability ──
// Services are rendered as Ant Design Tabs — one tab per service.

import { useCallback, useState } from 'react';
import { Tabs, Button, Empty, Modal } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import type { ServiceDef } from '../../types/compose';
import { ServiceCard } from './ServiceCard';
import AddServiceModal from './AddServiceModal';

const { confirm } = Modal;

export interface ServiceListProps {
  value: Record<string, ServiceDef>;
  onChange: (v: Record<string, ServiceDef>) => void;
  volumeNames: string[];
  networkNames: string[];
}

function ServiceList({ value, onChange, volumeNames, networkNames }: ServiceListProps) {
  const [modalOpen, setModalOpen] = useState(false);
  const entries = Object.entries(value);
  const [activeKey, setActiveKey] = useState<string | undefined>(
    entries.length > 0 ? entries[0][0] : undefined,
  );

  // ── Handlers ──

  const handleAdd = useCallback(
    (name: string, service: ServiceDef) => {
      onChange({ ...value, [name]: service });
      setActiveKey(name);
      setModalOpen(false);
    },
    [value, onChange],
  );

  const handleUpdate = useCallback(
    (oldName: string, newName: string, updated: ServiceDef) => {
      const next = { ...value };
      if (oldName !== newName) {
        delete next[oldName];
      }
      next[newName] = updated;
      onChange(next);
      // If the name changed, update active tab key
      if (oldName !== newName && activeKey === oldName) {
        setActiveKey(newName);
      }
    },
    [value, onChange, activeKey],
  );

  const handleDelete = useCallback(
    (name: string) => {
      const next = { ...value };
      delete next[name];
      onChange(next);
      // If the deleted tab was active, switch to another
      if (activeKey === name) {
        const remaining = Object.keys(next);
        setActiveKey(remaining.length > 0 ? remaining[0] : undefined);
      }
    },
    [value, onChange, activeKey],
  );

  const handleTabClose = useCallback(
    (targetKey: string) => {
      const targetService = value[targetKey];
      confirm({
        title: `Delete service '${targetKey}'?`,
        content: targetService?.image
          ? `Service uses image: ${targetService.image}`
          : 'This action cannot be undone.',
        okText: 'Delete',
        okType: 'danger',
        onOk: () => handleDelete(targetKey),
      });
    },
    [value, handleDelete],
  );

  // ── Empty state ──

  if (entries.length === 0) {
    return (
      <div style={{ textAlign: 'center', padding: '40px 0' }}>
        <Empty description="No services defined" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        <Button
          size="small"
          icon={<PlusOutlined />}
          onClick={() => setModalOpen(true)}
          style={{ marginTop: 12 }}
        >
          Add Service
        </Button>
        <AddServiceModal
          open={modalOpen}
          onCancel={() => setModalOpen(false)}
          onConfirm={handleAdd}
          existingNames={Object.keys(value)}
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
          <Button
            size="small"
            icon={<PlusOutlined />}
            onClick={() => setModalOpen(true)}
            style={{ marginLeft: 4 }}
          >
            Add
          </Button>
        }
        items={entries.map(([name, service]) => ({
          key: name,
          label: name,
          closable: true,
          children: (
            <ServiceCard
              name={name}
              value={service}
              onChange={(newName, updated) => handleUpdate(name, newName, updated)}
              onDelete={() => handleDelete(name)}
              volumeNames={volumeNames}
              networkNames={networkNames}
            />
          ),
        }))}
        style={{ marginTop: -8 }}
      />

      <AddServiceModal
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onConfirm={handleAdd}
        existingNames={Object.keys(value)}
      />
    </div>
  );
}

export default ServiceList;