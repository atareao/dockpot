import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { ConfigProvider, App as AntApp } from 'antd';
import esES from 'antd/locale/es_ES';
import { Stacks } from './pages/Stacks';
import { StackDetail } from './pages/StackDetail';

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

function App() {
  return (
    <ConfigProvider theme={theme} locale={esES}>
      <AntApp>
        <BrowserRouter>
          <Routes>
            <Route path="/" element={<Root />} />
            <Route path="/stacks" element={<Stacks />} />
            <Route path="/stacks/:id" element={<StackDetail />} />
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