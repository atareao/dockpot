import { useState, useEffect } from 'react';
import {
  Typography, Tabs, Card, Table, Button, Modal, Form, Input, Select, Switch,
  Space, Tag, Spin, App as AntApp,
} from 'antd';
import {
  PlusOutlined, ReloadOutlined, EditOutlined, DeleteOutlined, PlayCircleOutlined,
} from '@ant-design/icons';
import { api, Notifier } from '../api/http';
import { useTheme } from '../main';

const { Title, Text } = Typography;

function formatConfig(configJson: string): string {
  try {
    const parsed = JSON.parse(configJson);
    return JSON.stringify(parsed, null, 2);
  } catch {
    return configJson;
  }
}

function NotifiersTab() {
  const [notifiers, setNotifiers] = useState<Notifier[]>([]);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [testingId, setTestingId] = useState<string | null>(null);
  const [form] = Form.useForm();
  const { message } = AntApp.useApp();

  const loadNotifiers = async () => {
    try {
      setLoading(true);
      const data = await api.listNotifiers();
      setNotifiers(data);
    } catch (e: any) {
      message.error('Error loading notifiers: ' + e.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadNotifiers(); }, []);

  const handleOpenCreate = () => {
    setEditingId(null);
    form.resetFields();
    form.setFieldsValue({ enabled: true });
    setModalOpen(true);
  };

  const handleOpenEdit = (notifier: Notifier) => {
    setEditingId(notifier.id);
    form.setFieldsValue({
      name: notifier.name,
      notifier_type: notifier.notifier_type,
      config_json: formatConfig(notifier.config_json),
      enabled: notifier.enabled,
    });
    setModalOpen(true);
  };

  const handleSave = async (values: any) => {
    try {
      const payload = {
        name: values.name,
        notifier_type: values.notifier_type,
        config_json: values.config_json,
        enabled: values.enabled,
      };
      if (editingId) {
        await api.updateNotifier(editingId, payload);
        message.success('Notifier updated');
      } else {
        await api.createNotifier(payload);
        message.success('Notifier created');
      }
      setModalOpen(false);
      loadNotifiers();
    } catch (e: any) {
      message.error('Error: ' + e.message);
    }
  };

  const handleDelete = (id: string, name: string) => {
    Modal.confirm({
      title: `Delete notifier '${name}'?`,
      okText: 'Delete',
      okType: 'danger',
      onOk: async () => {
        try {
          await api.deleteNotifier(id);
          message.success('Notifier deleted');
          loadNotifiers();
        } catch (e: any) {
          message.error('Error: ' + e.message);
        }
      },
    });
  };

  const handleTest = async (id: string) => {
    try {
      setTestingId(id);
      const result = await api.testNotifier(id);
      if (result.status === 'success') {
        message.success('✅ Test notification sent successfully!');
      } else {
        message.warning('⚠️ ' + (result.message || 'Test completed with issues'));
      }
    } catch (e: any) {
      message.error('Test failed: ' + e.message);
    } finally {
      setTestingId(null);
    }
  };

  const columns = [
    { title: 'Name', dataIndex: 'name', key: 'name' },
    {
      title: 'Type', dataIndex: 'notifier_type', key: 'notifier_type',
      render: (type: string) => (
        <Tag color={type === 'telegram' ? 'blue' : type === 'ntfy' ? 'purple' : 'default'}>{type}</Tag>
      ),
    },
    {
      title: 'Enabled', dataIndex: 'enabled', key: 'enabled',
      render: (enabled: boolean) => <Tag color={enabled ? 'green' : 'red'}>{enabled ? 'Active' : 'Disabled'}</Tag>,
    },
    { title: 'Created', dataIndex: 'created_at', key: 'created_at', render: (date: string) => new Date(date).toLocaleDateString() },
    {
      title: 'Actions', key: 'actions',
      render: (_: any, record: Notifier) => (
        <Space>
          <Button size="small" icon={<EditOutlined />} onClick={() => handleOpenEdit(record)}>Edit</Button>
          <Button size="small" icon={<PlayCircleOutlined />} onClick={() => handleTest(record.id)} loading={testingId === record.id}>Test</Button>
          <Button size="small" danger icon={<DeleteOutlined />} onClick={() => handleDelete(record.id, record.name)}>Delete</Button>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
        <Text strong style={{ fontSize: 15 }}>Configure notification channels for stack events</Text>
        <Space>
          <Button size="small" icon={<ReloadOutlined />} onClick={loadNotifiers}>Refresh</Button>
          <Button size="small" type="primary" icon={<PlusOutlined />} onClick={handleOpenCreate}>Add Notifier</Button>
        </Space>
      </div>

      {loading ? (
        <Spin style={{ display: 'block', margin: '40px auto' }} />
      ) : (
        <Table dataSource={notifiers} columns={columns} rowKey="id" pagination={false} size="small" />
      )}

      <Modal title={editingId ? 'Edit Notifier' : 'Add Notifier'} open={modalOpen} onCancel={() => setModalOpen(false)} onOk={() => form.submit()} width={600}>
        <Form form={form} layout="vertical" onFinish={handleSave}>
          <Form.Item name="name" label="Name" rules={[{ required: true, message: 'Name is required' }]}>
            <Input placeholder="my-telegram-bot" />
          </Form.Item>
          <Form.Item name="notifier_type" label="Type" rules={[{ required: true }]}>
            <Select placeholder="Select notifier type">
              <Select.Option value="telegram">Telegram</Select.Option>
              <Select.Option value="ntfy">ntfy</Select.Option>
              <Select.Option value="webhook">Webhook</Select.Option>
            </Select>
          </Form.Item>
          <Form.Item name="config_json" label="Configuration (JSON)" rules={[{ required: true, message: 'Configuration is required' }]}>
            <Input.TextArea rows={6} placeholder='{"bot_token": "...", "chat_id": "..."}' style={{ fontFamily: 'monospace', fontSize: 12 }} />
          </Form.Item>
          <Form.Item name="enabled" label="Enabled" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}

export function Settings() {
  const { darkMode } = useTheme();

  return (
    <div>
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: '8px 12px', borderBottom: `1px solid ${darkMode ? '#303030' : '#f0f0f0'}`,
      }}>
        <Title level={4} style={{ margin: 0 }}>⚙️ Settings</Title>
      </div>
      <div style={{ padding: 12 }}>
        <Card styles={{ body: { padding: 12 } }}>
          <Tabs
            defaultActiveKey="notifiers"
            size="small"
            items={[
              {
                key: 'notifiers',
                label: '🔔 Notifiers',
                children: <NotifiersTab />,
              },
            ]}
          />
        </Card>
      </div>
    </div>
  );
}