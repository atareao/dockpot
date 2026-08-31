import { useState, useEffect, useCallback, useMemo } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Typography, Spin, Button, Space, Tag, Card, Descriptions, App as AntApp, Layout,
  Alert, Modal, Tabs, Dropdown, Segmented,
} from 'antd';
import { ArrowLeftOutlined, PlayCircleOutlined, StopOutlined, ReloadOutlined,
  CloudUploadOutlined, CheckCircleOutlined, CloseCircleOutlined,
  DownloadOutlined, GithubOutlined, CodeOutlined, MoreOutlined,
} from '@ant-design/icons';
import { api, Stack, StackSync } from '../api/http';
import type { EnvFileData } from '../components/visual/EnvFileList';
import { YamlEditor } from '../components/YamlEditor';
import { Terminal } from '../components/Terminal';
import { DiffViewer } from '../components/DiffViewer';
import { useTheme } from '../main';
import { load, dump } from 'js-yaml';
import type { ComposeDefinition } from '../types/compose';
import ServiceList from '../components/visual/ServiceList';
import VolumeList from '../components/visual/VolumeList';
import NetworkList from '../components/visual/NetworkList';
import ConfigSecretList from '../components/visual/ConfigSecretList';
import EnvFileList from '../components/visual/EnvFileList';

const { Title, Text } = Typography;
const { Content, Header } = Layout;

/** Parse YAML → ComposeDefinition */
function parseCompose(yaml: string): ComposeDefinition | null {
  if (!yaml || !yaml.trim()) return null;
  try {
    const parsed = load(yaml) as any;
    if (!parsed || typeof parsed !== 'object') return null;
    if (!parsed.services || typeof parsed.services !== 'object') parsed.services = {};
    parsed.volumes = parsed.volumes || {};
    parsed.networks = parsed.networks || {};
    parsed.configs = parsed.configs || {};
    parsed.secrets = parsed.secrets || {};
    return parsed as ComposeDefinition;
  } catch {
    return null;
  }
}

/** Serialize ComposeDefinition → YAML */
function serializeCompose(def: ComposeDefinition): string {
  const cleaned = { ...def };
  for (const key of ['volumes', 'networks', 'configs', 'secrets'] as const) {
    const k = key as keyof ComposeDefinition;
    if (cleaned[k] && typeof cleaned[k] === 'object' && Object.keys(cleaned[k] as object).length === 0) {
      delete cleaned[k];
    }
  }
  return dump(cleaned, { indent: 2, lineWidth: -1, noRefs: true, sortKeys: false, forceQuotes: false });
}

export function StackDetail() {
  const { id } = useParams<{ id: string }>();
  const [stack, setStack] = useState<Stack | null>(null);
  const [compose, setCompose] = useState('');
  const [originalCompose, setOriginalCompose] = useState('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [deploying, setDeploying] = useState(false);
  const [pulling, setPulling] = useState(false);
  const [syncConfig, setSyncConfig] = useState<StackSync | null>(null);
  const [syncModalOpen, setSyncModalOpen] = useState(false);
  const [envFiles, setEnvFiles] = useState<EnvFileData[]>([]);
  const [notifiers, setNotifiers] = useState<any[]>([]);
  const [stackNotifiers, setStackNotifiers] = useState<string[]>([]);
  const [stats, setStats] = useState<any>(null);
  const [diffModalOpen, setDiffModalOpen] = useState(false);
  const [diffData, setDiffData] = useState<any>(null);
  const [mode, setMode] = useState<'raw' | 'visual'>('raw');
  const [yamlValid, setYamlValid] = useState(true);
  const [yamlErrors, setYamlErrors] = useState<string[]>([]);
  const { message } = AntApp.useApp();
  const navigate = useNavigate();
  const { darkMode } = useTheme();

  // ── Data loading ──

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
      navigate('/');
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => { loadStack(); }, [loadStack]);

  const loadEnvFiles = async () => {
    if (!id) return;
    try { setEnvFiles(await api.listEnvFiles(id)); } catch {}
  };

  const loadNotifiers = async () => {
    if (!id) return;
    try {
      setNotifiers(await api.listNotifiers());
      setStackNotifiers(await api.getStackNotifiers(id));
    } catch {}
  };

  const loadStats = async () => {
    if (!id) return;
    try { setStats(await api.getStackStats(id)); } catch {}
  };

  useEffect(() => { if (id) { loadEnvFiles(); loadNotifiers(); loadStats(); } }, [id]);

  // ── Parse compose for visual mode ──

  const definition = useMemo(() => {
    if (mode !== 'visual') return null;
    return parseCompose(compose);
  }, [compose, mode]);

  const handleComposeChange = useCallback((updated: ComposeDefinition) => {
    try {
      const yaml = serializeCompose(updated);
      setCompose(yaml);
    } catch {}
  }, []);

  // ── Handlers for each section ──

  const handleServicesChange = useCallback(
    (services: ComposeDefinition['services']) => {
      if (!definition) return;
      handleComposeChange({ ...definition, services });
    },
    [definition, handleComposeChange],
  );

  const handleVolumesChange = useCallback(
    (volumes: ComposeDefinition['volumes']) => {
      if (!definition) return;
      handleComposeChange({ ...definition, volumes });
    },
    [definition, handleComposeChange],
  );

  const handleNetworksChange = useCallback(
    (networks: ComposeDefinition['networks']) => {
      if (!definition) return;
      handleComposeChange({ ...definition, networks });
    },
    [definition, handleComposeChange],
  );

  const handleConfigsChange = useCallback(
    (configs: ComposeDefinition['configs']) => {
      if (!definition) return;
      handleComposeChange({ ...definition, configs });
    },
    [definition, handleComposeChange],
  );

  const handleSecretsChange = useCallback(
    (secrets: ComposeDefinition['secrets']) => {
      if (!definition) return;
      handleComposeChange({ ...definition, secrets });
    },
    [definition, handleComposeChange],
  );

  // ── Actions ──

  const handleShowDiff = async () => {
    if (!id) return;
    try {
      const data = await api.syncDiff(id);
      setDiffData(data);
      setDiffModalOpen(true);
    } catch (e: any) {
      message.error('Diff failed: ' + e.message);
    }
  };

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
      setMode("raw");
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
          if (compose !== originalCompose) {
            await api.updateCompose(id, compose);
            setOriginalCompose(compose);
          }
          await api.startStack(id);
          setMode("raw");
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

  // ── Volume & network names for ServiceCard ──

  const volumeNames = useMemo(
    () => (definition?.volumes ? Object.keys(definition.volumes) : []),
    [definition?.volumes],
  );
  const networkNames = useMemo(
    () => (definition?.networks ? Object.keys(definition.networks) : []),
    [definition?.networks],
  );

  // ── Counts for badges ──

  const svcCount = definition?.services ? Object.keys(definition.services).length : 0;
  const volCount = definition?.volumes ? Object.keys(definition.volumes).length : 0;
  const netCount = definition?.networks ? Object.keys(definition.networks).length : 0;
  const cfgCount = definition?.configs ? Object.keys(definition.configs).length : 0;
  const secCount = definition?.secrets ? Object.keys(definition.secrets).length : 0;
  const envCount = envFiles.length;

  // ── Compose content for preview/raw ──

  const composeContent = (
    <div>
      {!yamlValid && mode === 'raw' && yamlErrors.length > 0 && (
        <Alert type="error" icon={<CloseCircleOutlined />}
          message={<ul style={{ margin: 0, paddingLeft: 16 }}>{yamlErrors.map((e, i) => <li key={i}>{e}</li>)}</ul>}
          style={{ marginBottom: 8 }} showIcon />
      )}
      <YamlEditor value={compose} onChange={(v) => setCompose(v)}
        onValidate={(isValid, errors) => { setYamlValid(isValid); setYamlErrors(errors); }}
        height={Math.min(500, window.innerHeight - 350)} />
    </div>
  );

  // ── Visual editor for each section ──

  const visualServices = definition ? (
    <ServiceList value={definition.services} onChange={handleServicesChange}
      volumeNames={volumeNames} networkNames={networkNames} />
  ) : composeContent;

  const visualVolumes = definition ? (
    <VolumeList value={definition.volumes ?? {}} onChange={handleVolumesChange} />
  ) : composeContent;

  const visualNetworks = definition ? (
    <NetworkList value={definition.networks ?? {}} onChange={handleNetworksChange} />
  ) : composeContent;

  const visualConfigs = definition ? (
    <ConfigSecretList title="Configs" value={definition.configs ?? {}} onChange={handleConfigsChange} />
  ) : composeContent;

  const visualSecrets = definition ? (
    <ConfigSecretList title="Secrets" value={definition.secrets ?? {}} onChange={handleSecretsChange} />
  ) : composeContent;

  // ── Loading ──

  if (loading) return <Spin size="large" style={{ display: 'block', margin: '40px auto' }} />;
  if (!stack) return null;

  // ── Tab items — dynamic based on mode ──

  // Always present: Logs, Notifiers
  const commonTabs = [
    {
      key: 'logs',
      label: '📋 Logs',
      children: (
        <div style={{ padding: '12px 12px 0' }}>
          <Terminal stackId={stack.id} stackName={stack.name} height={Math.min(500, window.innerHeight - 250)} />
        </div>
      ),
    },
    {
      key: 'notifiers',
      label: '📢 Notifiers',
      children: (
        <div style={{ padding: '12px 12px 0' }}>
          {notifiers.length === 0 ? <Text type="secondary">No notifiers configured</Text> : (
            <Space wrap>
              {notifiers.map((n) => (
                <Tag key={n.id} color={stackNotifiers.includes(n.id) ? 'blue' : 'default'}
                  style={{ cursor: 'pointer' }}
                  onClick={async () => {
                    const current = await api.getStackNotifiers(stack.id);
                    const updated = current.includes(n.id)
                      ? current.filter((x: string) => x !== n.id)
                      : [...current, n.id];
                    await api.setStackNotifiers(stack.id, updated);
                    setStackNotifiers(updated);
                  }}>
                  {n.name} ({n.notifier_type})
                </Tag>
              ))}
            </Space>
          )}
        </div>
      ),
    },
  ];

  // ── Tab items — conditional on mode ──

  const sectionContent = (key: string) => {
    if (mode === 'visual' && definition) {
      switch (key) {
        case 'services': return <div style={{ padding: '12px 12px 0' }}>{visualServices}</div>;
        case 'volumes': return <div style={{ padding: '12px 12px 0' }}>{visualVolumes}</div>;
        case 'networks': return <div style={{ padding: '12px 12px 0' }}>{visualNetworks}</div>;
        case 'configs': return <div style={{ padding: '12px 12px 0' }}>{visualConfigs}</div>;
        case 'secrets': return <div style={{ padding: '12px 12px 0' }}>{visualSecrets}</div>;
      }
    }
    return <div style={{ padding: '12px 12px 0' }}>{composeContent}</div>;
  };

  const envTab = {
    key: 'env',
    label: <span>🔤 Env Files <Tag style={{ fontSize: 10, marginLeft: 4 }}>{envCount}</Tag></span>,
    children: (
      <div style={{ padding: '12px 12px 0' }}>
        <EnvFileList value={envFiles}
          mode={mode}
          onUpsert={async (filename, content) => { if (!id) return; await api.upsertEnvFile(id, filename, content); loadEnvFiles(); }}
          onDelete={async (filename) => { if (!id) return; await api.deleteEnvFile(id, filename); loadEnvFiles(); }} />
      </div>
    ),
  };

  // In Visual mode: show all compose sections
  // In Preview/Raw mode: only Services + Env Files
  const composeTabs = mode === 'visual' && definition
    ? [
        {
          key: 'services',
          label: <span>🐳 Services <Tag style={{ fontSize: 10, marginLeft: 4 }}>{svcCount}</Tag></span>,
          children: sectionContent('services'),
        },
        {
          key: 'volumes',
          label: <span>💾 Volumes <Tag style={{ fontSize: 10, marginLeft: 4 }}>{volCount}</Tag></span>,
          children: sectionContent('volumes'),
        },
        {
          key: 'networks',
          label: <span>🌐 Networks <Tag style={{ fontSize: 10, marginLeft: 4 }}>{netCount}</Tag></span>,
          children: sectionContent('networks'),
        },
        {
          key: 'configs',
          label: <span>⚙️ Configs <Tag style={{ fontSize: 10, marginLeft: 4 }}>{cfgCount}</Tag></span>,
          children: sectionContent('configs'),
        },
        {
          key: 'secrets',
          label: <span>🔒 Secrets <Tag style={{ fontSize: 10, marginLeft: 4 }}>{secCount}</Tag></span>,
          children: sectionContent('secrets'),
        },
        envTab,
      ]
    : [
        {
          key: 'services',
          label: '🐳 Services',
          children: <div style={{ padding: '12px 12px 0' }}>{composeContent}</div>,
        },
        envTab,
      ];

  const tabItems = [...composeTabs, ...commonTabs];

  // ── More actions dropdown ──
  const moreItems = [
    { key: 'sync', icon: <GithubOutlined />, label: 'Git Sync', onClick: () => { setSyncModalOpen(true); id && api.getSyncConfig(id).then(setSyncConfig).catch(() => {}); } },
    { key: 'export', icon: <DownloadOutlined />, label: 'Export', onClick: () => api.exportStack(stack.id) },
    { key: 'diff', icon: <CodeOutlined />, label: 'Diff', onClick: handleShowDiff },
  ];

  return (
    <Layout style={{ minHeight: '100vh', background: darkMode ? '#000' : '#f5f5f5' }}>
      <Header style={{
        background: darkMode ? '#141414' : '#fff',
        padding: '0 12px',
        display: 'flex', alignItems: 'center', gap: 8,
        borderBottom: `1px solid ${darkMode ? '#303030' : '#f0f0f0'}`,
        flexWrap: 'wrap', minHeight: 48, height: 'auto',
      }}>
        <Button icon={<ArrowLeftOutlined />} onClick={() => navigate('/')} size="small" />
        <Title level={4} style={{ margin: 0, fontSize: 16, flex: 1, minWidth: 80, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {stack.name}
        </Title>
        <Tag color={stack.status === 'running' ? 'green' : stack.status === 'error' ? 'red' : 'default'} style={{ margin: 0 }}>
          {stack.status}
        </Tag>

        <div style={{ display: 'flex', alignItems: 'center', gap: 4, flexWrap: 'wrap' }}>
          <Button size="small" type="primary" icon={<PlayCircleOutlined />} onClick={() => handleAction('start')} disabled={stack.status === 'running'} />
          <Button size="small" icon={<StopOutlined />} onClick={() => handleAction('stop')} disabled={stack.status !== 'running'} />
          <Button size="small" icon={<ReloadOutlined />} onClick={() => handleAction('restart')} disabled={stack.status !== 'running'} />
          <Button size="small" type="primary" ghost icon={<CloudUploadOutlined />} onClick={handleDeploy} loading={deploying} />
          <Button size="small" icon={<DownloadOutlined />} onClick={handlePull} loading={pulling} />
          <Dropdown menu={{ items: moreItems }} trigger={['click']}>
            <Button size="small" icon={<MoreOutlined />} />
          </Dropdown>
          {syncConfig && syncConfig.sync_type !== 'none' && (
            <Tag color={syncConfig.status === 'synced' ? 'green' : syncConfig.status === 'conflict' ? 'red' : 'orange'} style={{ fontSize: 11, margin: 0 }}>
              {syncConfig.status}
            </Tag>
          )}
        </div>
      </Header>

      <Content style={{ padding: 12, background: darkMode ? '#000' : undefined }}>
        {/* Info Card */}
        <Card size="small" styles={{ body: { padding: '8px 12px' } }} style={{ marginBottom: 12 }}>
          <Descriptions column={{ xs: 1, sm: 3 }} size="small" style={{ marginBottom: 0 }}>
            <Descriptions.Item label="ID"><Text copyable={{ text: stack.id }} style={{ fontSize: 12 }}>{stack.id.substring(0, 8)}…</Text></Descriptions.Item>
            <Descriptions.Item label="Name">{stack.name}</Descriptions.Item>
            <Descriptions.Item label="Status">
              <Tag color={stack.status === 'running' ? 'green' : 'default'} style={{ margin: 0 }}>{stack.status}</Tag>
            </Descriptions.Item>
            <Descriptions.Item label="Description">{stack.description || '—'}</Descriptions.Item>
            <Descriptions.Item label="Created">{new Date(stack.created_at).toLocaleString()}</Descriptions.Item>
            <Descriptions.Item label="Updated">{new Date(stack.updated_at).toLocaleString()}</Descriptions.Item>
            <Descriptions.Item label="Last Started">
              {stats ? (stats.last_started_at ? new Date(stats.last_started_at).toLocaleString() : 'Never') : '…'}
            </Descriptions.Item>
            <Descriptions.Item label="Total Running">
              {stats
                ? stats.total_running_seconds > 0
                  ? `${Math.floor(stats.total_running_seconds / 3600)}h ${Math.floor((stats.total_running_seconds % 3600) / 60)}m`
                  : '0m'
                : '…'}
            </Descriptions.Item>
          </Descriptions>
        </Card>

        {/* Mode selector + Save buttons */}
        <div style={{ marginBottom: 12, display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
          <Segmented
            size="small"
            value={mode}
            onChange={(v) => {
              setMode(v as 'raw' | 'visual');
            }}
            options={[
              { label: '📝 Raw', value: 'raw' },
              { label: '🎨 Visual', value: 'visual' },
            ]}
          />
          {hasChanges && <Tag color="orange">unsaved</Tag>}
          <Space style={{ marginLeft: 'auto' }}>
            {mode === 'raw' && (
              <Button size="small" onClick={handleValidateSyntax} icon={<CheckCircleOutlined />}>
                Validate
              </Button>
            )}
            <Button size="small" type="primary" onClick={handleSaveCompose} loading={saving} disabled={mode === 'raw' && !yamlValid}>
              Save
            </Button>
            <Button size="small" type="primary" icon={<CloudUploadOutlined />} onClick={handleDeploy} loading={deploying} disabled={mode === 'raw' && !yamlValid}>
              Save & Deploy
            </Button>
          </Space>
        </div>

        {/* Tabs — flattened */}
        <Card styles={{ body: { padding: 12 } }}>
          <Tabs
            defaultActiveKey="services"
            size="small"
            type="card"
            style={{ margin: '-12px' }}
            items={tabItems}
          />
        </Card>

        {/* Modals */}
        <Modal title="⚙️ Git Sync Configuration" open={syncModalOpen} onCancel={() => setSyncModalOpen(false)} footer={null}>
          {syncConfig && (
            <div>
              <Descriptions column={1} size="small">
                <Descriptions.Item label="Type">{syncConfig.sync_type}</Descriptions.Item>
                <Descriptions.Item label="Repo URL">{syncConfig.remote_url}</Descriptions.Item>
                <Descriptions.Item label="Branch">{syncConfig.remote_branch}</Descriptions.Item>
                <Descriptions.Item label="Status">{syncConfig.status}</Descriptions.Item>
              </Descriptions>
              <Space style={{ marginTop: 12 }}>
                <Button size="small" onClick={async () => { if (!id) return; await api.syncPull(id); message.success('Pulled'); }}>Pull</Button>
                <Button size="small" onClick={async () => { if (!id) return; await api.syncPush(id); message.success('Pushed'); }}>Push</Button>
              </Space>
            </div>
          )}
        </Modal>

        <Modal title="📝 Diff" open={diffModalOpen} onCancel={() => setDiffModalOpen(false)} width={800} footer={null}>
          {diffData && <DiffViewer diffText={diffData?.diff || ''} />}
        </Modal>
      </Content>
    </Layout>
  );
}