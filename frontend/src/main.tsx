import React, { useState, useEffect, createContext, useContext } from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter, Routes, Route, useNavigate, useLocation } from 'react-router-dom';
import { ConfigProvider, App as AntApp, Layout as AntLayout, theme, Switch, Space, Button, Typography } from 'antd';
import esES from 'antd/locale/es_ES';
import { SunOutlined, MoonOutlined, DashboardOutlined, SettingOutlined, LogoutOutlined } from '@ant-design/icons';
import { StackDetail } from './pages/StackDetail';
import { Dashboard } from './pages/Dashboard';
import { Settings } from './pages/Settings';

const { Header, Content } = AntLayout;
const { Text } = Typography;

// ── Theme Context ──
type ThemeCtx = { darkMode: boolean; toggleTheme: () => void };
const ThemeContext = createContext<ThemeCtx>({ darkMode: false, toggleTheme: () => {} });
export const useTheme = () => useContext(ThemeContext);

function AppLayout({ children }: { children: React.ReactNode }) {
  const navigate = useNavigate();
  const location = useLocation();
  const { darkMode, toggleTheme } = useTheme();

  const currentPath = '/' + location.pathname.split('/').filter(Boolean)[0] || '/';

  return (
    <AntLayout style={{ minHeight: '100vh', background: darkMode ? '#000' : '#f5f5f5' }}>
      {/* Top Navigation Bar */}
      <Header style={{
        background: darkMode ? '#141414' : '#fff',
        borderBottom: `1px solid ${darkMode ? '#303030' : '#f0f0f0'}`,
        padding: '0 12px',
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        height: 48,
        lineHeight: '48px',
      }}>
        {/* Left: Logo + Nav Links */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 4, flex: 1, minWidth: 0 }}>
          <img src="/icon.svg" alt="Dockpot" width="28" height="28" style={{ marginRight: 4 }} />
          <Text strong style={{ fontSize: 15, marginRight: 16, whiteSpace: 'nowrap' }}>Dockpot</Text>

          <Button
            type={currentPath === '/' ? 'primary' : 'text'}
            size="small"
            icon={<DashboardOutlined />}
            onClick={() => navigate('/')}
          >
            Dashboard
          </Button>
          <Button
            type={currentPath === '/settings' ? 'primary' : 'text'}
            size="small"
            icon={<SettingOutlined />}
            onClick={() => navigate('/settings')}
          >
            Settings
          </Button>
        </div>

        {/* Right: User, Theme, Logout */}
        <Space size="small">
          <Text style={{ fontSize: 13, color: darkMode ? '#aaa' : '#666' }}>dev@local</Text>
          <Switch
            size="small"
            checkedChildren={<MoonOutlined />}
            unCheckedChildren={<SunOutlined />}
            checked={darkMode}
            onChange={toggleTheme}
          />
          <Button
            size="small"
            icon={<LogoutOutlined />}
            onClick={() => {/* TODO: real logout */}}
            type="text"
          />
        </Space>
      </Header>

      <Content style={{ background: darkMode ? '#000' : '#f5f5f5' }}>
        {children}
      </Content>
    </AntLayout>
  );
}

function App() {
  const [darkMode, setDarkMode] = useState(() => localStorage.getItem('dockpot-theme') === 'dark');
  const toggleTheme = () => setDarkMode(prev => !prev);

  useEffect(() => {
    localStorage.setItem('dockpot-theme', darkMode ? 'dark' : 'light');
    // Sincronizar fondo del body con el tema
    document.body.style.background = darkMode ? '#000' : '#f5f5f5';
    document.body.style.margin = '0';
  }, [darkMode]);

  return (
    <ThemeContext.Provider value={{ darkMode, toggleTheme }}>
      <ConfigProvider
        theme={{
          algorithm: darkMode ? theme.darkAlgorithm : theme.defaultAlgorithm,
          token: { colorPrimary: '#1677ff', borderRadius: 6 },
        }}
        locale={esES}
      >
        <AntApp>
          <BrowserRouter>
            <Routes>
              <Route path="/" element={<AppLayout><Dashboard /></AppLayout>} />
              <Route path="/stacks/:id" element={<StackDetail />} />
              <Route path="/settings" element={<AppLayout><Settings /></AppLayout>} />
            </Routes>
          </BrowserRouter>
        </AntApp>
      </ConfigProvider>
    </ThemeContext.Provider>
  );
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode><App /></React.StrictMode>,
);