import React, { useEffect, useRef, useState, useCallback } from 'react';
import { Button, Space, Tag } from 'antd';
import { ReloadOutlined, DownloadOutlined } from '@ant-design/icons';

interface TerminalProps { stackId: string; stackName: string; height?: string | number; }

function colorizeLine(line: string): React.ReactNode {
  // Detect lines with a timestamp in brackets like [2024-01-01T12:00:00] or [12:00:00]
  const timestampMatch = line.match(/^(\[[^\]]*\])\s*/);
  const lower = line.toUpperCase();

  let contentColor = '#c9d1d9';
  if (lower.includes('ERROR') || lower.includes('FATAL') || line.includes('] ERROR')) {
    contentColor = '#ff6b6b';
  } else if (lower.includes('WARN') || lower.includes('WARNING')) {
    contentColor = '#ffd93d';
  }

  if (timestampMatch) {
    const timestamp = timestampMatch[1];
    const rest = line.slice(timestampMatch[0].length);
    return (
      <div style={{ fontFamily: "'Cascadia Code', 'Fira Code', monospace", fontSize: 12, lineHeight: 1.5 }}>
        <span style={{ color: '#555' }}>{timestamp} </span>
        <span style={{ color: contentColor }}>{rest}</span>
      </div>
    );
  }

  return (
    <div style={{ color: contentColor, fontFamily: "'Cascadia Code', 'Fira Code', monospace", fontSize: 12, lineHeight: 1.5 }}>
      {line}
    </div>
  );
}

export function Terminal({ stackId, stackName, height = 400 }: TerminalProps) {
  const [connected, setConnected] = useState(false);
  const [lines, setLines] = useState<string[]>([]);
  const wsRef = useRef<WebSocket | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const connect = useCallback(() => {
    wsRef.current?.close();
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${protocol}//${window.location.host}/api/stacks/${stackId}/logs/ws`);
    wsRef.current = ws;
    ws.onopen = () => { setConnected(true); setLines([]); };
    ws.onmessage = (e) => setLines((prev) => [...prev.slice(-999), e.data]);
    ws.onclose = () => setConnected(false);
    ws.onerror = () => setConnected(false);
  }, [stackId]);

  useEffect(() => { connect(); return () => wsRef.current?.close(); }, [connect]);
  useEffect(() => { if (containerRef.current) containerRef.current.scrollTop = containerRef.current.scrollHeight; }, [lines]);

  return (
    <div>
      <Space style={{ marginBottom: 8 }}>
        <Tag color={connected ? 'green' : 'red'}>{connected ? '🟢 Connected' : '🔴 Disconnected'}</Tag>
        <Button size="small" icon={<ReloadOutlined />} onClick={connect}>Reconnect</Button>
        <Button size="small" icon={<DownloadOutlined />} onClick={() => {
          const a = document.createElement('a'); a.href = URL.createObjectURL(new Blob([lines.join('\n')], { type: 'text/plain' }));
          a.download = `${stackName}-logs.txt`; a.click();
        }}>Download</Button>
        <Button size="small" onClick={() => setLines([])}>Clear</Button>
        <Tag>{lines.length} lines</Tag>
      </Space>
      <div ref={containerRef} style={{
        background: '#0d1117', padding: 12, borderRadius: 6, overflow: 'auto', height,
        whiteSpace: 'pre-wrap', wordBreak: 'break-all',
      }}>
        {lines.length === 0
          ? <span style={{ color: '#8b949e', fontStyle: 'italic' }}>{connected ? 'Waiting for logs...' : 'Not connected. Click Reconnect.'}</span>
          : lines.map((line, i) => <div key={i}>{colorizeLine(line)}</div>)}
      </div>
    </div>
  );
}