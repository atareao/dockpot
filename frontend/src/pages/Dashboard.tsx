import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Typography, Card, Row, Col, Statistic, Spin, Table, Tag, Space, Layout, Alert, List,
} from 'antd';
import {
  ClusterOutlined, CheckCircleOutlined, StopOutlined, CloseCircleOutlined,
  ContainerOutlined, HddOutlined, InfoCircleOutlined,
} from '@ant-design/icons';
import { api, Stack, DockerInfo } from '../api/http';

const { Title, Text } = Typography;
const { Content, Header } = Layout;

export function Dashboard() {
  const [stacks, setStacks] = useState<Stack[]>([]);
  const [dockerInfo, setDockerInfo] = useState<DockerInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();

  const loadData = async () => {
    try {
      setLoading(true);
      setError(null);
      const [stacksData, dockerData] = await Promise.all([
        api.listStacks(),
        api.getDockerInfo().catch(() => null),
      ]);
      setStacks(stacksData);
      setDockerInfo(dockerData);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadData(); }, []);

  const totalStacks = stacks.length;
  const runningStacks = stacks.filter((s) => s.status === 'running').length;
  const stoppedStacks = stacks.filter((s) => s.status === 'stopped' || s.status === 'exited').length;
  const errorStacks = stacks.filter((s) => s.status === 'error').length;

  // Recent activity: last 5 updated stacks
  const recentActivity = [...stacks]
    .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
    .slice(0, 5);

  const recentColumns = [
    {
      title: 'Name',
      dataIndex: 'name',
      key: 'name',
      render: (name: string, record: Stack) => (
        <a onClick={() => navigate(`/stacks/${record.id}`)}>{name}</a>
      ),
    },
    {
      title: 'Status',
      dataIndex: 'status',
      key: 'status',
      render: (status: string) => (
        <Tag color={status === 'running' ? 'green' : status === 'error' ? 'red' : 'default'}>
          {status}
        </Tag>
      ),
    },
    {
      title: 'Updated',
      dataIndex: 'updated_at',
      key: 'updated_at',
      render: (date: string) => (
        <Text type="secondary">{new Date(date).toLocaleString()}</Text>
      ),
    },
  ];

  if (error) {
    return (
      <Layout style={{ minHeight: '100vh' }}>
        <Header style={{ background: 'transparent', padding: '0 24px', display: 'flex', alignItems: 'center' }}>
          <Title level={3} style={{ margin: 0 }}>📊 Dashboard</Title>
        </Header>
        <Content style={{ padding: 24 }}>
          <Alert type="error" message="Failed to load dashboard" description={error} showIcon />
        </Content>
      </Layout>
    );
  }

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Header style={{ background: 'transparent', padding: '0 24px', display: 'flex', alignItems: 'center', justifyContent: 'space-between', borderBottom: '1px solid #f0f0f0' }}>
        <Title level={3} style={{ margin: 0 }}>📊 Dashboard</Title>
      </Header>
      <Content style={{ padding: 24 }}>
        {loading ? (
          <Spin size="large" style={{ display: 'block', margin: '40px auto' }} />
        ) : (
          <>
            {/* Stat Cards */}
            <Row gutter={[16, 16]}>
              <Col xs={24} sm={12} lg={6}>
                <Card hoverable>
                  <Statistic
                    title="Total Stacks"
                    value={totalStacks}
                    prefix={<ClusterOutlined />}
                    valueStyle={{ color: '#1677ff' }}
                  />
                </Card>
              </Col>
              <Col xs={24} sm={12} lg={6}>
                <Card hoverable>
                  <Statistic
                    title="Running"
                    value={runningStacks}
                    prefix={<CheckCircleOutlined />}
                    valueStyle={{ color: '#52c41a' }}
                    suffix={`/ ${totalStacks}`}
                  />
                </Card>
              </Col>
              <Col xs={24} sm={12} lg={6}>
                <Card hoverable>
                  <Statistic
                    title="Stopped"
                    value={stoppedStacks}
                    prefix={<StopOutlined />}
                    valueStyle={{ color: '#faad14' }}
                  />
                </Card>
              </Col>
              <Col xs={24} sm={12} lg={6}>
                <Card hoverable>
                  <Statistic
                    title="Errors"
                    value={errorStacks}
                    prefix={<CloseCircleOutlined />}
                    valueStyle={{ color: errorStacks > 0 ? '#ff4d4f' : '#52c41a' }}
                  />
                </Card>
              </Col>
            </Row>

            {/* Docker Info & Recent Activity */}
            <Row gutter={[16, 16]} style={{ marginTop: 24 }}>
              <Col xs={24} lg={12}>
                <Card title={<><InfoCircleOutlined /> Docker Info</>}>
                  {dockerInfo ? (
                    <Row gutter={[16, 16]}>
                      <Col span={12}>
                        <Statistic
                          title="Docker Engine"
                          value={dockerInfo.engine}
                          prefix={<ContainerOutlined />}
                          valueStyle={{ fontSize: 18 }}
                        />
                      </Col>
                      <Col span={12}>
                        <Statistic
                          title="API Version"
                          value={dockerInfo.version}
                          valueStyle={{ fontSize: 18 }}
                        />
                      </Col>
                      <Col span={8}>
                        <Statistic
                          title="Containers"
                          value={dockerInfo.containers_total}
                          suffix={`(${dockerInfo.containers_running} running)`}
                          valueStyle={{ fontSize: 16 }}
                        />
                      </Col>
                      <Col span={8}>
                        <Statistic
                          title="Images"
                          value={dockerInfo.images}
                          prefix={<HddOutlined />}
                          valueStyle={{ fontSize: 16 }}
                        />
                      </Col>
                      <Col span={8}>
                        <Statistic
                          title="Disk Usage"
                          value={formatBytes(dockerInfo.disk_usage)}
                          valueStyle={{ fontSize: 16 }}
                        />
                      </Col>
                    </Row>
                  ) : (
                    <Text type="secondary">Docker info not available</Text>
                  )}
                </Card>
              </Col>
              <Col xs={24} lg={12}>
                <Card title="🕐 Recent Activity">
                  {recentActivity.length > 0 ? (
                    <Table
                      dataSource={recentActivity}
                      columns={recentColumns}
                      rowKey="id"
                      pagination={false}
                      size="small"
                    />
                  ) : (
                    <Text type="secondary">No stacks yet</Text>
                  )}
                </Card>
              </Col>
            </Row>
          </>
        )}
      </Content>
    </Layout>
  );
}

function formatBytes(bytes: number): string {
  if (!bytes || bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const val = bytes / Math.pow(1024, i);
  return `${val.toFixed(1)} ${units[i]}`;
}