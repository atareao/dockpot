import { useRef } from 'react';
import Editor, { OnMount, OnChange } from '@monaco-editor/react';
import { Spin } from 'antd';
import type { editor } from 'monaco-editor';

interface YamlEditorProps {
  value: string;
  onChange?: (value: string) => void;
  readOnly?: boolean;
  height?: string | number;
  onValidate?: (isValid: boolean, errors: string[]) => void;
}

export function YamlEditor({
  value,
  onChange,
  readOnly = false,
  height = 400,
  onValidate,
}: YamlEditorProps) {
  const editorRef = useRef<Parameters<OnMount>[0] | null>(null);

  const handleMount: OnMount = (editor, monaco) => {
    editorRef.current = editor;

    // Register YAML language markers
    monaco.languages.registerDocumentFormattingEditProvider('yaml', {
      provideDocumentFormattingEdits(model: editor.ITextModel) {
        return [
          {
            text: model.getValue(),
            range: model.getFullModelRange(),
          },
        ];
      },
    });

    // Track diagnostics (syntax errors)
    monaco.editor.onDidCreateModel((model: editor.ITextModel) => {
      model.onDidChangeContent(() => {
        const markers = monaco.editor.getModelMarkers({ owner: model.getLanguageId() });
        const errors = markers
          .filter((m: editor.IMarker) => m.severity === monaco.MarkerSeverity.Error)
          .map((m: editor.IMarker) => `Line ${m.startLineNumber}: ${m.message}`);
        onValidate?.(errors.length === 0, errors);
      });
    });
  };

  const handleChange: OnChange = (val) => {
    onChange?.(val ?? '');
  };

  return (
    <Editor
      height={height}
      defaultLanguage="yaml"
      theme="vs-dark"
      value={value}
      onChange={handleChange}
      onMount={handleMount}
      loading={<Spin />}
      options={{
        readOnly,
        minimap: { enabled: false },
        fontSize: 13,
        lineNumbers: 'on',
        scrollBeyondLastLine: false,
        automaticLayout: true,
        wordWrap: 'on',
        tabSize: 2,
        renderWhitespace: 'boundary',
        bracketPairColorization: { enabled: true },
        formatOnPaste: true,
        formatOnType: true,
      }}
    />
  );
}