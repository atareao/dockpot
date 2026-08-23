import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter, Routes, Route, Navigate, useNavigate, useLocation } from 'react-router-dom';
import { ConfigProvider, App as AntApp, Layout as AntLayout, Menu } from 'antd';
import esES from 'antd/locale/es_ES';
import { ClusterOutlined, ApiOutlined } from '@ant-design/icons';
import { Stacks } from './pages/Stacks';
import { StackDetail } from './pages/StackDetail';
import { Agents } from './pages/Agents';

const { Sider, Content } = AntLayout;

const theme = {
  token: {
    colorPrimary: '#1677ff',
    borderRadius: 6,
  },
};

function Root() {
  const isAuthenticated = document.cookie.includes('token=') || !!localStorage.getItem('token');

  if (!isAuthenticated && !window.location.pathname.startsWith('/auth/')) {
    window.location.href = '/auth/login';
    return null;
  }

  return <Navigate to="/stacks" replace />;
}

function AppLayout({ children }: { children: React.ReactNode }) {
  const navigate = useNavigate();
  const location = useLocation();

  const currentKey = location.pathname.startsWith('/agents') ? '/agents' : '/stacks';

  const menuItems = [
    { key: '/stacks', icon: <ClusterOutlined />, label: 'Stacks' },
    { key: '/agents', icon: <ApiOutlined />, label: 'Agents' },
  ];

  return (
    <AntLayout style={{ minHeight: '100vh' }}>
      <Sider width={200} theme="light" style={{ borderRight: '1px solid #f0f0f0' }}>
        <div style={{ padding: '16px', textAlign: 'center', fontWeight: 'bold', fontSize: 16 }}>
          🐳 Dockpot
        </div>
        <Menu
          mode="inline"
          selectedKeys={[currentKey]}
          items={menuItems}
          onClick={({ key }) => navigate(key)}
        />
      </Sider>
      <Content>{children}</Content>
    </AntLayout>
  );
}

function App() {
  return (
    <ConfigProvider theme={theme} locale={esES}>
      <AntApp>
        <BrowserRouter>
          <Routes>
            <Route path="/" element={<Root />} />
            <Route path="/stacks" element={<AppLayout><Stacks /></AppLayout>} />
            <Route path="/stacks/:id" element={<StackDetail />} />
            <Route path="/agents" element={<AppLayout><Agents /></AppLayout>} />
          </Routes>
        </BrowserRouter>
      </AntApp>
    </ConfigProvider>
  );
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);