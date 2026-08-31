import React, { useEffect, useRef, useState, useCallback } from 'react';
import { Button, Space, Tag } from 'antd';
import { ReloadOutlined, DownloadOutlined } from '@ant-design/icons';
import { connectSSE } from '../api/http';

// ── Scrollbar styles (matching TemplateBrowser) ──
const SCROLL_STYLE = `
.dp-scroll::-webkit-scrollbar {
  width: 5px;
  height: 5px;
}
.dp-scroll::-webkit-scrollbar-track {
  background: transparent;
}
.dp-scroll::-webkit-scrollbar-thumb {
  background: rgba(128,128,128,0.25);
  border-radius: 4px;
}
.dp-scroll::-webkit-scrollbar-thumb:hover {
  background: rgba(128,128,128,0.4);
}
.dp-scroll {
  scrollbar-width: thin;
  scrollbar-color: rgba(128,128,128,0.25) transparent;
}
`;

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
  const esRef = useRef<EventSource | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const lineBuffer = useRef<string[]>([]);

  const connect = useCallback(() => {
    esRef.current?.close();
    setLines([]);
    lineBuffer.current = [];
    const es = connectSSE(`/api/stacks/${stackId}/logs/ws`);
    esRef.current = es;

    es.addEventListener('log', (e: Event) => {
      const msg = (e as MessageEvent).data;
      lineBuffer.current.push(msg);
      if (lineBuffer.current.length > 1000) {
        lineBuffer.current = lineBuffer.current.slice(-999);
      }
      setLines([...lineBuffer.current]);
    });

    es.addEventListener('error', (e: Event) => {
      const msg = (e as MessageEvent).data || 'Error del servidor';
      lineBuffer.current.push(`❌ ${msg}`);
      setLines([...lineBuffer.current]);
    });

    es.onopen = () => setConnected(true);
    es.onerror = () => {
      setConnected(false);
      // EventSource will auto-reconnect by default
    };
  }, [stackId]);

  useEffect(() => { connect(); return () => esRef.current?.close(); }, [connect]);
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
        <Button size="small" onClick={() => { setLines([]); lineBuffer.current = []; }}>Clear</Button>
        <Tag>{lines.length} lines</Tag>
      </Space>
      <style>{SCROLL_STYLE}</style>
      <div ref={containerRef} className="dp-scroll" style={{
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