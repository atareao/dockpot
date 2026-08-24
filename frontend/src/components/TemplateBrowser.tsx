import { useState, useEffect } from 'react';
import { Modal, Input, Typography, Space, Button, Tag, Spin, App as AntApp } from 'antd';

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

export function TemplateBrowser({ open, onClose, onSelect }: TemplateBrowserProps) {
  const [templates, setTemplates] = useState<Template[]>([]);
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<Template | null>(null);
  const [stackName, setStackName] = useState('');
  const [variables, setVariables] = useState<Record<string, string>>({});
  const [composePreview, setComposePreview] = useState('');
  const { message } = AntApp.useApp();

  useEffect(() => {
    if (!open) return;
    setLoading(true);
    fetch('/api/templates').then(r => r.json()).then(data => {
      setTemplates(data);
      setSelected(null);
      setStackName('');
      setVariables({});
    }).catch(e => message.error('Failed to load templates: ' + e.message))
    .finally(() => setLoading(false));
  }, [open]);

  const handleSelect = (tpl: Template) => {
    setSelected(tpl);
    setStackName(tpl.name);
    const defaults: Record<string, string> = {};
    tpl.variables.forEach(v => { defaults[v.name] = v.default; });
    setVariables(defaults);
    setComposePreview(tpl.compose.replace(/\$\{STACK_NAME\}/g, tpl.name));
  };

  const handleVariableChange = (name: string, value: string) => {
    const updated = { ...variables, [name]: value };
    setVariables(updated);
    // Re-render preview
    if (selected) {
      let preview = selected.compose.replace(/\$\{STACK_NAME\}/g, stackName);
      for (const [k, v] of Object.entries(updated)) {
        preview = preview.replace(new RegExp(`\\$\\{${k}(:-[^}]*)?\\}`, 'g'), v || '');
      }
      setComposePreview(preview);
    }
  };

  const handleUse = () => {
    if (!selected) return;
    onSelect(stackName || selected.name, composePreview);
    onClose();
  };

  const categories = [...new Set(templates.map(t => t.category))];

  return (
    <Modal
      title="📦 Template Library"
      open={open}
      onCancel={onClose}
      footer={null}
      width={800}
    >
      {loading ? <Spin style={{ display: 'block', margin: '20px auto' }} /> : (
        <div style={{ display: 'flex', gap: 16 }}>
          <div style={{ width: 260, flexShrink: 0 }}>
            {categories.map(cat => (
              <div key={cat} style={{ marginBottom: 8 }}>
                <Text strong style={{ textTransform: 'capitalize', fontSize: 11, color: '#888' }}>{cat}</Text>
                {templates.filter(t => t.category === cat).map(tpl => (
                  <div key={tpl.name}
                    onClick={() => handleSelect(tpl)}
                    style={{
                      padding: '6px 8px', margin: '2px 0', borderRadius: 4, cursor: 'pointer',
                      background: selected?.name === tpl.name ? '#e6f4ff' : 'transparent',
                    }}
                  >
                    <Text strong style={{ fontSize: 13 }}>{tpl.name}</Text>
                    <Text type="secondary" style={{ display: 'block', fontSize: 11 }}>{tpl.description}</Text>
                  </div>
                ))}
              </div>
            ))}
          </div>
          <div style={{ flex: 1 }}>
            {selected ? (
              <>
                <Space style={{ marginBottom: 8 }}>
                  <Text>Stack name:</Text>
                  <Input value={stackName} onChange={e => {
                    setStackName(e.target.value);
                    let preview = selected.compose.replace(/\$\{STACK_NAME\}/g, e.target.value);
                    for (const [k, v] of Object.entries(variables)) {
                      preview = preview.replace(new RegExp(`\\$\\{${k}(:-[^}]*)?\\}`, 'g'), v || '');
                    }
                    setComposePreview(preview);
                  }} style={{ width: 200 }} />
                </Space>
                {selected.variables.map(v => (
                  <div key={v.name} style={{ marginBottom: 4 }}>
                    <Space style={{ width: '100%' }}>
                      <Text style={{ width: 140, fontSize: 12 }}>{v.name}</Text>
                      <Input
                        size="small"
                        placeholder={v.description}
                        value={variables[v.name] || ''}
                        onChange={e => handleVariableChange(v.name, e.target.value)}
                        style={{ flex: 1 }}
                      />
                      {v.required && <Tag color="red" style={{ fontSize: 10 }}>required</Tag>}
                    </Space>
                  </div>
                ))}
                <div style={{
                  background: '#1e1e1e', color: '#d4d4d4', padding: 8, borderRadius: 4,
                  marginTop: 8, overflow: 'auto', maxHeight: 200, fontSize: 11,
                  fontFamily: "'Cascadia Code', monospace", whiteSpace: 'pre-wrap',
                }}>
                  {composePreview}
                </div>
                <Button type="primary" onClick={handleUse} style={{ marginTop: 8 }}>
                  Use Template
                </Button>
              </>
            ) : <Text type="secondary">Select a template from the left</Text>}
          </div>
        </div>
      )}
    </Modal>
  );
}