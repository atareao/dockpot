import React, { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Typography, Spin, Button, Space, Tag, Card, Descriptions, App as AntApp, Layout,
} from 'antd';
import { ArrowLeftOutlined, PlayCircleOutlined, StopOutlined, ReloadOutlined, EditOutlined } from '@ant-design/icons';
import { api, Stack } from '../api/http';

const { Title } = Typography;
const { Content, Header } = Layout;

export function StackDetail() {
  const { id } = useParams<{ id: string }>();
  const [stack, setStack] = useState<Stack | null>(null);
  const [loading, setLoading] = useState(true);
  const { message } = AntApp.useApp();
  const navigate = useNavigate();

  const loadStack = async () => {
    if (!id) return;
    try {
      setLoading(true);
      const data = await api.getStack(id);
      setStack(data);
    } catch (e: any) {
      message.error('Error: ' + e.message);
      navigate('/stacks');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadStack(); }, [id]);

  const handleAction = async (action: 'start' | 'stop' | 'restart') => {
    if (!id) return;
    try {
      if (action === 'start') await api.startStack(id);
      else if (action === 'stop') await api.stopStack(id);
      else await api.restartStack(id);
      loadStack();
    } catch (e: any) {
      message.error('Error: ' + e.message);
    }
  };

  if (loading) return <Spin size="large" style={{ display: 'block', margin: '40px auto' }} />;
  if (!stack) return null;

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Header style={{ background: '#fff', padding: '0 24px', display: 'flex', alignItems: 'center', gap: 16 }}>
        <Button icon={<ArrowLeftOutlined />} onClick={() => navigate('/stacks')} />
        <Title level={3} style={{ margin: 0 }}>{stack.name}</Title>
        <Tag color={stack.status === 'running' ? 'green' : stack.status === 'error' ? 'red' : 'default'}>
          {stack.status}
        </Tag>
      </Header>
      <Content style={{ padding: 24 }}>
        <Card>
          <Descriptions column={1}>
            <Descriptions.Item label="ID">{stack.id}</Descriptions.Item>
            <Descriptions.Item label="Name">{stack.name}</Descriptions.Item>
            <Descriptions.Item label="Description">{stack.description || '—'}</Descriptions.Item>
            <Descriptions.Item label="Status">
              <Tag color={stack.status === 'running' ? 'green' : 'default'}>{stack.status}</Tag>
            </Descriptions.Item>
            <Descriptions.Item label="Path">{stack.path}</Descriptions.Item>
          </Descriptions>
          <Space style={{ marginTop: 16 }}>
            <Button
              type="primary"
              icon={<PlayCircleOutlined />}
              onClick={() => handleAction('start')}
              disabled={stack.status === 'running'}
            >
              Start
            </Button>
            <Button
              icon={<StopOutlined />}
              onClick={() => handleAction('stop')}
              disabled={stack.status !== 'running'}
            >
              Stop
            </Button>
            <Button
              icon={<ReloadOutlined />}
              onClick={() => handleAction('restart')}
              disabled={stack.status !== 'running'}
            >
              Restart
            </Button>
            <Button icon={<EditOutlined />}>Edit Compose</Button>
          </Space>
        </Card>

        <Card title="docker-compose.yaml" style={{ marginTop: 16 }}>
          <pre style={{
            background: '#1e1e1e',
            color: '#d4d4d4',
            padding: 16,
            borderRadius: 6,
            overflow: 'auto',
            maxHeight: 400,
            fontSize: 13,
          }}>
            {stack.compose}
          </pre>
        </Card>
      </Content>
    </Layout>
  );
}