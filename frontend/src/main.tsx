import React, { useState, useEffect } from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter, Routes, Route, useNavigate, useLocation } from 'react-router-dom';
import { ConfigProvider, App as AntApp, Layout as AntLayout, Menu, theme, Switch, Space } from 'antd';
import esES from 'antd/locale/es_ES';
import { ClusterOutlined, ApiOutlined, DashboardOutlined, BellOutlined, SunOutlined, MoonOutlined } from '@ant-design/icons';
import { Stacks } from './pages/Stacks';
import { StackDetail } from './pages/StackDetail';
import { Agents } from './pages/Agents';
import { Dashboard } from './pages/Dashboard';
import { Notifiers } from './pages/Notifiers';

const { Sider, Content } = AntLayout;

function AppLayout({ children }: { children: React.ReactNode }) {
  const navigate = useNavigate();
  const location = useLocation();

  const currentKey = '/' + location.pathname.split('/').filter(Boolean)[0] || '/';

  const menuItems = [
    { key: '/', icon: <DashboardOutlined />, label: 'Dashboard' },
    { key: '/stacks', icon: <ClusterOutlined />, label: 'Stacks' },
    { key: '/notifiers', icon: <BellOutlined />, label: 'Notifiers' },
    { key: '/agents', icon: <ApiOutlined />, label: 'Agents' },
  ];

  return (
    <AntLayout style={{ minHeight: '100vh' }}>
      <Sider width={200} theme="light" style={{ borderRight: '1px solid #f0f0f0' }}>
        <div style={{ padding: '16px', textAlign: 'center', fontWeight: 'bold', fontSize: 16 }}>
          🐳 Dockpot
        </div>
        <Menu mode="inline" selectedKeys={[currentKey]} items={menuItems} onClick={({ key }) => navigate(key)} />
      </Sider>
      <Content>{children}</Content>
    </AntLayout>
  );
}

function App() {
  const [darkMode, setDarkMode] = useState(() => localStorage.getItem('dockpot-theme') === 'dark');

  useEffect(() => {
    localStorage.setItem('dockpot-theme', darkMode ? 'dark' : 'light');
  }, [darkMode]);

  return (
    <ConfigProvider
      theme={{
        algorithm: darkMode ? theme.darkAlgorithm : theme.defaultAlgorithm,
        token: { colorPrimary: '#1677ff', borderRadius: 6 },
      }}
      locale={esES}
    >
      <AntApp>
        <div style={{ position: 'fixed', top: 8, right: 8, zIndex: 1000 }}>
          <Space>
            <Switch
              checkedChildren={<MoonOutlined />}
              unCheckedChildren={<SunOutlined />}
              checked={darkMode}
              onChange={setDarkMode}
            />
          </Space>
        </div>
        <BrowserRouter>
          <Routes>
            <Route path="/" element={<AppLayout><Dashboard /></AppLayout>} />
            <Route path="/stacks" element={<AppLayout><Stacks /></AppLayout>} />
            <Route path="/stacks/:id" element={<StackDetail />} />
            <Route path="/notifiers" element={<AppLayout><Notifiers /></AppLayout>} />
            <Route path="/agents" element={<AppLayout><Agents /></AppLayout>} />
          </Routes>
        </BrowserRouter>
      </AntApp>
    </ConfigProvider>
  );
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode><App /></React.StrictMode>,
);