import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Table, Button, Modal, Form, Input, App as AntApp, Typography, Space, Tag, Card, Spin, Layout, Alert,
} from 'antd';
import { PlusOutlined, PlayCircleOutlined, StopOutlined, ReloadOutlined, SwapOutlined, FileAddOutlined } from '@ant-design/icons';
import { api, Stack } from '../api/http';
import { TemplateBrowser } from '../components/TemplateBrowser';

const { Title } = Typography;
const { Content, Header } = Layout;

export function Stacks() {
  const [stacks, setStacks] = useState<Stack[]>([]);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [convertModalOpen, setConvertModalOpen] = useState(false);
  const [dockerRunCmd, setDockerRunCmd] = useState('');
  const [convertedCompose, setConvertedCompose] = useState('');
  const [convertError, setConvertError] = useState('');
  const [convertLoading, setConvertLoading] = useState(false);
  const [templateModalOpen, setTemplateModalOpen] = useState(false);
  const [form] = Form.useForm();
  const { message } = AntApp.useApp();
  const navigate = useNavigate();

  const loadStacks = async () => {
    try { setLoading(true); setStacks(await api.listStacks()); }
    catch (e: any) { message.error('Error loading stacks: ' + e.message); }
    finally { setLoading(false); }
  };

  useEffect(() => { loadStacks(); }, []);

  const handleCreate = async (values: { name: string; description?: string }) => {
    try {
      await api.createStack(values);
      message.success(`Stack '${values.name}' created`);
      setModalOpen(false);
      form.resetFields();
      loadStacks();
    } catch (e: any) { message.error('Error: ' + e.message); }
  };

  const handleAction = async (id: string, action: 'start' | 'stop' | 'restart') => {
    try {
      if (action === 'start') await api.startStack(id);
      else if (action === 'stop') await api.stopStack(id);
      else await api.restartStack(id);
      loadStacks();
    } catch (e: any) { message.error('Error: ' + e.message); }
  };

  const handleConvert = async () => {
    if (!dockerRunCmd.trim()) { message.error('Enter a docker run command'); return; }
    try {
      setConvertLoading(true);
      const result = await api.convertDockerRun(dockerRunCmd);
      if (result.valid && result.compose) {
        setConvertedCompose(result.compose);
        setConvertError('');
      } else {
        setConvertedCompose('');
        setConvertError(result.error || 'Conversion failed');
      }
    } catch (e: any) { setConvertError(e.message); }
    finally { setConvertLoading(false); }
  };

  const handleSaveFromConvert = async () => {
    const name = prompt('Stack name:');
    if (!name || !name.trim()) return;
    try {
      await api.createStack({ name: name.trim(), compose: convertedCompose });
      message.success("Stack '" + name + "' created from docker run");
      setConvertModalOpen(false);
      setDockerRunCmd('');
      setConvertedCompose('');
      loadStacks();
    } catch (e: any) { message.error('Error: ' + e.message); }
  };

  const handleDelete = (id: string, name: string) => {
    Modal.confirm({
      title: `Delete '${name}'?`,
      content: 'This will remove the stack and ALL its files. Are you sure?',
      okText: 'Delete', okType: 'danger',
      onOk: async () => { try { await api.deleteStack(id); message.success(`Stack '${name}' deleted`); loadStacks(); } catch (e: any) { message.error('Error: ' + e.message); } },
    });
  };

  const columns = [
    { title: 'Name', dataIndex: 'name', key: 'name', render: (name: string, record: Stack) => (<a onClick={() => navigate(`/stacks/${record.id}`)}>{name}</a>) },
    { title: 'Status', dataIndex: 'status', key: 'status', render: (status: string) => (<Tag color={status === 'running' ? 'green' : status === 'error' ? 'red' : 'default'}>{status}</Tag>) },
    { title: 'Description', dataIndex: 'description', key: 'description', ellipsis: true },
    { title: 'Actions', key: 'actions', render: (_: any, record: Stack) => (
      <Space>
        <Button size="small" icon={<PlayCircleOutlined />} onClick={() => handleAction(record.id, 'start')} disabled={record.status === 'running'}>Start</Button>
        <Button size="small" icon={<StopOutlined />} onClick={() => handleAction(record.id, 'stop')} disabled={record.status !== 'running'}>Stop</Button>
        <Button size="small" icon={<ReloadOutlined />} onClick={() => handleAction(record.id, 'restart')} disabled={record.status !== 'running'}>Restart</Button>
        <Button size="small" danger onClick={() => handleDelete(record.id, record.name)}>Delete</Button>
      </Space>
    )},
  ];

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Header style={{ background: '#fff', padding: '0 24px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Title level={3} style={{ margin: 0 }}>🐳 Dockpot</Title>
        <Space>
          <Button icon={<FileAddOutlined />} onClick={() => setTemplateModalOpen(true)}>Templates</Button>
          <Button icon={<SwapOutlined />} onClick={() => setConvertModalOpen(true)}>Convert docker run</Button>
          <Button icon={<ReloadOutlined />} onClick={loadStacks}>Refresh</Button>
          <Button type="primary" icon={<PlusOutlined />} onClick={() => setModalOpen(true)}>New Stack</Button>
        </Space>
      </Header>
      <Content style={{ padding: 24 }}>
        <Card>{loading ? <Spin size="large" style={{ display: 'block', margin: '40px auto' }} /> : <Table dataSource={stacks} columns={columns} rowKey="id" pagination={{ pageSize: 20, showSizeChanger: true, showTotal: (total) => `${total} stacks` }} />}</Card>
      </Content>

      <Modal title="New Stack" open={modalOpen} onCancel={() => setModalOpen(false)} onOk={() => form.submit()}>
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item name="name" label="Name" rules={[{ required: true, message: 'Stack name is required' }]}><Input placeholder="my-app" /></Form.Item>
          <Form.Item name="description" label="Description"><Input.TextArea rows={2} placeholder="Optional description" /></Form.Item>
        </Form>
      </Modal>

      <Modal title="🔄 Convert docker run → compose.yaml" open={convertModalOpen}
        onCancel={() => { setConvertModalOpen(false); setDockerRunCmd(''); setConvertedCompose(''); }}
        footer={null} width={700}>
        <div style={{ marginBottom: 8 }}>
          <label style={{ display: 'block', marginBottom: 4 }}>Paste your docker run command:</label>
          <Input.TextArea rows={3} value={dockerRunCmd} onChange={(e) => setDockerRunCmd(e.target.value)} placeholder="docker run -d --name myapp -p 8080:80 nginx:alpine" />
        </div>
        <Button type="primary" onClick={handleConvert} loading={convertLoading} style={{ marginBottom: 16 }}>Convert</Button>
        {convertError && <Alert type="error" message={convertError} style={{ marginBottom: 8 }} showIcon />}
        {convertedCompose && (
          <>
            <pre style={{ background: '#1e1e1e', color: '#d4d4d4', padding: 12, borderRadius: 6, overflow: 'auto', maxHeight: 300, fontSize: 12, marginBottom: 8 }}>{convertedCompose}</pre>
            <Button type="primary" onClick={handleSaveFromConvert}>Save as Stack</Button>
          </>
        )}
      </Modal>

      <TemplateBrowser
        open={templateModalOpen}
        onClose={() => setTemplateModalOpen(false)}
        onSelect={async (name, compose) => {
          try { await api.createStack({ name, compose }); message.success("Stack '" + name + "' created from template"); loadStacks(); }
          catch (e: any) { message.error('Error: ' + e.message); }
        }}
      />
    </Layout>
  );
}