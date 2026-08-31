import { Collapse, Input, InputNumber, Switch, Button, Typography, theme, Space, Tooltip } from 'antd';
import { DeleteOutlined } from '@ant-design/icons';
import { HealthcheckDef } from '../../types/compose';

const { Text } = Typography;
const { Panel } = Collapse;

interface HealthcheckFormProps {
  value?: HealthcheckDef;
  onChange: (v: HealthcheckDef | undefined) => void;
}

function HealthcheckForm({ value, onChange }: HealthcheckFormProps) {
  const { token } = theme.useToken();

  const update = (patch: Partial<HealthcheckDef>) => {
    onChange({ ...value, ...patch } as HealthcheckDef);
  };

  return (
    <Collapse
      size="small"
      style={{ background: token.colorBgContainer, borderRadius: 6 }}
    >
      <Panel
        key="healthcheck"
        header={
          <Space size={4}>
            <Text strong style={{ fontSize: 13 }}>Healthcheck</Text>
            {value?.disable && (
              <Text type="warning" style={{ fontSize: 11 }}>(disabled)</Text>
            )}
          </Space>
        }
        extra={
          <Button
            size="small"
            danger
            type="text"
            icon={<DeleteOutlined />}
            onClick={(e) => {
              e.stopPropagation();
              onChange(undefined);
            }}
            style={{ fontSize: 12 }}
          >
            Remove
          </Button>
        }
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {/* test */}
          <div>
            <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>test</Text>
            <Tooltip title="Health check command (e.g. curl -f http://localhost || exit 1)">
              <Input
                size="small"
                placeholder='e.g. ["CMD", "curl", "-f", "http://localhost"]'
                value={Array.isArray(value?.test) ? value.test.join(' ') : (value?.test ?? '')}
                onChange={(e) => update({ test: e.target.value || undefined })}
              />
            </Tooltip>
          </div>

          {/* interval + timeout row */}
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
            <div>
              <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>interval</Text>
              <Tooltip title="Time between checks (e.g. 30s, 10s)">
                <Input
                  size="small"
                  placeholder="30s"
                  value={value?.interval ?? ''}
                  onChange={(e) => update({ interval: e.target.value || undefined })}
                />
              </Tooltip>
            </div>
            <div>
              <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>timeout</Text>
              <Tooltip title="Max time for a check to complete (e.g. 10s)">
                <Input
                  size="small"
                  placeholder="10s"
                  value={value?.timeout ?? ''}
                  onChange={(e) => update({ timeout: e.target.value || undefined })}
                />
              </Tooltip>
            </div>
          </div>

          {/* retries + start_period row */}
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
            <div>
              <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>retries</Text>
              <Tooltip title="Consecutive failures before marking unhealthy">
                <InputNumber
                  size="small"
                  min={0}
                  placeholder="3"
                  value={value?.retries ?? null}
                  onChange={(v) => update({ retries: v ?? undefined })}
                  style={{ width: '100%' }}
                />
              </Tooltip>
            </div>
            <div>
              <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>start_period</Text>
              <Tooltip title="Grace period before health checks start (e.g. 5s)">
                <Input
                  size="small"
                  placeholder="5s"
                  value={value?.start_period ?? ''}
                  onChange={(e) => update({ start_period: e.target.value || undefined })}
                />
              </Tooltip>
            </div>
          </div>

          {/* start_interval + disable row */}
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
            <div>
              <Text style={{ fontSize: 12, display: 'block', marginBottom: 2 }}>start_interval</Text>
              <Tooltip title="Interval during start period (e.g. 5s)">
                <Input
                  size="small"
                  placeholder="5s"
                  value={value?.start_interval ?? ''}
                  onChange={(e) => update({ start_interval: e.target.value || undefined })}
                />
              </Tooltip>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, paddingTop: 18 }}>
              <Tooltip title="Disable the health check">
                <Switch
                  size="small"
                  checked={!!value?.disable}
                  onChange={(checked) => update({ disable: checked || undefined })}
                />
              </Tooltip>
              <Text style={{ fontSize: 12 }}>disable</Text>
            </div>
          </div>
        </div>
      </Panel>
    </Collapse>
  );
}

export default HealthcheckForm;