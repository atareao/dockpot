// ── Editable list of environment files as editable-card tabs ──
// Each tab = one env file, with dual editing: Visual (key-value table) or Raw (textarea).

import { useState, useCallback, useEffect } from 'react';
import { Tabs, Button, Input, Typography, theme, Modal, Segmented } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import EnvVarTable from './EnvVarTable';

const { Text } = Typography;
const { confirm } = Modal;

export interface EnvFileData {
  id: string;
  filename: string;
  content: string;
}

export interface EnvFileListProps {
  value: EnvFileData[];
  onUpsert: (filename: string, content: string) => Promise<void>;
  onDelete: (filename: string) => Promise<void>;
  onRename?: (oldName: string, newName: string) => Promise<void>;
  /** Global mode override — when set, hides the per-file Raw/Visual toggle */
  mode?: 'raw' | 'visual';
}

// ── Parse env content (KEY=VALUE lines) → Record<string, string> ──

function parseEnv(content: string): Record<string, string> {
  const result: Record<string, string> = {};
  for (const line of content.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const eqIdx = trimmed.indexOf('=');
    if (eqIdx === -1) continue;
    const key = trimmed.slice(0, eqIdx).trim();
    const val = trimmed.slice(eqIdx + 1).trim();
    if (key) {
      result[key] = val;
    }
  }
  return result;
}

// ── Serialize Record<string, string> → env content string ──

function serializeEnv(vars: Record<string, string>): string {
  return Object.entries(vars)
    .map(([k, v]) => `${k}=${v}`)
    .join('\n') + '\n';
}

/**
 * List of environment files rendered as editable-card tabs.
 * Each tab offers two editing modes: Visual (key-value table) and Raw (textarea).
 */
export default function EnvFileList({ value, onUpsert, onDelete, onRename, mode: globalMode }: EnvFileListProps) {
  const { token } = theme.useToken();
  const entries = value;
  const [activeKey, setActiveKey] = useState<string | undefined>(
    entries.length > 0 ? entries[0].filename : undefined,
  );
  const [renaming, setRenaming] = useState<string | null>(null);
  const [renameDraft, setRenameDraft] = useState('');
  const [rawContents, setRawContents] = useState<Record<string, string>>({});
  const [kvContents, setKvContents] = useState<Record<string, Record<string, string>>>({});
  // Per-file editor mode: false = Raw, true = Visual (key-value)
  const [editorModes, setEditorModes] = useState<Record<string, boolean>>({});

  // Initialize editing state when entries change
  useEffect(() => {
    const nextRaw: Record<string, string> = {};
    const nextKv: Record<string, Record<string, string>> = {};
    for (const e of entries) {
      nextRaw[e.filename] = rawContents[e.filename] ?? e.content;
      nextKv[e.filename] = kvContents[e.filename] ?? parseEnv(e.content);
    }
    setRawContents(nextRaw);
    setKvContents(nextKv);
  }, [entries]);

  const handleAdd = useCallback(() => {
    let idx = 1;
    const existing = new Set(entries.map((e) => e.filename));
    let name = `.env`;
    while (existing.has(name)) {
      name = `.env${idx}`;
      idx++;
    }
    onUpsert(name, '').then(() => {
      setActiveKey(name);
    });
  }, [entries, onUpsert]);

  const handleTabClose = useCallback(
    (targetKey: string) => {
      const env = entries.find((e) => e.filename === targetKey);
      confirm({
        title: `Delete '${targetKey}'?`,
        content: env?.content
          ? `This env file has ${env.content.split('\n').length} line(s).`
          : 'Are you sure?',
        okText: 'Delete',
        okType: 'danger',
        onOk: () => onDelete(targetKey),
      });
    },
    [entries, onDelete],
  );

  // Get the effective content string for a file (from whichever mode)
  const getContent = useCallback(
    (filename: string): string => {
      const isVisual = editorModes[filename];
      if (isVisual) {
        return serializeEnv(kvContents[filename] ?? {});
      }
      return rawContents[filename] ?? '';
    },
    [editorModes, kvContents, rawContents],
  );

  // Check if a file has unsaved changes
  const hasChanges = useCallback(
    (filename: string): boolean => {
      const original = entries.find((e) => e.filename === filename)?.content ?? '';
      return getContent(filename) !== original;
    },
    [entries, getContent],
  );

  // Save a file
  const handleSave = useCallback(
    async (filename: string) => {
      const content = getContent(filename);
      await onUpsert(filename, content);
    },
    [getContent, onUpsert],
  );

  const handleRename = useCallback(
    (oldName: string, newName: string) => {
      if (!newName.trim() || newName === oldName) return;
      if (onRename) {
        onRename(oldName, newName);
      } else {
        const content = getContent(oldName);
        onDelete(oldName);
        onUpsert(newName, content);
      }
      if (activeKey === oldName) setActiveKey(newName);
    },
    [getContent, onRename, onDelete, onUpsert, activeKey],
  );

  // Save on tab switch
  const handleTabChange = useCallback(
    async (key: string) => {
      if (activeKey && hasChanges(activeKey)) {
        await handleSave(activeKey);
      }
      setActiveKey(key);
    },
    [activeKey, handleSave, hasChanges],
  );

  // Empty state
  if (entries.length === 0) {
    return (
      <div style={{ textAlign: 'center', padding: '40px 0' }}>
        <Text style={{ color: token.colorTextQuaternary, fontStyle: 'italic' }}>
          No environment files defined.
        </Text>
        <div style={{ marginTop: 12 }}>
          <Button size="small" type="primary" icon={<PlusOutlined />} onClick={handleAdd}>
            Add .env file
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div>
      <Tabs
        type="editable-card"
        size="small"
        activeKey={activeKey}
        onChange={handleTabChange}
        onEdit={(targetKey, action) => {
          if (action === 'remove' && typeof targetKey === 'string') {
            handleTabClose(targetKey);
          }
        }}
        tabBarExtraContent={
          <Button size="small" icon={<PlusOutlined />} onClick={handleAdd} style={{ marginLeft: 4 }}>
            Add
          </Button>
        }
        items={entries.map((env) => {
          const changed = hasChanges(env.filename);
          const isVisual = editorModes[env.filename] ?? false;
          return {
            key: env.filename,
            label: renaming === env.filename ? (
              <Input
                size="small"
                value={renameDraft}
                onChange={(e) => setRenameDraft(e.target.value)}
                onBlur={() => {
                  handleRename(env.filename, renameDraft);
                  setRenaming(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    handleRename(env.filename, renameDraft);
                    setRenaming(null);
                  }
                  if (e.key === 'Escape') setRenaming(null);
                }}
                onClick={(e) => e.stopPropagation()}
                style={{ width: 120 }}
                autoFocus
              />
            ) : (
              <span
                onClick={(e) => {
                  e.stopPropagation();
                  setRenameDraft(env.filename);
                  setRenaming(env.filename);
                }}
                style={{ cursor: 'pointer' }}
              >
                {changed ? `${env.filename} *` : env.filename}
              </span>
            ),
            closable: true,
            children: (
              <EnvFileTab
                isVisual={globalMode ? globalMode === 'visual' : isVisual}
                changed={changed}
                rawValue={rawContents[env.filename] ?? env.content}
                kvValue={kvContents[env.filename] ?? parseEnv(env.content)}
                onRawChange={(content) =>
                  setRawContents((prev) => ({ ...prev, [env.filename]: content }))
                }
                onKvChange={(vars) =>
                  setKvContents((prev) => ({ ...prev, [env.filename]: vars }))
                }
                onModeChange={globalMode ? undefined : (visual) =>
                  setEditorModes((prev) => ({ ...prev, [env.filename]: visual }))
                }
                onSave={() => handleSave(env.filename)}
                token={token}
              />
            ),
          };
        })}
        style={{ marginTop: -4 }}
      />
    </div>
  );
}

// ── Single env file tab content ──

function EnvFileTab({
  isVisual,
  changed,
  rawValue,
  kvValue,
  onRawChange,
  onKvChange,
  onModeChange,
  onSave,
  token,
}: {
  isVisual: boolean;
  changed: boolean;
  rawValue: string;
  kvValue: Record<string, string>;
  onRawChange: (v: string) => void;
  onKvChange: (v: Record<string, string>) => void;
  onModeChange?: (visual: boolean) => void;
  onSave: () => void;
  token: ReturnType<typeof theme.useToken>['token'];
}) {
  const handleModeChange = useCallback(
    (v: any) => {
      const visual = v as boolean;
      if (visual) {
        onKvChange(parseEnv(rawValue));
      }
      if (!visual) {
        onRawChange(serializeEnv(kvValue));
      }
      onModeChange?.(visual);
    },
    [rawValue, kvValue, onKvChange, onRawChange, onModeChange],
  );

  return (
    <div style={{ paddingTop: 8 }}>
      {/* Toolbar */}
      <div
        style={{
          marginBottom: 10,
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          flexWrap: 'wrap',
          gap: 8,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {onModeChange && (
            <Segmented
              size="small"
              options={[
                { label: '📝 Raw', value: false },
                { label: '🎨 Visual', value: true },
              ]}
              value={isVisual}
              onChange={handleModeChange}
            />
          )}
          {changed && (
            <Text style={{ fontSize: 11, color: '#faad14' }}>⚠️ unsaved</Text>
          )}
        </div>
        {changed && (
          <Button size="small" type="primary" onClick={onSave}>
            Save
          </Button>
        )}
      </div>

      {/* Editor */}
      {isVisual ? (
        <EnvVarTable
          value={kvValue}
          onChange={onKvChange}
          title="Environment Variables"
          keyPlaceholder="VARIABLE_NAME"
          valuePlaceholder="value"
        />
      ) : (
        <textarea
          value={rawValue}
          onChange={(e) => onRawChange(e.target.value)}
          rows={14}
          style={{
            width: '100%',
            fontFamily: '"Cascadia Code", "Fira Code", "Consolas", monospace',
            fontSize: 13,
            padding: 10,
            borderRadius: 6,
            border: `1px solid ${token.colorBorder}`,
            background: token.colorBgElevated,
            color: token.colorText,
            resize: 'vertical',
            lineHeight: 1.5,
            outline: 'none',
          }}
          placeholder="# Example env file&#10;DB_HOST=localhost&#10;DB_PORT=5432&#10;DB_USER=user"
        />
      )}
    </div>
  );
}