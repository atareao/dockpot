import { useState, useEffect, useMemo } from 'react';
import { Modal, Input, Typography, Button, Tag, Spin, Tabs, App as AntApp, theme, Empty } from 'antd';
import { SearchOutlined } from '@ant-design/icons';

const { Text } = Typography;

interface Template {
  name: string;
  description: string;
  category: string;
  compose: string;
  variables: { name: string; description: string; default: string; required: boolean }[];
}

interface TemplateBrowserProps {
  open: boolean;
  onClose: () => void;
  onSelect: (name: string, compose: string) => void;
}

const CATEGORY_EMOJIS: Record<string, string> = {
  ai: '🤖', analytics: '📈', app: '📱', cloud: '☁️', communication: '💬',
  content: '📝', database: '🗄️', dev: '🛠️', management: '⚙️', media: '🎬',
  middleware: '🔗', monitoring: '📊', productivity: '✅', proxy: '🌐',
  security: '🔒', storage: '💾', tools: '🧰', web: '🌍',
};

const CATEGORY_COLORS: Record<string, string> = {
  ai: 'purple', analytics: 'gold', app: 'blue', cloud: 'cyan',
  communication: 'geekblue', content: 'green', database: 'volcano',
  dev: 'orange', management: 'lime', media: 'magenta', middleware: 'blue',
  monitoring: 'geekblue', productivity: 'green', proxy: 'purple',
  security: 'red', storage: 'cyan', tools: 'default', web: 'blue',
};

// ── Scrollbar styles ──
const SCROLL_STYLE = `
.dp-scroll::-webkit-scrollbar {
  width: 5px;
  height: 5px;
}
.dp-scroll::-webkit-scrollbar-track {
  background: transparent;
}
.dp-scroll::-webkit-scrollbar-thumb {
  background: rgba(128,128,128,0.25);
  border-radius: 4px;
}
.dp-scroll::-webkit-scrollbar-thumb:hover {
  background: rgba(128,128,128,0.4);
}
.dp-scroll {
  scrollbar-width: thin;
  scrollbar-color: rgba(128,128,128,0.25) transparent;
}
`;

export function TemplateBrowser({ open, onClose, onSelect }: TemplateBrowserProps) {
  const [templates, setTemplates] = useState<Template[]>([]);
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<Template | null>(null);
  const [stackName, setStackName] = useState('');
  const [variables, setVariables] = useState<Record<string, string>>({});
  const [composePreview, setComposePreview] = useState('');
  const [search, setSearch] = useState('');
  const [activeCategory, setActiveCategory] = useState<string | null>(null);
  const { message } = AntApp.useApp();
  const { token } = theme.useToken();

  useEffect(() => {
    if (!open) return;
    setLoading(true);
    fetch('/api/templates').then(r => r.json()).then(data => {
      setTemplates(data);
      setSelected(null);
      setStackName('');
      setVariables({});
      setSearch('');
      setActiveCategory(null);
    }).catch(e => message.error('Failed to load templates: ' + e.message))
    .finally(() => setLoading(false));
  }, [open]);

  const categories = useMemo(() => {
    const cats = [...new Set(templates.map(t => t.category))].sort();
    return cats.map(c => ({
      name: c,
      count: templates.filter(t => t.category === c).length,
      emoji: CATEGORY_EMOJIS[c] || '📦',
    }));
  }, [templates]);

  const filtered = useMemo(() => {
    let result = templates;
    if (search) {
      const q = search.toLowerCase();
      result = result.filter(t =>
        t.name.toLowerCase().includes(q) ||
        t.description.toLowerCase().includes(q) ||
        t.category.toLowerCase().includes(q) ||
        t.variables.some(v => v.name.toLowerCase().includes(q))
      );
    }
    if (activeCategory) {
      result = result.filter(t => t.category === activeCategory);
    }
    return result;
  }, [templates, search, activeCategory]);

  const handleSelect = (tpl: Template) => {
    setSelected(tpl);
    setStackName(tpl.name);
    const defaults: Record<string, string> = {};
    tpl.variables.forEach(v => { defaults[v.name] = v.default; });
    setVariables(defaults);
    setComposePreview(tpl.compose.replace(/\$\{STACK_NAME\}/g, tpl.name));
  };

  const updatePreview = (name: string, vars: Record<string, string>, compose: string) => {
    let preview = compose.replace(/\$\{STACK_NAME\}/g, name);
    for (const [k, v] of Object.entries(vars)) {
      preview = preview.replace(new RegExp(`\\$\\{${k}(:-[^}]*)?\\}`, 'g'), v || '');
    }
    setComposePreview(preview);
  };

  const handleVariableChange = (varName: string, value: string) => {
    const updated = { ...variables, [varName]: value };
    setVariables(updated);
    if (selected) updatePreview(stackName, updated, selected.compose);
  };

  const handleNameChange = (value: string) => {
    setStackName(value);
    if (selected) updatePreview(value, variables, selected.compose);
  };

  const handleUse = () => {
    if (!selected) return;
    onSelect(stackName || selected.name, composePreview);
    onClose();
  };

  const isMobile = typeof window !== 'undefined' && window.innerWidth < 768;

  return (
    <Modal
      title="📦 Template Library"
      open={open}
      onCancel={onClose}
      footer={null}
      width={960}
      styles={{
        body: {
          padding: 0,
          height: '80vh',
          maxHeight: 700,
          display: 'flex',
          flexDirection: 'column',
        },
      }}
      destroyOnClose
    >
      <style>{SCROLL_STYLE}</style>
      {loading ? (
        <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', flex: 1 }}>
          <Spin size="large" />
        </div>
      ) : templates.length === 0 ? (
        <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <Empty description="No templates available" />
        </div>
      ) : (
        <>
          {/* ── Search + Category pills ── */}
          <div style={{
            padding: '12px 16px',
            borderBottom: `1px solid ${token.colorBorderSecondary}`,
            display: 'flex', flexDirection: 'column', gap: 8,
          }}>
            <Input.Search
              placeholder="Search templates by name, description or category…"
              allowClear
              value={search}
              onChange={e => setSearch(e.target.value)}
              prefix={<SearchOutlined style={{ color: token.colorTextTertiary }} />}
              size="small"
            />
            <div style={{
              display: 'flex', gap: 4, flexWrap: 'wrap',
              overflowX: 'auto', paddingBottom: 2,
              marginBottom: -4,
            }} className="dp-scroll">
              {categories.map(cat => {
                const isActive = activeCategory === cat.name;
                return (
                  <Tag.CheckableTag
                    key={cat.name}
                    checked={isActive}
                    onChange={() => setActiveCategory(isActive ? null : cat.name)}
                    style={{
                      padding: '0 10px',
                      fontSize: 12,
                      lineHeight: '24px',
                      borderRadius: 12,
                      border: isActive ? `1px solid ${token.colorPrimary}` : '1px solid transparent',
                      background: isActive ? token.colorPrimaryBg : 'transparent',
                      cursor: 'pointer',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {cat.emoji} {cat.name} {cat.count}
                  </Tag.CheckableTag>
                );
              })}
            </div>
          </div>

          {/* ── Main content ── */}
          <div style={{
            display: 'flex', flex: 1, overflow: 'hidden',
            flexDirection: isMobile ? 'column' : 'row',
          }}>
            {/* Left: template list */}
            <div style={{
              flex: isMobile ? 'none' : '0 0 280px',
              overflowY: 'auto',
              borderRight: isMobile ? 'none' : `1px solid ${token.colorBorderSecondary}`,
              borderBottom: isMobile ? `1px solid ${token.colorBorderSecondary}` : 'none',
              maxHeight: isMobile ? 180 : 'none',
            }} className="dp-scroll">
              {filtered.length === 0 ? (
                <div style={{ padding: 24, textAlign: 'center' }}>
                  <Text type="secondary">No templates match your search</Text>
                </div>
              ) : (
                filtered.map(tpl => {
                  const isSelected = selected?.name === tpl.name;
                  const color = CATEGORY_COLORS[tpl.category] || 'default';
                  return (
                    <div
                      key={tpl.name}
                      onClick={() => handleSelect(tpl)}
                      style={{
                        padding: '10px 14px',
                        cursor: 'pointer',
                        background: isSelected ? token.colorPrimaryBg : 'transparent',
                        borderLeft: isSelected ? `3px solid ${token.colorPrimary}` : '3px solid transparent',
                        borderBottom: `1px solid ${token.colorBorderSecondary}`,
                        transition: 'all 0.12s',
                      }}
                      onMouseEnter={e => { if (!isSelected) e.currentTarget.style.background = token.colorFillTertiary; }}
                      onMouseLeave={e => { if (!isSelected) e.currentTarget.style.background = 'transparent'; }}
                    >
                      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 2 }}>
                        <Text strong style={{
                          fontSize: 14,
                          color: isSelected ? token.colorPrimary : token.colorText,
                        }}>
                          {tpl.name}
                        </Text>
                        <Tag color={color} style={{ fontSize: 10, lineHeight: '16px', margin: 0, flexShrink: 0 }}>
                          {tpl.category}
                        </Tag>
                      </div>
                      <Text type="secondary" ellipsis style={{ fontSize: 12, display: 'block', lineHeight: 1.3 }}>
                        {tpl.description}
                      </Text>
                      <div style={{ marginTop: 4 }}>
                        {tpl.variables.length > 0 && (
                          <Text style={{ fontSize: 10, color: token.colorTextTertiary }}>
                            {tpl.variables.length} var{tpl.variables.length !== 1 ? 's' : ''}
                            {tpl.variables.some(v => v.required) && (
                              <> · <span style={{ color: token.colorError }}>{tpl.variables.filter(v => v.required).length} required</span></>
                            )}
                          </Text>
                        )}
                      </div>
                    </div>
                  );
                })
              )}
            </div>

            {/* Right: Config + Preview */}
            <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
              {selected ? (
                <Tabs
                  defaultActiveKey="config"
                  size="small"
                  style={{ padding: '12px 16px 0', flex: 1, display: 'flex', flexDirection: 'column' }}
                  items={[
                    {
                      key: 'config',
                      label: '⚙️ Configure',
                      children: (
                        <div style={{ overflowY: 'auto', flex: 1, paddingBottom: 12 }} className="dp-scroll">
                          {/* Stack name */}
                          <div style={{ marginBottom: 12 }}>
                            <Text strong style={{ fontSize: 13, display: 'block', marginBottom: 4 }}>Stack Name</Text>
                            <Input
                              value={stackName}
                              onChange={e => handleNameChange(e.target.value)}
                              placeholder="my-stack-name"
                            />
                          </div>

                          {/* Variables */}
                          {selected.variables.length > 0 && (
                            <div style={{ marginBottom: 12 }}>
                              <Text strong style={{ fontSize: 13, display: 'block', marginBottom: 6 }}>Variables</Text>
                              <div style={{ display: 'grid', gridTemplateColumns: isMobile ? '1fr' : '1fr 1fr', gap: 8 }}>
                                {selected.variables.map(v => (
                                  <div key={v.name}>
                                    <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>
                                      {v.name}
                                      {v.required && <Tag color="red" style={{ fontSize: 9, marginLeft: 4, lineHeight: '14px' }}>required</Tag>}
                                    </Text>
                                    <Input
                                      size="small"
                                      placeholder={v.description || v.default}
                                      value={variables[v.name] || ''}
                                      onChange={e => handleVariableChange(v.name, e.target.value)}
                                    />
                                  </div>
                                ))}
                              </div>
                            </div>
                          )}

                          <Button type="primary" onClick={handleUse} block size="large">
                            🚀 Create Stack
                          </Button>
                        </div>
                      ),
                    },
                    {
                      key: 'preview',
                      label: '👁️ Preview',
                      children: (
                        <pre style={{
                          background: '#1e1e1e', color: '#d4d4d4', padding: 12, borderRadius: 6,
                          overflow: 'auto', flex: 1, fontSize: 12, margin: 0,
                          fontFamily: "'Cascadia Code', 'JetBrains Mono', monospace",
                          lineHeight: 1.4, minHeight: 280,
                        }}>
                          {composePreview}
                        </pre>
                      ),
                    },
                  ]}
                />
              ) : (
                <div style={{
                  flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center',
                  padding: 24,
                }}>
                  <div style={{ textAlign: 'center', maxWidth: 400 }}>
                    <Text type="secondary" style={{ fontSize: 14, display: 'block', marginBottom: 16 }}>
                      Select a template from the list or use the search to find what you need
                    </Text>
                    <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, justifyContent: 'center' }}>
                      {categories.slice(0, 9).map(cat => (
                        <Tag
                          key={cat.name}
                          color={CATEGORY_COLORS[cat.name] || 'default'}
                          style={{ cursor: 'pointer' }}
                          onClick={() => setActiveCategory(cat.name)}
                        >
                          {cat.emoji} {cat.name} ({cat.count})
                        </Tag>
                      ))}
                      {categories.length > 9 && (
                        <Tag style={{ cursor: 'pointer' }} onClick={() => setActiveCategory(categories[9].name)}>
                          +{categories.length - 9} more…
                        </Tag>
                      )}
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>
        </>
      )}
    </Modal>
  );
}