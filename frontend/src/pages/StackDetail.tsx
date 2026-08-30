import { useState, useEffect, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Typography, Spin, Button, Space, Tag, Card, Descriptions, App as AntApp, Layout,
  Alert, Switch, Modal, Tabs, Divider, Dropdown,
} from 'antd';
import { ArrowLeftOutlined, PlayCircleOutlined, StopOutlined, ReloadOutlined,
  CloudUploadOutlined, CheckCircleOutlined, CloseCircleOutlined,
  DownloadOutlined, GithubOutlined, CodeOutlined, MoreOutlined,
} from '@ant-design/icons';
import { api, Stack, StackSync } from '../api/http';
import { YamlEditor } from '../components/YamlEditor';
import { Terminal } from '../components/Terminal';
import { DiffViewer } from '../components/DiffViewer';
import { useTheme } from '../main';

const { Title, Text } = Typography;
const { Content, Header } = Layout;

function isMobile() {
  return window.innerWidth < 768;
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
  const [envFiles, setEnvFiles] = useState<any[]>([]);
  const [notifiers, setNotifiers] = useState<any[]>([]);
  const [stackNotifiers, setStackNotifiers] = useState<string[]>([]);
  const [stats, setStats] = useState<any>(null);
  const [notifierModalOpen, setNotifierModalOpen] = useState(false);
  const [envContent, setEnvContent] = useState('');
  const [envFilename, setEnvFilename] = useState('.env');
  const [diffModalOpen, setDiffModalOpen] = useState(false);
  const [diffData, setDiffData] = useState<any>(null);
  const [editMode, setEditMode] = useState(false);
  const [yamlValid, setYamlValid] = useState(true);
  const [yamlErrors, setYamlErrors] = useState<string[]>([]);
  const { message } = AntApp.useApp();
  const navigate = useNavigate();
  const { darkMode } = useTheme();

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
          if (compose !== originalCompose) {
            await api.updateCompose(id, compose);
            setOriginalCompose(compose);
          }
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

        {/* Actions: icon-only always, less-used in dropdown */}
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
        {/* Info Card — siempre visible */}
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
          </Descriptions>
        </Card>

        <Card styles={{ body: { padding: 12 } }}>
          <Tabs
            defaultActiveKey="compose"
            size="small"
            style={{ margin: '-12px' }}
            items={[
              {
                key: 'compose',
                label: '📄 Compose',
                children: (
                  <div style={{ padding: '12px 12px 0' }}>
                    <Space style={{ marginBottom: 12 }}>
                      <Switch
                        checkedChildren={<><CheckCircleOutlined /> Edit</>}
                        unCheckedChildren="Preview"
                        size="small"
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
                        height={Math.min(500, window.innerHeight - 350)}
                      />
                    ) : (
                      <pre style={{
                        background: '#1e1e1e',
                        color: '#d4d4d4',
                        padding: 12,
                        borderRadius: 6,
                        overflow: 'auto',
                        maxHeight: Math.min(400, window.innerHeight - 300),
                        fontSize: 12,
                        margin: 0,
                      }}>
                        {compose}
                      </pre>
                    )}

                    {editMode && (
                      <div style={{
                        display: 'flex', gap: 8, marginTop: 12,
                        flexDirection: isMobile() ? 'column' : 'row',
                      }}>
                        <Button size="small" onClick={handleValidateSyntax} icon={<CheckCircleOutlined />} block={isMobile()}>
                          Validate
                        </Button>
                        <Button size="small" type="primary" onClick={handleSaveCompose} loading={saving} disabled={!yamlValid} block={isMobile()}>
                          Save
                        </Button>
                        <Button size="small" type="primary" icon={<CloudUploadOutlined />} onClick={handleDeploy} loading={deploying} block={isMobile()}>
                          Save & Deploy
                        </Button>
                      </div>
                    )}

                    {/* Env Files — dentro del mismo tab */}
                    <Divider style={{ marginTop: 16, marginBottom: 12 }} />
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                      <Text strong style={{ fontSize: 13 }}>🔤 Environment Files</Text>
                      <Button size="small" type="primary" onClick={() => {
                        setEnvFilename('.env'); setEnvContent(''); setNotifierModalOpen(true);
                      }}>Add .env</Button>
                    </div>
                    {envFiles.length === 0 ? <Text type="secondary" style={{ fontSize: 13 }}>No env files</Text> : (
                      <Space wrap>
                        {envFiles.map((env) => (
                          <Tag key={env.id} closable onClose={async () => {
                            await api.deleteEnvFile(stack.id, env.filename);
                            loadEnvFiles();
                          }} style={{ cursor: 'pointer' }} onClick={() => {
                            setEnvFilename(env.filename);
                            setEnvContent(env.content);
                            setNotifierModalOpen(true);
                          }}>
                            {env.filename}
                          </Tag>
                        ))}
                      </Space>
                    )}
                  </div>
                ),
              },
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
                key: 'stats',
                label: '📊 Stats',
                children: (
                  <div style={{ padding: '12px 12px 0' }}>
                    {stats ? (
                      <Descriptions column={{ xs: 1, sm: 2 }} size="small">
                        <Descriptions.Item label="Last Started">
                          {stats.last_started_at ? new Date(stats.last_started_at).toLocaleString() : 'Never'}
                        </Descriptions.Item>
                        <Descriptions.Item label="Total Running">
                          {stats.total_running_seconds > 0
                            ? `${Math.floor(stats.total_running_seconds / 3600)}h ${Math.floor((stats.total_running_seconds % 3600) / 60)}m`
                            : '0m'}
                        </Descriptions.Item>
                      </Descriptions>
                    ) : <Text type="secondary">No stats available</Text>}
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
                          <Tag
                            key={n.id}
                            color={stackNotifiers.includes(n.id) ? 'blue' : 'default'}
                            style={{ cursor: 'pointer' }}
                            onClick={async () => {
                              const current = await api.getStackNotifiers(stack.id);
                              const updated = current.includes(n.id)
                                ? current.filter((x: string) => x !== n.id)
                                : [...current, n.id];
                              await api.setStackNotifiers(stack.id, updated);
                              setStackNotifiers(updated);
                            }}
                          >
                            {n.name} ({n.notifier_type})
                          </Tag>
                        ))}
                      </Space>
                    )}
                  </div>
                ),
              },
            ]}
          />
        </Card>

        <Modal
          title="⚙️ Git Sync Configuration"
          open={syncModalOpen}
          onCancel={() => setSyncModalOpen(false)}
          footer={null}
          width={500}
        >
          <SyncConfigForm
            stackId={stack.id}
            stackName={stack.name}
            config={syncConfig}
            onSaved={() => {
              setSyncModalOpen(false);
              id && api.getSyncConfig(id).then(setSyncConfig).catch(() => {});
            }}
          />
        </Modal>

        <Modal
          title={`🔤 Edit ${envFilename}`}
          open={notifierModalOpen}
          onCancel={() => setNotifierModalOpen(false)}
          onOk={async () => {
            if (!id) return;
            await api.upsertEnvFile(id, envFilename, envContent);
            setNotifierModalOpen(false);
            loadEnvFiles();
          }}
          width={600}
        >
          <div style={{ marginBottom: 8 }}>
            <label>Filename:</label>
            <input value={envFilename} onChange={(e) => setEnvFilename(e.target.value)}
              style={{ width: '100%', padding: '4px 8px', borderRadius: 4, border: '1px solid #d9d9d9', fontSize: 14 }}
            />
          </div>
          <textarea value={envContent} onChange={(e) => setEnvContent(e.target.value)}
            rows={10}
            style={{ width: '100%', fontFamily: 'monospace', fontSize: 13, padding: 8, borderRadius: 4, border: '1px solid #d9d9d9' }}
            placeholder="DB_HOST=localhost&#10;DB_PORT=5432"
          />
        </Modal>

        <Modal
          title="📊 Git Diff"
          open={diffModalOpen}
          onCancel={() => setDiffModalOpen(false)}
          footer={null}
          width={800}
        >
          {diffData && (
            <>
              <Space style={{ marginBottom: 8 }}>
                <Tag>{diffData.files_changed?.length || 0} files changed</Tag>
                <Tag color="green">+{diffData.additions || 0}</Tag>
                <Tag color="red">-{diffData.deletions || 0}</Tag>
              </Space>
              <DiffViewer diffText={diffData.diff_text || ''} height={400} />
            </>
          )}
        </Modal>
      </Content>
    </Layout>
  );
}

// ───── Sync Config Form ─────

function SyncConfigForm({ stackId, stackName, config, onSaved }: {
  stackId: string;
  stackName: string;
  config: StackSync | null;
  onSaved: () => void;
}) {
  const [syncType, setSyncType] = useState(config?.sync_type || 'none');
  const [remoteUrl, setRemoteUrl] = useState(config?.remote_url || '');
  const [remoteBranch, setRemoteBranch] = useState(config?.remote_branch || 'main');
  const [authToken, setAuthToken] = useState('');
  const [syncing, setSyncing] = useState(false);
  const [pulling, setPulling] = useState(false);
  const { message } = AntApp.useApp();

  const handleSave = async () => {
    try {
      setSyncing(true);
      await api.setSyncConfig(stackId, {
        sync_type: syncType,
        remote_url: remoteUrl || undefined,
        remote_branch: remoteBranch || 'main',
        auth_token: authToken || undefined,
      });
      message.success(`Sync config saved for '${stackName}'`);
      onSaved();
    } catch (e: any) {
      message.error('Error: ' + e.message);
    } finally {
      setSyncing(false);
    }
  };

  const handlePull = async () => {
    try {
      setPulling(true);
      const result = await api.syncPull(stackId);
      message.success('🔽 ' + result.message);
      onSaved();
    } catch (e: any) {
      message.error('Pull failed: ' + e.message);
    } finally {
      setPulling(false);
    }
  };

  const handlePush = async () => {
    try {
      const result = await api.syncPush(stackId);
      message.success('🔼 ' + result.message);
      onSaved();
    } catch (e: any) {
      message.error('Push failed: ' + e.message);
    }
  };

  return (
    <Space direction="vertical" style={{ width: '100%' }}>
      <div>
        <label style={{ display: 'block', marginBottom: 4 }}>Sync Type</label>
        <select
          value={syncType}
          onChange={(e) => setSyncType(e.target.value)}
          style={{
            width: '100%', padding: '4px 8px',
            borderRadius: 4, border: '1px solid #d9d9d9', fontSize: 14,
          }}
        >
          <option value="none">None</option>
          <option value="git_dir">Local Git (no remote)</option>
          <option value="git_remote">Remote Git</option>
        </select>
      </div>

      {syncType === 'git_remote' && (
        <>
          <InputField label="Remote URL" value={remoteUrl} onChange={setRemoteUrl} placeholder="https://github.com/user/repo.git" />
          <InputField label="Branch" value={remoteBranch} onChange={setRemoteBranch} placeholder="main" />
          <InputField label="Auth Token (optional)" value={authToken} onChange={setAuthToken} placeholder="ghp_xxx..." type="password" />
        </>
      )}

      <Space style={{ marginTop: 8 }} wrap>
        <Button type="primary" onClick={handleSave} loading={syncing}>
          Save Config
        </Button>
        {syncType === 'git_remote' && (
          <>
            <Button onClick={handlePull} loading={pulling}>Pull</Button>
            <Button onClick={handlePush}>Push</Button>
          </>
        )}
      </Space>

      {config && config.last_commit && (
        <div style={{ marginTop: 8, fontSize: 12, color: '#888' }}>
          Last commit: <code>{config.last_commit.substring(0, 12)}</code>
          {config.last_synced_at && ` | ${new Date(config.last_synced_at).toLocaleString()}`}
        </div>
      )}
    </Space>
  );
}

function InputField({ label, value, onChange, placeholder, type = 'text' }: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: string;
}) {
  return (
    <div>
      <label style={{ display: 'block', marginBottom: 4 }}>{label}</label>
      <input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        style={{
          width: '100%', padding: '4px 8px',
          borderRadius: 4, border: '1px solid #d9d9d9', fontSize: 14,
        }}
      />
    </div>
  );
}