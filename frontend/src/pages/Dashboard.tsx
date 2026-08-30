import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Typography, Card, Row, Col, Statistic, Spin, Tag, Alert,
  Button, Modal, Form, Input, Space, Tabs, App as AntApp,
} from 'antd';
import {
  ClusterOutlined, CheckCircleOutlined, StopOutlined, CloseCircleOutlined,
  ContainerOutlined,
  PlusOutlined, ReloadOutlined, PlayCircleOutlined, SwapOutlined, FileAddOutlined,
} from '@ant-design/icons';
import { api, Stack, DockerInfo, ExternalProject } from '../api/http';
import { TemplateBrowser } from '../components/TemplateBrowser';
import { useTheme } from '../main';

const { Title, Text } = Typography;

export function Dashboard() {
  const [stacks, setStacks] = useState<Stack[]>([]);
  const [externalProjects, setExternalProjects] = useState<ExternalProject[]>([]);
  const [dockerInfo, setDockerInfo] = useState<DockerInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();
  const { message } = AntApp.useApp();
  const { darkMode } = useTheme();

  // ── Create modal ──
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [createForm] = Form.useForm();

  // ── Convert modal ──
  const [convertModalOpen, setConvertModalOpen] = useState(false);
  const [dockerRunCmd, setDockerRunCmd] = useState('');
  const [convertedCompose, setConvertedCompose] = useState('');
  const [convertError, setConvertError] = useState('');
  const [convertLoading, setConvertLoading] = useState(false);

  // ── Templates modal ──
  const [templateModalOpen, setTemplateModalOpen] = useState(false);

  const loadData = async () => {
    try {
      setLoading(true);
      setError(null);
      const [stacksData, dockerData, projects] = await Promise.all([
        api.listStacks(),
        api.getDockerInfo().catch(() => null),
        api.discoverProjects().catch(() => [] as ExternalProject[]),
      ]);
      setStacks(stacksData);
      setDockerInfo(dockerData);
      setExternalProjects(projects.filter(p => !p.managed));
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadData(); }, []);

  const handleCreate = async (values: any) => {
    try {
      const stack = await api.createStack({
        name: values.name,
        compose: values.compose,
      });
      message.success(`Stack '${stack.name}' created`);
      setCreateModalOpen(false);
      createForm.resetFields();
      loadData();
    } catch (e: any) {
      message.error('Error: ' + e.message);
    }
  };

  const handleDelete = (id: string, name: string) => {
    Modal.confirm({
      title: `Delete stack '${name}'?`,
      content: 'This will remove the stack and ALL its files. Are you sure?',
      okText: 'Delete', okType: 'danger',
      onOk: async () => {
        try {
          await api.deleteStack(id);
          message.success(`Stack '${name}' deleted`);
          loadData();
        } catch (e: any) {
          message.error('Error: ' + e.message);
        }
      },
    });
  };

  const handleImport = async (name: string) => {
    try {
      await api.importProject(name);
      message.success(`✅ Project '${name}' imported as stack`);
      loadData();
    } catch (e: any) {
      message.error(`Import failed: ${e.message}`);
    }
  };

  const handleCaptureContainer = async (containerName: string) => {
    try {
      await api.createFromContainer(containerName);
      message.success(`📦 Container '${containerName}' captured as stack`);
      loadData();
    } catch (e: any) {
      message.error(`Capture failed: ${e.message}`);
    }
  };

  const handleAction = async (id: string, action: 'start' | 'stop' | 'restart') => {
    try {
      if (action === 'start') await api.startStack(id);
      else if (action === 'stop') await api.stopStack(id);
      else await api.restartStack(id);
      loadData();
      message.success(`Stack ${action}ed`);
    } catch (e: any) {
      message.error(`Error: ${e.message}`);
    }
  };

  const handleConvert = async () => {
    if (!dockerRunCmd.trim()) return;
    setConvertLoading(true);
    setConvertError('');
    setConvertedCompose('');
    try {
      const result = await api.convertDockerRun(dockerRunCmd);
      setConvertedCompose(result.compose || '');
      message.success('Converted successfully');
    } catch (e: any) {
      setConvertError(e.message);
    } finally {
      setConvertLoading(false);
    }
  };

  const totalStacks = stacks.length;
  const runningStacks = stacks.filter(s => s.status === 'running').length;
  const stoppedStacks = totalStacks - runningStacks;

  if (error) {
    return (
      <div style={{ padding: 12 }}>
        <Alert type="error" message="Failed to load dashboard" description={error} showIcon />
      </div>
    );
  }

  const composeProjects = externalProjects.filter(p => p.type !== 'container');
  const standaloneContainers = externalProjects.filter(p => p.type === 'container');

  return (
    <div>
      {/* Local Header */}
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: '8px 12px', borderBottom: `1px solid ${darkMode ? '#303030' : '#f0f0f0'}`,
        flexWrap: 'wrap', gap: 8,
      }}>
        <Title level={4} style={{ margin: 0 }}>📊 Dashboard</Title>
      </div>

      <div style={{ padding: 12 }}>
        {loading ? (
          <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: 200 }}>
            <Spin size="large" />
          </div>
        ) : (
          <Tabs
            defaultActiveKey="stacks"
            size="small"
            tabBarExtraContent={
              <Space size="small" wrap>
                <Button size="small" icon={<FileAddOutlined />} onClick={() => setTemplateModalOpen(true)}>Templates</Button>
                <Button size="small" icon={<SwapOutlined />} onClick={() => setConvertModalOpen(true)}>Convert</Button>
                <Button size="small" icon={<ReloadOutlined />} onClick={loadData}>Refresh</Button>
                <Button size="small" type="primary" icon={<PlusOutlined />} onClick={() => setCreateModalOpen(true)}>New Stack</Button>
              </Space>
            }
            items={[
              {
                key: 'stacks',
                label: '📦 My Stacks',
                children: (
                  <>
                    {/* Stat Cards */}
                    <Row gutter={[16, 16]}>
                      <Col xs={24} sm={12} lg={6}>
                        <Card hoverable>
                          <Statistic title="Total Stacks" value={totalStacks} prefix={<ClusterOutlined />} valueStyle={{ color: '#1677ff' }} />
                        </Card>
                      </Col>
                      <Col xs={24} sm={12} lg={6}>
                        <Card hoverable>
                          <Statistic title="Running" value={runningStacks} prefix={<CheckCircleOutlined />} valueStyle={{ color: '#52c41a' }} suffix={`/ ${totalStacks}`} />
                        </Card>
                      </Col>
                      <Col xs={24} sm={12} lg={6}>
                        <Card hoverable>
                          <Statistic title="Stopped" value={stoppedStacks} prefix={<StopOutlined />} valueStyle={{ color: '#faad14' }} />
                        </Card>
                      </Col>
                      <Col xs={24} sm={12} lg={6}>
                        <Card hoverable>
                          <Statistic title="Docker Containers" value={dockerInfo?.containers_total ?? '—'} prefix={<ContainerOutlined />} valueStyle={{ color: '#722ed1' }} />
                        </Card>
                      </Col>
                    </Row>

                    {/* Stacks Grid */}
                    <Row gutter={[16, 16]} style={{ marginTop: 16, display: 'flex', flexWrap: 'wrap' }}>
                      {stacks.map((stack) => (
                        <Col xs={24} sm={12} lg={8} xl={6} key={stack.id} style={{ display: 'flex' }}>
                          <Card
                            hoverable
                            style={{
                              width: '100%',
                              borderLeft: `4px solid ${
                                stack.status === 'running' ? '#52c41a' :
                                stack.status === 'error' ? '#ff4d4f' :
                                '#d9d9d9'
                              }`,
                            }}
                            onClick={() => navigate(`/stacks/${stack.id}`)}
                            styles={{ body: { padding: 16, display: 'flex', flexDirection: 'column', height: '100%' } }}
                          >
                            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 8 }}>
                              <Text strong ellipsis style={{ fontSize: 15, maxWidth: '60%' }}>{stack.name}</Text>
                              <Tag color={stack.status === 'running' ? 'green' : stack.status === 'error' ? 'red' : 'default'}>
                                {stack.status}
                              </Tag>
                            </div>
                            {stack.description && (
                              <Text type="secondary" style={{ display: 'block', marginBottom: 8, fontSize: 13 }} ellipsis>
                                {stack.description}
                              </Text>
                            )}
                            <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 12 }}>
                              Updated: {new Date(stack.updated_at).toLocaleString()}
                            </Text>
                            <div style={{ marginTop: 'auto' }}>
                              <Space.Compact block>
                                <Button size="small" icon={<PlayCircleOutlined />} disabled={stack.status === 'running'} onClick={(e) => { e.stopPropagation(); handleAction(stack.id, 'start'); }} />
                                <Button size="small" icon={<StopOutlined />} disabled={stack.status !== 'running'} onClick={(e) => { e.stopPropagation(); handleAction(stack.id, 'stop'); }} />
                                <Button size="small" icon={<ReloadOutlined />} disabled={stack.status !== 'running'} onClick={(e) => { e.stopPropagation(); handleAction(stack.id, 'restart'); }} />
                                <Button size="small" danger icon={<CloseCircleOutlined />} onClick={(e) => { e.stopPropagation(); handleDelete(stack.id, stack.name); }} />
                              </Space.Compact>
                            </div>
                          </Card>
                        </Col>
                      ))}
                    </Row>

                    {stacks.length === 0 && !loading && (
                      <Card style={{ marginTop: 16, textAlign: 'center', padding: 40 }}>
                        <Title level={4} type="secondary">No stacks yet</Title>
                        <Text type="secondary">Create your first stack to get started</Text>
                        <br /><br />
                        <Button type="primary" icon={<PlusOutlined />} onClick={() => setCreateModalOpen(true)}>New Stack</Button>
                      </Card>
                    )}
                  </>
                ),
              },
              {
                key: 'discover',
                label: '🔍 Discover',
                children: (
                  <>
                    {composeProjects.length > 0 && (
                      <>
                        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
                          <Title level={5} style={{ margin: 0 }}>🔍 External Compose Projects</Title>
                          <Tag>{composeProjects.length} found</Tag>
                          <Text type="secondary" style={{ fontSize: 13 }}>Compose projects running outside Dockpot</Text>
                        </div>
                        <Row gutter={[16, 16]} style={{ display: 'flex', flexWrap: 'wrap' }}>
                          {composeProjects.map((p) => (
                            <Col xs={24} sm={12} lg={8} xl={6} key={p.name} style={{ display: 'flex' }}>
                              <Card style={{ width: '100%', opacity: 0.85, borderLeft: '4px solid #1677ff' }} styles={{ body: { padding: 16, display: 'flex', flexDirection: 'column', height: '100%' } }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 8 }}>
                                  <Text strong ellipsis style={{ fontSize: 15, maxWidth: '60%' }}>{p.name}</Text>
                                  <Tag color="default" style={{ fontSize: 11 }}>compose</Tag>
                                </div>
                                <Text type="secondary" ellipsis style={{ fontSize: 12, display: 'block', marginBottom: 12 }}>{p.config_files}</Text>
                                <div style={{ marginTop: 'auto' }}>
                                  <Button type="primary" size="small" icon={<PlusOutlined />} block onClick={() => handleImport(p.name)}>Import to Dockpot</Button>
                                </div>
                              </Card>
                            </Col>
                          ))}
                        </Row>
                      </>
                    )}

                    {standaloneContainers.length > 0 && (
                      <>
                        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: composeProjects.length > 0 ? 24 : 0, marginBottom: 12 }}>
                          <Title level={5} style={{ margin: 0 }}>📦 Standalone Containers</Title>
                          <Tag>{standaloneContainers.length} found</Tag>
                          <Text type="secondary" style={{ fontSize: 13 }}>Containers running with `docker run`</Text>
                        </div>
                        <Row gutter={[16, 16]} style={{ display: 'flex', flexWrap: 'wrap' }}>
                          {standaloneContainers.map((c) => (
                            <Col xs={24} sm={12} lg={8} xl={6} key={c.name} style={{ display: 'flex' }}>
                              <Card style={{ width: '100%', opacity: 0.8, borderLeft: '4px solid #722ed1' }} styles={{ body: { padding: 16, display: 'flex', flexDirection: 'column', height: '100%' } }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 8 }}>
                                  <Text strong ellipsis style={{ fontSize: 15, maxWidth: '60%' }}>{c.name}</Text>
                                  <Tag color="blue" style={{ fontSize: 11 }}>container</Tag>
                                </div>
                                {c.image && <Text type="secondary" ellipsis style={{ fontSize: 12, display: 'block', marginBottom: 4 }}>Image: {c.image}</Text>}
                                {c.ports && <Text type="secondary" ellipsis style={{ fontSize: 12, display: 'block', marginBottom: 12 }}>Ports: {c.ports}</Text>}
                                <div style={{ marginTop: 'auto' }}>
                                  <Button type="primary" size="small" icon={<PlusOutlined />} block ghost onClick={() => handleCaptureContainer(c.name)}>Capture as Stack</Button>
                                </div>
                              </Card>
                            </Col>
                          ))}
                        </Row>
                      </>
                    )}

                    {externalProjects.length === 0 && (
                      <div style={{ textAlign: 'center', padding: 40 }}>
                        <Title level={4} type="secondary">Nothing to discover</Title>
                        <Text type="secondary">All detected compose projects and containers are already managed by Dockpot</Text>
                      </div>
                    )}
                  </>
                ),
              },
            ]}
          />
        )}
      </div>

      {/* ── Create Modal ── */}
      <Modal title="New Stack" open={createModalOpen} onCancel={() => setCreateModalOpen(false)} onOk={() => createForm.submit()} width={700}>
        <Form form={createForm} layout="vertical" onFinish={handleCreate}>
          <Form.Item name="name" label="Stack Name" rules={[{ required: true, message: 'Name is required' }]}>
            <Input placeholder="my-app" />
          </Form.Item>
          <Form.Item name="compose" label="docker-compose.yaml" rules={[{ required: true, message: 'Compose content is required' }]}>
            <Input.TextArea rows={12} placeholder="services:&#10;  app:&#10;    image: nginx:alpine" style={{ fontFamily: 'monospace', fontSize: 12 }} />
          </Form.Item>
        </Form>
      </Modal>

      {/* ── Convert Modal ── */}
      <Modal title="Convert docker run to compose" open={convertModalOpen} onCancel={() => { setConvertModalOpen(false); setConvertError(''); setConvertedCompose(''); }} footer={null} width={700}>
        <Input.TextArea rows={3} value={dockerRunCmd} onChange={(e) => setDockerRunCmd(e.target.value)} placeholder="docker run -d --name myapp -p 80:80 nginx:alpine" style={{ fontFamily: 'monospace', fontSize: 12, marginBottom: 8 }} />
        <Button type="primary" onClick={handleConvert} loading={convertLoading} style={{ marginBottom: 8 }}>Convert</Button>
        {convertError && <Alert type="error" message={convertError} showIcon style={{ marginBottom: 8 }} />}
        {convertedCompose && (
          <pre style={{ background: '#1e1e1e', color: '#d4d4d4', padding: 12, borderRadius: 6, overflow: 'auto', maxHeight: 300, fontSize: 12, marginBottom: 8 }}>{convertedCompose}</pre>
        )}
      </Modal>

      {/* ── Templates Modal ── */}
      <TemplateBrowser open={templateModalOpen} onClose={() => setTemplateModalOpen(false)} onSelect={(name, compose) => {
        api.createStack({ name, compose }).then((stack) => {
          message.success(`Stack '${stack.name}' created from template`);
          setTemplateModalOpen(false);
          loadData();
        }).catch((e) => message.error('Error: ' + e.message));
      }} />
    </div>
  );
}