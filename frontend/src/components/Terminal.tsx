import React, { useEffect, useRef, useState, useCallback } from 'react';
import { Button, Space, Tag } from 'antd';
import { ReloadOutlined, DownloadOutlined } from '@ant-design/icons';

interface TerminalProps {
  stackId: string;
  stackName: string;
  height?: string | number;
}

export function Terminal({ stackId, stackName, height = 400 }: TerminalProps) {
  const [connected, setConnected] = useState(false);
  const [lines, setLines] = useState<string[]>([]);
  const wsRef = useRef<WebSocket | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const maxLines = 1000;

  const connect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
    }

    // Determine WS protocol
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const url = `${protocol}//${host}/api/stacks/${stackId}/logs/ws`;

    const ws = new WebSocket(url);
    wsRef.current = ws;

    ws.onopen = () => {
      setConnected(true);
      setLines([]);
    };

    ws.onmessage = (event) => {
      setLines((prev) => {
        const next = [...prev, event.data];
        return next.length > maxLines ? next.slice(-maxLines) : next;
      });
    };

    ws.onclose = () => {
      setConnected(false);
    };

    ws.onerror = () => {
      setConnected(false);
    };
  }, [stackId]);

  useEffect(() => {
    connect();
    return () => {
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, [connect]);

  // Auto-scroll to bottom
  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [lines]);

  const handleClear = () => {
    setLines([]);
  };

  const handleReconnect = () => {
    connect();
  };

  const handleDownload = () => {
    const blob = new Blob([lines.join('\n')], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${stackName}-logs.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div>
      <Space style={{ marginBottom: 8 }}>
        <Tag color={connected ? 'green' : 'red'}>
          {connected ? '🟢 Connected' : '🔴 Disconnected'}
        </Tag>
        <Button size="small" icon={<ReloadOutlined />} onClick={handleReconnect}>
          Reconnect
        </Button>
        <Button size="small" icon={<DownloadOutlined />} onClick={handleDownload}>
          Download
        </Button>
        <Button size="small" onClick={handleClear}>
          Clear
        </Button>
        {lines.length > 0 && (
          <Tag color="default">{lines.length} lines</Tag>
        )}
      </Space>
      <div
        ref={containerRef}
        style={{
          background: '#0d1117',
          color: '#c9d1d9',
          fontFamily: "'Cascadia Code', 'Fira Code', 'JetBrains Mono', 'Consolas', monospace",
          fontSize: 12,
          lineHeight: 1.5,
          padding: 12,
          borderRadius: 6,
          overflow: 'auto',
          height,
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-all',
        }}
      >
        {lines.length === 0 ? (
          <span style={{ color: '#8b949e', fontStyle: 'italic' }}>
            {connected ? 'Waiting for logs...' : 'Not connected. Click Reconnect.'}
          </span>
        ) : (
          lines.map((line, i) => (
            <div key={i}>{line}</div>
          ))
        )}
      </div>
    </div>
  );
}