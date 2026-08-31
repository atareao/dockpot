// ── Visual Compose Editor – main container ──
// Parses YAML → ComposeDefinition, renders structured editors via
// top-level tabs: Services, Volumes, Networks, Configs, Secrets, Env Files.
// Services tab contains its own sub-tabs (one per service).

import { useMemo, useCallback, useState } from 'react';
import { Alert, Tabs, Badge } from 'antd';
import { load, dump } from 'js-yaml';
import type { ComposeDefinition } from '../types/compose';
import ServiceList from './visual/ServiceList';
import VolumeList from './visual/VolumeList';
import NetworkList from './visual/NetworkList';
import ConfigSecretList from './visual/ConfigSecretList';
import EnvFileList from './visual/EnvFileList';
import type { EnvFileData } from './visual/EnvFileList';

export interface ComposeVisualEditorProps {
  value: string;
  onChange: (value: string) => void;
  /** Optional: environment files to show in a separate tab */
  envFiles?: EnvFileData[];
  onEnvFileUpsert?: (filename: string, content: string) => Promise<void>;
  onEnvFileDelete?: (filename: string) => Promise<void>;
}

/**
 * Parse a YAML compose string into a ComposeDefinition.
 * Returns null if parsing fails.
 */
function parseCompose(yaml: string): ComposeDefinition | null {
  if (!yaml || !yaml.trim()) return null;
  try {
    const parsed = load(yaml) as any;
    if (!parsed || typeof parsed !== 'object') return null;
    // Ensure services is always an object
    if (!parsed.services || typeof parsed.services !== 'object') {
      parsed.services = {};
    }
    // Ensure other sections exist
    parsed.volumes = parsed.volumes || {};
    parsed.networks = parsed.networks || {};
    parsed.configs = parsed.configs || {};
    parsed.secrets = parsed.secrets || {};
    return parsed as ComposeDefinition;
  } catch {
    return null;
  }
}

/**
 * Serialize a ComposeDefinition to YAML string.
 */
function serializeCompose(def: ComposeDefinition): string {
  const cleaned = { ...def };
  // Clean empty top-level sections (but keep services even if empty)
  for (const key of ['volumes', 'networks', 'configs', 'secrets'] as const) {
    const k = key as keyof ComposeDefinition;
    if (cleaned[k] && typeof cleaned[k] === 'object' && Object.keys(cleaned[k] as object).length === 0) {
      delete cleaned[k];
    }
  }
  return dump(cleaned, {
    indent: 2,
    lineWidth: -1,
    noRefs: true,
    sortKeys: false,
    forceQuotes: false,
  });
}

export function ComposeVisualEditor({ value, onChange, envFiles, onEnvFileUpsert, onEnvFileDelete }: ComposeVisualEditorProps) {
  const [parseError, setParseError] = useState<string | null>(null);

  // Parse YAML → ComposeDefinition (re-parse when value changes externally)
  const definition = useMemo(() => {
    const parsed = parseCompose(value);
    if (!parsed) {
      setParseError('Cannot parse this compose file as valid YAML. Switch to Raw mode to edit manually.');
      return null;
    }
    setParseError(null);
    return parsed;
  }, [value]);

  // Handle changes from any sub-editor
  const handleChange = useCallback(
    (updated: ComposeDefinition) => {
      try {
        const yaml = serializeCompose(updated);
        onChange(yaml);
        setParseError(null);
      } catch (e: any) {
        setParseError(`Serialization error: ${e.message}`);
      }
    },
    [onChange],
  );

  // Handlers for each section
  const handleServicesChange = useCallback(
    (services: ComposeDefinition['services']) => {
      if (!definition) return;
      handleChange({ ...definition, services });
    },
    [definition, handleChange],
  );

  const handleVolumesChange = useCallback(
    (volumes: ComposeDefinition['volumes']) => {
      if (!definition) return;
      handleChange({ ...definition, volumes });
    },
    [definition, handleChange],
  );

  const handleNetworksChange = useCallback(
    (networks: ComposeDefinition['networks']) => {
      if (!definition) return;
      handleChange({ ...definition, networks });
    },
    [definition, handleChange],
  );

  const handleConfigsChange = useCallback(
    (configs: ComposeDefinition['configs']) => {
      if (!definition) return;
      handleChange({ ...definition, configs });
    },
    [definition, handleChange],
  );

  const handleSecretsChange = useCallback(
    (secrets: ComposeDefinition['secrets']) => {
      if (!definition) return;
      handleChange({ ...definition, secrets });
    },
    [definition, handleChange],
  );

  // Collect volume and network names for Select components in ServiceCard
  const volumeNames = useMemo(
    () => (definition?.volumes ? Object.keys(definition.volumes) : []),
    [definition?.volumes],
  );
  const networkNames = useMemo(
    () => (definition?.networks ? Object.keys(definition.networks) : []),
    [definition?.networks],
  );

  // Counts for badges
  const svcCount = definition?.services ? Object.keys(definition.services).length : 0;
  const volCount = definition?.volumes ? Object.keys(definition.volumes).length : 0;
  const netCount = definition?.networks ? Object.keys(definition.networks).length : 0;
  const cfgCount = definition?.configs ? Object.keys(definition.configs).length : 0;
  const secCount = definition?.secrets ? Object.keys(definition.secrets).length : 0;
  const envCount = envFiles?.length ?? 0;

  // Error state
  if (parseError) {
    return (
      <Alert
        type="warning"
        message="Cannot switch to Visual mode"
        description={parseError}
        showIcon
        style={{ marginBottom: 12 }}
      />
    );
  }

  // Empty state
  if (!definition) {
    return (
      <Alert
        type="info"
        message="Empty compose file"
        description="Start by adding services below, or switch to Raw mode to paste YAML directly."
        showIcon
        style={{ marginBottom: 12 }}
      />
    );
  }

  return (
    <Tabs
      size="small"
      type="card"
      defaultActiveKey="services"
      style={{ marginTop: -4 }}
      items={[
        {
          key: 'services',
          label: (
            <span>
              🐳 Services <Badge count={svcCount} size="small" style={{ backgroundColor: '#1677ff' }} />
            </span>
          ),
          children: (
            <ServiceList
              value={definition.services}
              onChange={handleServicesChange}
              volumeNames={volumeNames}
              networkNames={networkNames}
            />
          ),
        },
        {
          key: 'volumes',
          label: (
            <span>
              💾 Volumes <Badge count={volCount} size="small" style={{ backgroundColor: '#52c41a' }} />
            </span>
          ),
          children: (
            <VolumeList
              value={definition.volumes ?? {}}
              onChange={handleVolumesChange}
            />
          ),
        },
        {
          key: 'networks',
          label: (
            <span>
              🌐 Networks <Badge count={netCount} size="small" style={{ backgroundColor: '#722ed1' }} />
            </span>
          ),
          children: (
            <NetworkList
              value={definition.networks ?? {}}
              onChange={handleNetworksChange}
            />
          ),
        },
        {
          key: 'configs',
          label: (
            <span>
              ⚙️ Configs <Badge count={cfgCount} size="small" style={{ backgroundColor: '#fa8c16' }} />
            </span>
          ),
          children: (
            <ConfigSecretList
              title="Configs"
              value={definition.configs ?? {}}
              onChange={handleConfigsChange}
            />
          ),
        },
        {
          key: 'secrets',
          label: (
            <span>
              🔒 Secrets <Badge count={secCount} size="small" style={{ backgroundColor: '#eb2f96' }} />
            </span>
          ),
          children: (
            <ConfigSecretList
              title="Secrets"
              value={definition.secrets ?? {}}
              onChange={handleSecretsChange}
            />
          ),
        },
      ].concat(
        envFiles !== undefined
          ? {
              key: 'env',
              label: (
                <span>
                  🔤 Env Files <Badge count={envCount} size="small" style={{ backgroundColor: '#13c2c2' }} />
                </span>
              ),
              children: (
                <EnvFileList
                  value={envFiles}
                  onUpsert={onEnvFileUpsert ?? (async () => {})}
                  onDelete={onEnvFileDelete ?? (async () => {})}
                />
              ),
            }
          : [],
      )}
    />
  );
}

export default ComposeVisualEditor;