import { useMemo, useCallback } from 'react';
import { Input, Button, Typography, Tooltip } from 'antd';
import { DeleteOutlined, PlusOutlined } from '@ant-design/icons';
import { useTheme } from '../../main';

const { Text } = Typography;

interface EnvVarTableProps {
  value: Record<string, string>;
  onChange: (v: Record<string, string>) => void;
  title: string;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
}

interface EnvRow {
  id: number;
  keyName: string;
  val: string;
}

let nextId = 0;

function entriesToRows(record: Record<string, string>): EnvRow[] {
  return Object.entries(record).map(([k, v]) => ({
    id: nextId++,
    keyName: k,
    val: v,
  }));
}

function rowsToEntries(rows: EnvRow[]): Record<string, string> {
  const result: Record<string, string> = {};
  for (const r of rows) {
    if (r.keyName.trim()) {
      result[r.keyName.trim()] = r.val;
    }
  }
  return result;
}

export default function EnvVarTable({
  value,
  onChange,
  title,
  keyPlaceholder = 'KEY',
  valuePlaceholder = 'VALUE',
}: EnvVarTableProps) {
  const { darkMode } = useTheme();

  const rows = useMemo(() => entriesToRows(value), [value]);

  const handleKeyChange = useCallback(
    (id: number, newKey: string) => {
      const updated = rows.map((r) => (r.id === id ? { ...r, keyName: newKey } : r));
      onChange(rowsToEntries(updated));
    },
    [rows, onChange],
  );

  const handleValueChange = useCallback(
    (id: number, newVal: string) => {
      const updated = rows.map((r) => (r.id === id ? { ...r, val: newVal } : r));
      onChange(rowsToEntries(updated));
    },
    [rows, onChange],
  );

  const handleDelete = useCallback(
    (id: number) => {
      const updated = rows.filter((r) => r.id !== id);
      onChange(rowsToEntries(updated));
    },
    [rows, onChange],
  );

  const handleAdd = useCallback(() => {
    const newRow: EnvRow = { id: nextId++, keyName: '', val: '' };
    const updated = [...rows, newRow];
    onChange(rowsToEntries(updated));
  }, [rows, onChange]);

  const borderColor = darkMode ? '#303030' : '#f0f0f0';
  const inputBg = darkMode ? '#141414' : '#fff';
  const headerBg = darkMode ? '#1d1d1d' : '#fafafa';

  return (
    <div>
      {/* Title */}
      <Text strong style={{ fontSize: 13, display: 'block', marginBottom: 8 }}>
        {title}
      </Text>

      {/* Header row */}
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: '1fr 1fr 48px',
          gap: 8,
          padding: '6px 8px',
          background: headerBg,
          border: `1px solid ${borderColor}`,
          borderBottom: 'none',
          borderRadius: '6px 6px 0 0',
        }}
      >
        <Text style={{ fontSize: 12, color: darkMode ? '#888' : '#999' }}>Key</Text>
        <Text style={{ fontSize: 12, color: darkMode ? '#888' : '#999' }}>Value</Text>
        <Text style={{ fontSize: 12, color: darkMode ? '#888' : '#999', textAlign: 'center' }} />
      </div>

      {/* Rows */}
      <div
        style={{
          border: `1px solid ${borderColor}`,
          borderTop: 'none',
          borderRadius: rows.length === 0 ? '0 0 6px 6px' : 0,
        }}
      >
        {rows.length === 0 ? (
          <div
            style={{
              padding: '12px 8px',
              textAlign: 'center',
              color: darkMode ? '#555' : '#bbb',
              fontSize: 13,
            }}
          >
            No variables defined
          </div>
        ) : (
          rows.map((row, index) => (
            <div
              key={row.id}
              style={{
                display: 'grid',
                gridTemplateColumns: '1fr 1fr 48px',
                gap: 8,
                padding: '6px 8px',
                background: inputBg,
                borderBottom: index < rows.length - 1 ? `1px solid ${borderColor}` : 'none',
                alignItems: 'center',
              }}
            >
              <Tooltip title="Variable name (e.g. DB_HOST)">
                <Input
                  size="small"
                  placeholder={keyPlaceholder}
                  value={row.keyName}
                  onChange={(e) => handleKeyChange(row.id, e.target.value)}
                  variant="borderless"
                  style={{
                    background: darkMode ? '#1d1d1d' : '#f5f5f5',
                    borderRadius: 4,
                    fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace',
                    fontSize: 13,
                  }}
                />
              </Tooltip>
              <Tooltip title="Variable value (e.g. localhost)">
                <Input
                  size="small"
                  placeholder={valuePlaceholder}
                  value={row.val}
                  onChange={(e) => handleValueChange(row.id, e.target.value)}
                  variant="borderless"
                  style={{
                    background: darkMode ? '#1d1d1d' : '#f5f5f5',
                    borderRadius: 4,
                    fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace',
                    fontSize: 13,
                  }}
                />
              </Tooltip>
              <div style={{ display: 'flex', justifyContent: 'center' }}>
                <Button
                  type="text"
                  size="small"
                  danger
                  icon={<DeleteOutlined />}
                  onClick={() => handleDelete(row.id)}
                />
              </div>
            </div>
          ))
        )}
      </div>

      {/* Add button */}
      <Button
        type="dashed"
        size="small"
        icon={<PlusOutlined />}
        onClick={handleAdd}
        style={{
          width: '100%',
          marginTop: 8,
          fontSize: 13,
        }}
      >
        Add Variable
      </Button>
    </div>
  );
}