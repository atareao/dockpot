import React from 'react';

interface DiffViewerProps {
  diffText: string;
  height?: string | number;
}

export function DiffViewer({ diffText, height = 400 }: DiffViewerProps) {
  if (!diffText) {
    return (
      <div style={{
        background: '#0d1117', color: '#8b949e', padding: 16,
        borderRadius: 6, fontFamily: "'Cascadia Code', 'Fira Code', monospace", fontSize: 12,
        height, display: 'flex', alignItems: 'center', justifyContent: 'center',
      }}>
        No changes — working tree matches HEAD
      </div>
    );
  }

  const lines = diffText.split('\n');

  return (
    <div style={{
      background: '#0d1117', borderRadius: 6, overflow: 'auto', height,
      fontFamily: "'Cascadia Code', 'Fira Code', monospace", fontSize: 12, lineHeight: 1.6,
    }}>
      <table style={{ width: '100%', borderCollapse: 'collapse' }}>
        <tbody>
          {lines.map((line, i) => {
            const bg = line.startsWith('+') ? '#1b3b1b' :
                       line.startsWith('-') ? '#3b1b1b' :
                       line.startsWith('@@') ? '#1a2332' :
                       line.startsWith('diff --git') || line.startsWith('index ') || line.startsWith('---') || line.startsWith('+++') ? '#161b22' : 'transparent';
            const color = line.startsWith('+') ? '#3fb950' :
                         line.startsWith('-') ? '#f85149' :
                         line.startsWith('@@') ? '#8b949e' :
                         line.startsWith('diff --git') || line.startsWith('index ') ? '#58a6ff' :
                         line.startsWith('---') || line.startsWith('+++') ? '#58a6ff' : '#c9d1d9';
            const prefix = line.startsWith('+') ? '+' :
                          line.startsWith('-') ? '-' :
                          line.startsWith('@@') ? '@@' : ' ';

            return (
              <tr key={i} style={{ background: bg }}>
                <td style={{
                  width: 24, textAlign: 'right', padding: '0 8px', userSelect: 'none',
                  color: '#484f58', borderRight: '1px solid #21262d',
                }}>{i + 1}</td>
                <td style={{
                  width: 20, textAlign: 'center', padding: '0 4px', userSelect: 'none',
                  color: line.startsWith('+') ? '#3fb950' : line.startsWith('-') ? '#f85149' : '#484f58',
                }}>{prefix}</td>
                <td style={{ padding: '0 8px', color, whiteSpace: 'pre-wrap' }}>
                  {line || ' '}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}