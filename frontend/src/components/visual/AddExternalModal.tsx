// ── Modal to add an external Docker resource (volume, network, config, secret) ──

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Modal, Form, Input, Select, Alert, Spin, Typography } from 'antd';

const { Text } = Typography;

// ── Types ──

export interface ExternalResource {
  name: string;
  driver?: string;
  scope?: string;
  internal?: boolean;
}

export interface AddExternalModalProps {
  open: boolean;
  onCancel: () => void;
  onConfirm: (name: string) => void;
  title: string;
  fetchUrl: string;
  existingNames: string[];
  labelKey: string;
}

// ── Component ──

function AddExternalModal({
  open,
  onCancel,
  onConfirm,
  title,
  fetchUrl,
  existingNames,
  labelKey,
}: AddExternalModalProps) {
  const [form] = Form.useForm<{ selectedName: string; resourceName?: string }>();

  const [resources, setResources] = useState<ExternalResource[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const fetchedRef = useRef(false);

  // Fetch resources when modal opens
  useEffect(() => {
    if (open && !fetchedRef.current) {
      fetchedRef.current = true;
      setLoading(true);
      setError(null);

      fetch(fetchUrl, { credentials: 'include' })
        .then((res) => {
          if (!res.ok) {
            throw new Error(`Failed to fetch ${labelKey.toLowerCase()}s (${res.status})`);
          }
          return res.json();
        })
        .then((data: ExternalResource[]) => {
          setResources(data);
          setLoading(false);
        })
        .catch((err: Error) => {
          setError(err.message);
          setLoading(false);
        });
    }
  }, [open, fetchUrl, labelKey]);

  // Reset form and fetch state when modal opens/closes
  useEffect(() => {
    if (open) {
      form.resetFields();
      fetchedRef.current = false;
    } else {
      // Clear data on close so next open re-fetches
      setResources([]);
      setError(null);
      setLoading(false);
      fetchedRef.current = false;
    }
  }, [open, form]);

  const existingSet = useMemo(() => new Set(existingNames), [existingNames]);

  const handleOk = useCallback(async () => {
    try {
      const values = await form.validateFields();
      const name = values.resourceName?.trim() || values.selectedName;
      onConfirm(name);
    } catch {
      // validation failed — Ant Design shows inline errors
    }
  }, [form, onConfirm]);

  // Build select options
  const selectOptions = useMemo(() => {
    return resources.map((r) => {
      const hasDriver = r.driver && (fetchUrl.includes('/volumes') || fetchUrl.includes('/networks'));
      const label = hasDriver ? `${r.name} — ${r.driver}` : r.name;
      return { value: r.name, label };
    });
  }, [resources, fetchUrl]);

  // Filter out already-existing names from select options
  const filteredOptions = useMemo(
    () => selectOptions.filter((opt) => !existingSet.has(opt.value)),
    [selectOptions, existingSet],
  );

  return (
    <Modal
      title={title}
      open={open}
      onCancel={onCancel}
      onOk={handleOk}
      okText="Add"
      destroyOnClose
      width={480}
    >
      {/* Loading state */}
      {loading && (
        <div style={{ textAlign: 'center', padding: '40px 0' }}>
          <Spin tip={`Loading ${labelKey.toLowerCase()}s...`} />
        </div>
      )}

      {/* Error state */}
      {!loading && error && (
        <Alert
          message="Failed to load resources"
          description={error}
          type="error"
          showIcon
          style={{ marginTop: 16 }}
        />
      )}

      {/* Empty state */}
      {!loading && !error && resources.length === 0 && (
        <div style={{ textAlign: 'center', padding: '40px 0' }}>
          <Text style={{ fontStyle: 'italic', color: 'rgba(0,0,0,0.35)' }}>
            No external {labelKey.toLowerCase()}s found.
          </Text>
        </div>
      )}

      {/* Form — always rendered but hidden when loading/error/empty */}
      {!loading && !error && resources.length > 0 && (
        <Form
          form={form}
          layout="vertical"
          style={{ marginTop: 16 }}
        >
          <Form.Item
            name="selectedName"
            label={`Select ${labelKey}`}
            rules={[{ required: true, message: `Please select a ${labelKey.toLowerCase()}` }]}
          >
            <Select
              size="small"
              showSearch
              placeholder={`Search or select a ${labelKey.toLowerCase()}...`}
              optionFilterProp="label"
              options={filteredOptions}
            />
          </Form.Item>

          <Form.Item
            name="resourceName"
            label={`Resource Name (optional)`}
            tooltip={`If set, creates an external ${labelKey.toLowerCase()} with this name referencing the selected resource. Leave empty to use the selected resource name directly.`}
            rules={[
              {
                validator: (_, value) => {
                  if (value && existingSet.has(value.trim())) {
                    return Promise.reject(
                      new Error(`A ${labelKey.toLowerCase()} named "${value.trim()}" already exists`),
                    );
                  }
                  return Promise.resolve();
                },
              },
            ]}
          >
            <Input
              size="small"
              placeholder={`Custom ${labelKey.toLowerCase()} name (optional)`}
            />
          </Form.Item>
        </Form>
      )}
    </Modal>
  );
}

export default AddExternalModal;