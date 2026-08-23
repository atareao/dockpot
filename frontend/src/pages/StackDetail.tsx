import React, { useState, useEffect, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Typography, Spin, Button, Space, Tag, Card, Descriptions, App as AntApp, Layout,
  Alert, Tabs, Switch, Modal,
} from 'antd';
import {
  ArrowLeftOutlined, PlayCircleOutlined, StopOutlined, ReloadOutlined,
  CloudUploadOutlined, CheckCircleOutlined, CloseCircleOutlined,
  DownloadOutlined, CodeOutlined, ConsoleOutlined,
} from '@ant-design/icons';
import { api, Stack } from '../api/http';
import { YamlEditor } from '../components/YamlEditor';
import { Terminal } from '../components/Terminal';

const { Title, Text } = Typography;
const { Content, Header } = Layout;

export function StackDetail() {
  const { id } = useParams<{ id: string }>();
  const [stack, setStack] = useState<Stack | null>(null);
  const [compose, setCompose] = useState('');
  const [originalCompose, setOriginalCompose] = useState('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [deploying, setDeploying] = useState(false);
  const [pulling, setPulling] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [yamlValid, setYamlValid] = useState(true);
  const [yamlErrors, setYamlErrors] = useState<string[]>([]);
  const { message } = AntApp.useApp();
  const navigate = useNavigate();

  const loadStack = useCallback(async () => {
    if (!id) return;
    try {
      setLoading(true);
      const data = await api.getStack(id);
      setStack(data);
      setCompose(data.compose);
      setOriginalCompose(data.compose);
    } catch (e: any) {
      message.error('Error: ' + e.message);
      navigate('/stacks');
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => { loadStack(); }, [loadStack]);

  const handleAction = async (action: 'start' | 'stop' | 'restart') => {
    if (!id) return;
    try {
      if (action === 'start') await api.startStack(id);
      else if (action === 'stop') await api.stopStack(id);
      else await api.restartStack(id);
      loadStack();
      message.success(`Stack ${action}ed`);
    } catch (e: any) {
      message.error('Error: ' + e.message);
    }
  };

  const handleSaveCompose = async () => {
    if (!id) return;
    if (!yamlValid) {
      message.error('Cannot save — YAML has syntax errors');
      return;
    }
    try {
      setSaving(true);
      await api.updateCompose(id, compose);
      setOriginalCompose(compose);
      message.success('Compose saved');
      setEditMode(false);
    } catch (e: any) {
      message.error('Error: ' + e.message);
    } finally {
      setSaving(false);
    }
  };

  const handleDeploy = async () => {
    if (!id || !stack) return;
    if (compose !== originalCompose && !yamlValid) {
      message.error('Fix YAML errors before deploying');
      return;
    }

    Modal.confirm({
      title: `Deploy '${stack.name}'?`,
      content: compose !== originalCompose
        ? 'Unsaved changes will be saved first, then the stack will start.'
        : 'This will run `docker compose up -d` for this stack.',
      okText: 'Deploy',
      onOk: async () => {
        try {
          setDeploying(true);
          // Save if modified
          if (compose !== originalCompose) {
            await api.updateCompose(id, compose);
            setOriginalCompose(compose);
          }
          // Start the stack
          await api.startStack(id);
          setEditMode(false);
          loadStack();
          message.success(`🚀 '${stack.name}' deployed`);
        } catch (e: any) {
          message.error('Deploy failed: ' + e.message);
        } finally {
          setDeploying(false);
        }
      },
    });
  };

  const handlePull = async () => {
    if (!id) return;
    try {
      setPulling(true);
      await api.pullStack(id);
      message.success('📥 Images updated');
    } catch (e: any) {
      message.error('Pull failed: ' + e.message);
    } finally {
      setPulling(false);
    }
  };

  const handleValidateSyntax = async () => {
    try {
      const result = await api.validateCompose(compose);
      if (result.valid) {
        message.success('✅ YAML syntax is valid');
        setYamlValid(true);
        setYamlErrors([]);
      } else {
        message.error('❌ ' + result.error);
      }
    } catch (e: any) {
      message.error('Validation error: ' + e.message);
    }
  };

  const hasChanges = compose !== originalCompose;

  if (loading) return <Spin size="large" style={{ display: 'block', margin: '40px auto' }} />;
  if (!stack) return null;

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Header style={{ background: '#fff', padding: '0 24px', display: 'flex', alignItems: 'center', gap: 16, borderBottom: '1px solid #f0f0f0' }}>
        <Button icon={<ArrowLeftOutlined />} onClick={() => navigate('/stacks')} />
        <Title level={3} style={{ margin: 0 }}>{stack.name}</Title>
        <Tag color={stack.status === 'running' ? 'green' : stack.status === 'error' ? 'red' : 'default'}>
          {stack.status}
        </Tag>
        <div style={{ flex: 1 }} />
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
        <Button
          type="primary"
          ghost
          icon={<CloudUploadOutlined />}
          onClick={handleDeploy}
          loading={deploying}
        >
          Deploy
        </Button>
        <Button
          icon={<DownloadOutlined />}
          onClick={handlePull}
          loading={pulling}
        >
          Update Images
        </Button>
      </Header>
      <Content style={{ padding: 24 }}>
        <Card size="small" style={{ marginBottom: 16 }}>
          <Descriptions column={2} size="small">
            <Descriptions.Item label="ID"><Text copyable>{stack.id}</Text></Descriptions.Item>
            <Descriptions.Item label="Name">{stack.name}</Descriptions.Item>
            <Descriptions.Item label="Description">{stack.description || '—'}</Descriptions.Item>
            <Descriptions.Item label="Status">
              <Tag color={stack.status === 'running' ? 'green' : 'default'}>{stack.status}</Tag>
            </Descriptions.Item>
            <Descriptions.Item label="Created">{new Date(stack.created_at).toLocaleString()}</Descriptions.Item>
            <Descriptions.Item label="Updated">{new Date(stack.updated_at).toLocaleString()}</Descriptions.Item>
          </Descriptions>
        </Card>

        <Card
          title={
            <Space>
              <span>docker-compose.yaml</span>
              <Switch
                checkedChildren={<><CheckCircleOutlined /> Edit</>}
                unCheckedChildren="Preview"
                checked={editMode}
                onChange={(v) => {
                  if (!v && hasChanges && yamlValid) {
                    handleSaveCompose();
                  }
                  setEditMode(v);
                }}
              />
              {hasChanges && <Tag color="orange">unsaved</Tag>}
            </Space>
          }
          extra={
            editMode ? (
              <Space>
                <Button size="small" onClick={handleValidateSyntax} icon={<CheckCircleOutlined />}>
                  Validate
                </Button>
                <Button
                  size="small"
                  type="primary"
                  onClick={handleSaveCompose}
                  loading={saving}
                  disabled={!yamlValid}
                >
                  Save
                </Button>
                <Button
                  type="primary"
                  icon={<CloudUploadOutlined />}
                  onClick={handleDeploy}
                  loading={deploying}
                >
                  Save & Deploy
                </Button>
              </Space>
            ) : null
          }
          style={{ marginTop: 16 }}
        >
          {!yamlValid && editMode && yamlErrors.length > 0 && (
            <Alert
              type="error"
              icon={<CloseCircleOutlined />}
              message={
                <ul style={{ margin: 0, paddingLeft: 16 }}>
                  {yamlErrors.map((e, i) => <li key={i}>{e}</li>)}
                </ul>
              }
              style={{ marginBottom: 8 }}
              showIcon
            />
          )}

          {editMode ? (
            <YamlEditor
              value={compose}
              onChange={(v) => setCompose(v)}
              onValidate={(isValid, errors) => {
                setYamlValid(isValid);
                setYamlErrors(errors);
              }}
              height={500}
            />
          ) : (
            <pre style={{
              background: '#1e1e1e',
              color: '#d4d4d4',
              padding: 16,
              borderRadius: 6,
              overflow: 'auto',
              maxHeight: 400,
              fontSize: 13,
              margin: 0,
            }}>
              {compose}
            </pre>
          )}
        </Card>

        <Card title="📋 Live Logs" style={{ marginTop: 16 }}>
          <Terminal stackId={stack.id} stackName={stack.name} height={350} />
        </Card>
      </Content>
    </Layout>
  );
}