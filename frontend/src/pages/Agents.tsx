import { useState, useEffect } from 'react';
import {
  Typography, Spin, Button, Space, Tag, Card, Modal, Form, Input, App as AntApp, Layout, Table,
} from 'antd';
import { PlusOutlined, ReloadOutlined, DeleteOutlined, EditOutlined } from '@ant-design/icons';
import { api, Agent } from '../api/http';

const { Title } = Typography;
const { Content, Header } = Layout;

export function Agents() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form] = Form.useForm();
  const { message } = AntApp.useApp();

  const loadAgents = async () => {
    try {
      setLoading(true);
      const data = await api.listAgents();
      setAgents(data);
    } catch (e: any) {
      message.error('Error: ' + e.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadAgents(); }, []);

  const handleOpenCreate = () => {
    setEditingId(null);
    form.resetFields();
    setModalOpen(true);
  };

  const handleOpenEdit = (agent: Agent) => {
    setEditingId(agent.id);
    form.setFieldsValue({
      name: agent.name,
      host: agent.host,
      port: agent.port,
      description: agent.description,
    });
    setModalOpen(true);
  };

  const handleSave = async (values: { name: string; host: string; port?: number; description?: string }) => {
    try {
      if (editingId) {
        await api.updateAgent(editingId, values);
        message.success('Agent updated');
      } else {
        await api.createAgent(values);
        message.success('Agent created');
      }
      setModalOpen(false);
      loadAgents();
    } catch (e: any) {
      message.error('Error: ' + e.message);
    }
  };

  const handleDelete = (id: string, name: string) => {
    Modal.confirm({
      title: `Delete agent '${name}'?`,
      okText: 'Delete',
      okType: 'danger',
      onOk: async () => {
        try {
          await api.deleteAgent(id);
          message.success('Agent deleted');
          loadAgents();
        } catch (e: any) {
          message.error('Error: ' + e.message);
        }
      },
    });
  };

  const columns = [
    { title: 'Name', dataIndex: 'name', key: 'name' },
    { title: 'Host', dataIndex: 'host', key: 'host' },
    {
      title: 'Port', dataIndex: 'port', key: 'port',
      render: (p: number) => <Tag>{p}</Tag>,
    },
    {
      title: 'TLS', dataIndex: 'tls_enabled', key: 'tls',
      render: (tls: boolean) => <Tag color={tls ? 'green' : 'orange'}>{tls ? 'TLS' : 'No TLS'}</Tag>,
    },
    {
      title: 'Enabled', dataIndex: 'enabled', key: 'enabled',
      render: (e: boolean) => <Tag color={e ? 'green' : 'red'}>{e ? 'Active' : 'Disabled'}</Tag>,
    },
    {
      title: 'Actions', key: 'actions',
      render: (_: any, record: Agent) => (
        <Space>
          <Button size="small" icon={<EditOutlined />} onClick={() => handleOpenEdit(record)}>Edit</Button>
          <Button size="small" danger icon={<DeleteOutlined />} onClick={() => handleDelete(record.id, record.name)}>Delete</Button>
        </Space>
      ),
    },
  ];

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Header style={{ background: '#fff', padding: '0 24px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Title level={3} style={{ margin: 0 }}>🔌 Agents</Title>
        <Space>
          <Button icon={<ReloadOutlined />} onClick={loadAgents}>Refresh</Button>
          <Button type="primary" icon={<PlusOutlined />} onClick={handleOpenCreate}>Add Agent</Button>
        </Space>
      </Header>
      <Content style={{ padding: 24 }}>
        <Card>
          {loading ? (
            <Spin size="large" style={{ display: 'block', margin: '40px auto' }} />
          ) : (
            <Table dataSource={agents} columns={columns} rowKey="id" pagination={false} />
          )}
        </Card>
      </Content>

      <Modal
        title={editingId ? 'Edit Agent' : 'Add Agent'}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={() => form.submit()}
      >
        <Form form={form} layout="vertical" onFinish={handleSave}>
          <Form.Item name="name" label="Name" rules={[{ required: true }]}>
            <Input placeholder="my-docker-host" />
          </Form.Item>
          <Form.Item name="host" label="Host" rules={[{ required: true }]}>
            <Input placeholder="192.168.1.100" />
          </Form.Item>
          <Form.Item name="port" label="Port" initialValue={2376}>
            <Input type="number" />
          </Form.Item>
          <Form.Item name="description" label="Description">
            <Input.TextArea rows={2} />
          </Form.Item>
        </Form>
      </Modal>
    </Layout>
  );
}