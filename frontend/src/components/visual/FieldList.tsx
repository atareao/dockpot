import { useCallback } from 'react';
import { Input, Button, Typography, Tooltip } from 'antd';
import { PlusOutlined, CloseOutlined } from '@ant-design/icons';

const { Text } = Typography;

export interface FieldListProps {
  /** Array of strings to display and edit */
  value: string[];
  /** Called when the list changes (add, remove, edit) */
  onChange: (v: string[]) => void;
  /** Label displayed above the list */
  title: string;
  /** Placeholder text for each input field */
  placeholder?: string;
}

/**
 * A generic editable list of strings.
 *
 * Each row is a compact input with a delete button.
 * An "Add" button at the bottom appends a new empty entry.
 */
function FieldList({ value, onChange, title, placeholder = 'Enter value…' }: FieldListProps) {
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
      onChange(value.filter((_, i) => i !== index));
    },
    [value, onChange],
  );

  const handleAdd = useCallback(() => {
    onChange([...value, '']);
  }, [value, onChange]);

  return (
    <div>
      <Text strong style={{ fontSize: 13, display: 'block', marginBottom: 6 }}>
        {title}
      </Text>

      {value.map((item, index) => (
        <div
          key={index}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            marginBottom: 6,
          }}
        >
          <Tooltip title="Add a value. Press Enter to add another.">
            <Input
              size="small"
              value={item}
              placeholder={placeholder}
              onChange={e => handleChange(index, e.target.value)}
              style={{ flex: 1 }}
            />
          </Tooltip>
          <Button
            size="small"
            danger
            type="text"
            icon={<CloseOutlined />}
            onClick={() => handleRemove(index)}
            aria-label={`Remove item ${index + 1}`}
          />
        </div>
      ))}

      <Button
        size="small"
        type="dashed"
        icon={<PlusOutlined />}
        onClick={handleAdd}
        block
      >
        Add
      </Button>
    </div>
  );
}

export default FieldList;