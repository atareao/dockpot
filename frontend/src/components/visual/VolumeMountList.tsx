import { useCallback } from 'react';
import { Input, Select, Button, Tooltip } from 'antd';
import {
  PlusOutlined,
  DeleteOutlined,
  WarningFilled,
} from '@ant-design/icons';


export interface VolumeMountListProps {
  value: string[];
  onChange: (value: string[]) => void;
  /** Names of top-level volumes defined in the compose (for autocomplete) */
  volumeNames?: string[];
}

/**
 * Editable volume mounts list for docker-compose visual editor.
 *
 * Each entry is a string like "volume_name:/container/path" or "./data:/path:ro".
 * When volumeNames are provided, the source part gets a Select with autocomplete.
 * A mount is only valid when it has both source and target (contains a ':').
 */
export function VolumeMountList({ value, onChange, volumeNames }: VolumeMountListProps) {
  const handleChange = useCallback(
    (index: number, newVal: string) => {
      const next = [...value];
      next[index] = newVal;
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

  // Split a mount string into source and target+options
  const parseMount = (mount: string): { source: string; rest: string } => {
    const colonIdx = mount.indexOf(':');
    if (colonIdx === -1) return { source: mount, rest: '' };
    return { source: mount.slice(0, colonIdx), rest: mount.slice(colonIdx) };
  };

  // A mount is valid if it has a source AND a target (contains ':')
  const isValidMount = (mount: string): boolean => {
    const trimmed = mount.trim();
    if (!trimmed) return true; // empty is ok (not yet filled)
    const colonIdx = trimmed.indexOf(':');
    if (colonIdx === -1) return false; // no target path
    const source = trimmed.slice(0, colonIdx).trim();
    const target = trimmed.slice(colonIdx + 1).trim();
    if (!source) return false;
    if (!target) return false;
    // Target must be an absolute path
    if (!target.startsWith('/')) return false;
    return true;
  };

  if (value.length === 0) {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        <div style={{ color: '#8b949e', fontSize: 13, fontStyle: 'italic' }}>
          No volume mounts defined.
        </div>
        <Button size="small" icon={<PlusOutlined />} onClick={handleAdd}>
          Add Volume Mount
        </Button>
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {value.map((mount, i) => {
        const { source, rest } = parseMount(mount);
        const empty = mount.trim() === '';
        const valid = empty || isValidMount(mount);
        return (
          <div
            key={i}
            style={{
              display: 'flex',
              gap: 4,
              alignItems: 'center',
              padding: '2px 4px',
              borderRadius: 6,
              border: `1px solid ${valid ? 'transparent' : '#ff4d4f'}`,
            }}
          >
            {!empty && !valid && (
              <Tooltip title="Invalid mount. Format: source:/container/path (target must be absolute)">
                <WarningFilled style={{ color: '#ff4d4f', fontSize: 13, flexShrink: 0 }} />
              </Tooltip>
            )}
            {volumeNames && volumeNames.length > 0 ? (
              <Tooltip title="Volume name or host path">
                <Select
                  size="small"
                  showSearch
                  allowClear
                  placeholder="volume"
                  value={source || undefined}
                  onChange={(v) => handleChange(i, v ? `${v}${rest}` : '')}
                  style={{ width: 140, flexShrink: 0 }}
                  options={volumeNames.map((n) => ({ value: n, label: n }))}
                />
              </Tooltip>
            ) : (
              <Tooltip title="Volume name or host path">
                <Input
                  size="small"
                  value={source}
                  placeholder="volume_name"
                  onChange={(e) => handleChange(i, `${e.target.value}${rest}`)}
                  style={{ width: 130, flexShrink: 0, fontFamily: "'Cascadia Code', 'Fira Code', monospace", fontSize: 12 }}
                />
              </Tooltip>
            )}
            <Tooltip title="Container path (must be absolute, e.g. /usr/share/nginx/html)">
            <Input
              size="small"
              value={rest}
              placeholder=":/container/path:ro"
              status={!empty && !valid ? 'error' : undefined}
              onChange={(e) => {
                const val = e.target.value;
                const newRest = val.startsWith(':') ? val : `:${val}`;
                handleChange(i, `${source}${newRest}`);
              }}
              style={{ flex: 1, fontFamily: "'Cascadia Code', 'Fira Code', monospace", fontSize: 12 }}
            />
          </Tooltip>
            <Button
              size="small"
              danger
              icon={<DeleteOutlined />}
              onClick={() => handleRemove(i)}
              aria-label={`Remove volume mount ${i + 1}`}
            />
          </div>
        );
      })}
      <div>
        <Button size="small" icon={<PlusOutlined />} onClick={handleAdd}>
          Add Volume Mount
        </Button>
      </div>
    </div>
  );
}

export default VolumeMountList;