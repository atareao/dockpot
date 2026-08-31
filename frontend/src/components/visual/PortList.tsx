import { useCallback } from 'react';
import { Input, Button, Space, Typography, Tooltip } from 'antd';
import { PlusOutlined, DeleteOutlined, WarningFilled } from '@ant-design/icons';
import { useTheme } from '../../main';

const { Text } = Typography;

// ── Port mapping pattern ──
// Accepts:
//   "8080:80"                    — host:container
//   "8080:80/tcp"                — host:container/protocol
//   "127.0.0.1:8080:80"          — ip:host:container
//   "${VAR}:80"                  — compose variable:container
//   "${VAR:-80}:80"              — compose variable with default:container
//   "${VAR}:${PORT}"             — compose variable:compose variable
//   "${VAR:-8080}:${PORT:-80}"   — both as compose variables
const PORT_PATTERN = /^(\d{1,5}|[\w.-]+:\d{1,5}|\$\{[^}]*\}):(\d{1,5}|\$\{[^}]*\})(\/(tcp|udp|sctp))?$/;

interface PortListProps {
  value: string[];
  onChange: (v: string[]) => void;
}

/**
 * Editable list of Docker port mappings.
 *
 * Each entry is a string in the format:
 *   "8080:80"           — host:container
 *   "8080:80/tcp"       — host:container/protocol
 *   "127.0.0.1:8080:80" — ip:host:container
 *   "${VAR}:80"         — compose variable support
 */
export default function PortList({ value, onChange }: PortListProps) {
  const { darkMode } = useTheme();

  // --- Helpers ---

  const isValid = (entry: string): boolean => PORT_PATTERN.test(entry.trim());

  const handleChange = useCallback(
    (index: number, newValue: string) => {
      const next = [...value];
      next[index] = newValue;
      onChange(next);
    },
    [value, onChange],
  );

  const handleRemove = useCallback(
    (index: number) => {
      const next = value.filter((_, i) => i !== index);
      onChange(next);
    },
    [value, onChange],
  );

  const handleAdd = useCallback(() => {
    onChange([...value, '']);
  }, [value, onChange]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        handleAdd();
      }
    },
    [handleAdd],
  );

  // --- Styles ---

  const rowBg = darkMode ? '#1d1d1d' : '#fafafa';
  const borderColor = darkMode ? '#303030' : '#f0f0f0';

  // --- Render ---

  return (
    <div>
      {/* Header hint */}
      <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>
        Port mappings ({value.length})
      </Text>

      {/* List of port entries */}
      <Space direction="vertical" style={{ width: '100%' }} size={4}>
        {value.map((entry, index) => {
          const empty = entry.trim() === '';
          const valid = empty || isValid(entry);

          return (
            <div
              key={index}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 6,
                padding: '4px 8px',
                borderRadius: 6,
                background: rowBg,
                border: `1px solid ${valid ? borderColor : '#ff4d4f'}`,
                transition: 'border-color 0.15s',
              }}
            >
              {/* Validation warning icon */}
              {!empty && !valid && (
                <Tooltip title="Invalid format. Expected format: HOST:CONTAINER[/PROTOCOL], e.g. 8080:80 or 8080:80/tcp">
                  <WarningFilled style={{ color: '#ff4d4f', fontSize: 13, flexShrink: 0 }} />
                </Tooltip>
              )}

              {/* Port mapping input */}
              <Tooltip title="Port mapping format: HOST:CONTAINER[/PROTOCOL] (e.g. 8080:80 or 8080:80/tcp)">
                <Input
                  size="small"
                  variant="borderless"
                  value={entry}
                  onChange={(e) => handleChange(index, e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder="8080:80"
                  status={!empty && !valid ? 'error' : undefined}
                  style={{
                    flex: 1,
                    fontFamily: "'Cascadia Code', 'Fira Code', 'Consolas', monospace",
                    fontSize: 13,
                    background: 'transparent',
                  }}
                />
              </Tooltip>

              {/* Delete button */}
              <Button
                size="small"
                type="text"
                danger
                icon={<DeleteOutlined />}
                onClick={() => handleRemove(index)}
                aria-label={`Remove port mapping ${entry || index + 1}`}
              />
            </div>
          );
        })}
      </Space>

      {/* Add port button */}
      <Button
        size="small"
        type="dashed"
        icon={<PlusOutlined />}
        onClick={handleAdd}
        style={{ width: '100%', marginTop: 8 }}
      >
        Add Port
      </Button>
    </div>
  );
}