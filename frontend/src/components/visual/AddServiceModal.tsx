// ── Modal to add a new service to docker-compose ──

import { useCallback, useEffect, useMemo } from 'react';
import { Modal, Form, Input, Select } from 'antd';
import type { ServiceDef } from '../../types/compose';

const RESTART_OPTIONS = [
  { value: 'no', label: 'no' },
  { value: 'always', label: 'always' },
  { value: 'on-failure', label: 'on-failure' },
  { value: 'unless-stopped', label: 'unless-stopped' },
];

export interface AddServiceModalProps {
  open: boolean;
  onCancel: () => void;
  onConfirm: (name: string, service: ServiceDef) => void;
  existingNames: string[];
}

function AddServiceModal({ open, onCancel, onConfirm, existingNames }: AddServiceModalProps) {
  const [form] = Form.useForm<{ name: string; image?: string; container_name?: string; restart: string }>();

  // Reset form fields when modal opens
  useEffect(() => {
    if (open) {
      form.resetFields();
    }
  }, [open, form]);

  const existingSet = useMemo(() => new Set(existingNames), [existingNames]);

  const handleOk = useCallback(async () => {
    try {
      const values = await form.validateFields();
      const { name, image, container_name, restart } = values;

      const service: ServiceDef = {
        ...(image ? { image } : {}),
        ...(container_name ? { container_name } : {}),
        restart: restart || 'no',
      };

      onConfirm(name.trim(), service);
    } catch {
      // validation failed — do nothing, Ant Design shows errors inline
    }
  }, [form, onConfirm]);

  return (
    <Modal
      title="Add Service"
      open={open}
      onCancel={onCancel}
      onOk={handleOk}
      okText="Add"
      destroyOnClose
      width={480}
    >
      <Form
        form={form}
        layout="vertical"
        initialValues={{ restart: 'no' }}
        style={{ marginTop: 16 }}
      >
        <Form.Item
          name="name"
          label="Service Name"
          rules={[
            { required: true, message: 'Service name is required' },
            { whitespace: true, message: 'Service name cannot be blank' },
            {
              validator: (_, value) => {
                if (value && existingSet.has(value.trim())) {
                  return Promise.reject(new Error(`Service "${value.trim()}" already exists`));
                }
                return Promise.resolve();
              },
            },
          ]}
        >
          <Input
            size="small"
            placeholder="web"
          />
        </Form.Item>

        <Form.Item name="image" label="Image">
          <Input
            size="small"
            placeholder="nginx:latest"
          />
        </Form.Item>

        <Form.Item name="container_name" label="Container Name">
          <Input
            size="small"
            placeholder="${STACK_NAME}-app"
          />
        </Form.Item>

        <Form.Item name="restart" label="Restart">
          <Select
            size="small"
            options={RESTART_OPTIONS}
          />
        </Form.Item>
      </Form>
    </Modal>
  );
}

export default AddServiceModal;