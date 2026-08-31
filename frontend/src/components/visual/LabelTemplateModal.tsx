// ── Label Template Modal ──
// Gallery-style modal for selecting label templates (Traefik, Caddy, NPM, etc.)
// and applying them to a service.

import { useState, useEffect, useCallback, useMemo } from 'react';
import { Modal, Card, Input, Button, Typography, Spin, Alert, Tag, Space, Row, Col, Empty, message } from 'antd';
import { ThunderboltOutlined, CheckOutlined } from '@ant-design/icons';
import { api, LabelTemplate } from '../../api/http';

const { Text } = Typography;

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

export interface LabelTemplateModalProps {
  open: boolean;
  onCancel: () => void;
  onApply: (labels: Record<string, string>) => void;
  serviceName: string;
  existingLabels: Record<string, string>;
}

/** Category display config */
const CATEGORY_META: Record<string, { emoji: string; color: string }> = {
  traefik: { emoji: '🏷️', color: '#1677ff' },
  caddy: { emoji: '🧢', color: '#52c41a' },
  npm: { emoji: '🌐', color: '#722ed1' },
  watchtower: { emoji: '🐳', color: '#fa8c16' },
  dozzle: { emoji: '📋', color: '#13c2c2' },
  adguard: { emoji: '🛡️', color: '#eb2f96' },
  portainer: { emoji: '🐳', color: '#2db7f5' },
  homepage: { emoji: '🏠', color: '#87d068' },
};

function getCategoryMeta(cat: string) {
  const m = CATEGORY_META[cat];
  if (m) return m;
  return { emoji: '📦', color: '#999' };
}

export default function LabelTemplateModal({
  open,
  onCancel,
  onApply,
  serviceName,
  existingLabels,
}: LabelTemplateModalProps) {
  const [templatesByCategory, setTemplatesByCategory] = useState<Record<string, LabelTemplate[]>>({});
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [variables, setVariables] = useState<Record<string, Record<string, string>>>({});

  // Load templates when modal opens
  useEffect(() => {
    if (!open) return;
    setLoading(true);
    setError(null);
    setSelected(new Set());
    setVariables({});
    api
      .listLabelTemplates()
      .then((data) => {
        setTemplatesByCategory(data);
        setLoading(false);
      })
      .catch((e) => {
        setError(e.message);
        setLoading(false);
      });
  }, [open]);

  // Toggle template selection
  const toggleTemplate = useCallback((name: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      return next;
    });
  }, []);

  // Update a variable for a template
  const setVar = useCallback(
    (templateName: string, varName: string, value: string) => {
      setVariables((prev) => ({
        ...prev,
        [templateName]: { ...(prev[templateName] || {}), [varName]: value },
      }));
    },
    [],
  );

  // Find template by name across all categories
  const findTemplate = useCallback(
    (name: string): LabelTemplate | undefined => {
      for (const cat of Object.values(templatesByCategory)) {
        const found = cat.find((t) => t.name === name);
        if (found) return found;
      }
      return undefined;
    },
    [templatesByCategory],
  );

  // Apply selected templates
  const handleApply = useCallback(async () => {
    if (selected.size === 0) return;

    let allLabels: Record<string, string> = { ...existingLabels };

    for (const name of selected) {
      const tpl = findTemplate(name);
      if (!tpl) continue;

      // Build variables: use user input or defaults
      const vars: Record<string, string> = {};
      for (const v of tpl.variables || []) {
        vars[v.name] = variables[name]?.[v.name] || v.default || '';
      }

      try {
        const result = await api.renderLabelTemplate(name, serviceName, vars);
        allLabels = { ...allLabels, ...result.labels };
      } catch (e: any) {
        message.error(`Failed to render template '${name}': ${e.message}`);
        return;
      }
    }

    onApply(allLabels);
  }, [selected, findTemplate, variables, serviceName, existingLabels, onApply]);

  // Categories sorted by priority
  const categoryOrder = useMemo(
    () =>
      Object.keys(templatesByCategory).sort((a, b) => {
        const order = ['traefik', 'caddy', 'npm', 'watchtower', 'dozzle', 'adguard', 'portainer', 'homepage'];
        const ia = order.indexOf(a);
        const ib = order.indexOf(b);
        return (ia === -1 ? 99 : ia) - (ib === -1 ? 99 : ib);
      }),
    [templatesByCategory],
  );

  return (
    <Modal
      title={
        <span>
          <ThunderboltOutlined style={{ marginRight: 6 }} />
          Label Templates
        </span>
      }
      open={open}
      onCancel={onCancel}
      width={700}
      footer={
        <Space>
          <Button onClick={onCancel}>Cancel</Button>
          <Button
            type="primary"
            onClick={handleApply}
            disabled={selected.size === 0}
            icon={<CheckOutlined />}
          >
            Apply ({selected.size}) template{selected.size !== 1 ? 's' : ''}
          </Button>
        </Space>
      }
      destroyOnClose
    >
      {/* Loading */}
      {loading && (
        <div style={{ textAlign: 'center', padding: '60px 0' }}>
          <Spin tip="Loading templates..." />
        </div>
      )}

      {/* Error */}
      {error && (
        <Alert
          type="error"
          message="Failed to load templates"
          description={error}
          showIcon
          style={{ marginBottom: 12 }}
        />
      )}

      {/* Empty */}
      {!loading && !error && categoryOrder.length === 0 && (
        <Empty
          description={
            <span>
              No label templates found. Create YAML files in{' '}
              <Text code>templates/labels/</Text> to add templates.
            </span>
          }
          style={{ padding: '40px 0' }}
        />
      )}

      {/* Gallery */}
      {!loading && !error && categoryOrder.length > 0 && (
        <div className="dp-scroll" style={{ maxHeight: 480, overflow: 'auto' }}>
          <style>{SCROLL_STYLE}</style>
          {categoryOrder.map((cat) => {
            const meta = getCategoryMeta(cat);
            const templates = templatesByCategory[cat];
            return (
              <div key={cat} style={{ marginBottom: 20 }}>
                <Text
                  strong
                  style={{
                    fontSize: 14,
                    display: 'block',
                    marginBottom: 8,
                    color: meta.color,
                  }}
                >
                  {meta.emoji} {cat.toUpperCase()}
                </Text>
                <Row gutter={[8, 8]}>
                  {templates.map((tpl) => {
                    const isSelected = selected.has(tpl.name);
                    const hasVars = (tpl.variables || []).length > 0;
                    return (
                      <Col key={tpl.name} xs={12} sm={8} md={6}>
                        <Card
                          size="small"
                          hoverable
                          onClick={() => toggleTemplate(tpl.name)}
                          style={{
                            border: isSelected ? `2px solid ${meta.color}` : '1px solid #d9d9d9',
                            cursor: 'pointer',
                            transition: 'border 0.2s',
                          }}
                          bodyStyle={{ padding: 10 }}
                        >
                          <Text
                            strong
                            style={{ fontSize: 12, display: 'block', marginBottom: 2 }}
                          >
                            {tpl.name.replace(/^[a-z]+-/, '')}
                          </Text>
                          <Text
                            type="secondary"
                            style={{ fontSize: 11, display: 'block', marginBottom: 4 }}
                          >
                            {tpl.description}
                          </Text>
                          {isSelected && (
                            <Tag color="success" style={{ fontSize: 10, margin: 0 }}>
                              Selected
                            </Tag>
                          )}
                        </Card>

                        {/* Variables form (shown when selected and has vars) */}
                        {isSelected && hasVars && (
                          <div
                            style={{
                              marginTop: 6,
                              padding: '6px 8px',
                              background: '#f5f5f5',
                              borderRadius: 4,
                            }}
                          >
                            {(tpl.variables || []).map((v) => (
                              <div key={v.name} style={{ marginBottom: 4 }}>
                                <Text style={{ fontSize: 10, display: 'block', marginBottom: 1 }}>
                                  {v.name}
                                  {v.required && <Text style={{ color: 'red' }}> *</Text>}
                                </Text>
                                <Input
                                  size="small"
                                  placeholder={v.default || v.description}
                                  value={variables[tpl.name]?.[v.name] ?? v.default ?? ''}
                                  onChange={(e) => setVar(tpl.name, v.name, e.target.value)}
                                />
                              </div>
                            ))}
                          </div>
                        )}
                      </Col>
                    );
                  })}
                </Row>
              </div>
            );
          })}
        </div>
      )}
    </Modal>
  );
}