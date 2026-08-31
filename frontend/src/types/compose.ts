// ── Docker Compose type definitions ──

export interface BuildDef {
  context?: string;
  dockerfile?: string;
  args?: Record<string, string>;
  target?: string;
  cache_from?: string[];
  cache_to?: string[];
  labels?: Record<string, string>;
  network?: string;
  ssh?: string[];
  secrets?: string[];
  tags?: string[];
  platforms?: string[];
  pull?: boolean;
}

export interface HealthcheckDef {
  test?: string | string[];
  interval?: string;
  timeout?: string;
  retries?: number;
  start_period?: string;
  start_interval?: string;
  disable?: boolean;
}

export interface DeployResourcesDef {
  limits?: { cpus?: string; memory?: string; pids?: number };
  reservations?: { cpus?: string; memory?: string; generic_resources?: Record<string, string> };
}

export interface DeployRestartPolicyDef {
  condition?: string;
  delay?: string;
  max_attempts?: number;
  window?: string;
}

export interface DeployPlacementDef {
  constraints?: string[];
  preferences?: { spread?: string }[];
  max_replicas_per_node?: number;
}

export interface DeployDef {
  mode?: string;
  replicas?: number;
  resources?: DeployResourcesDef;
  restart_policy?: DeployRestartPolicyDef;
  placement?: DeployPlacementDef;
  labels?: Record<string, string>;
  update_config?: {
    parallelism?: number;
    delay?: string;
    failure_action?: string;
    monitor?: string;
    max_failure_ratio?: number;
    order?: string;
  };
  rollback_config?: {
    parallelism?: number;
    delay?: string;
    failure_action?: string;
    monitor?: string;
    max_failure_ratio?: number;
    order?: string;
  };
  endpoint_mode?: string;
}

export interface LoggingDef {
  driver?: string;
  options?: Record<string, string>;
}

export interface UlimitsDef {
  nofile?: { soft?: number; hard?: number } | number;
  nproc?: { soft?: number; hard?: number } | number;
  [key: string]: { soft?: number; hard?: number } | number | undefined;
}

export interface DependsOnDef {
  condition?: string;
  restart?: boolean;
  required?: boolean;
}

export interface ServiceDef {
  // Basic
  image?: string;
  build?: string | BuildDef;
  container_name?: string;
  restart?: string;
  command?: string | string[];
  entrypoint?: string | string[];

  // Networking
  ports?: string[];
  expose?: string[];
  networks?: string[];
  dns?: string[];
  dns_search?: string[];
  network_mode?: string;
  extra_hosts?: string[];
  domainname?: string;
  mac_address?: string;

  // Storage
  volumes?: string[];
  tmpfs?: string[];
  configs?: string[];
  secrets?: string[];

  // Environment
  environment?: Record<string, string>;
  env_file?: string | string[];

  // Metadata
  labels?: Record<string, string>;
  profiles?: string[];

  // Resource & runtime
  deploy?: DeployDef;
  healthcheck?: HealthcheckDef;
  logging?: LoggingDef;
  cap_add?: string[];
  cap_drop?: string[];
  privileged?: boolean;
  ulimits?: UlimitsDef;
  user?: string;
  working_dir?: string;
  hostname?: string;
  stdin_open?: boolean;
  tty?: boolean;
  read_only?: boolean;
  init?: boolean;

  // Dependencies
  depends_on?: Record<string, DependsOnDef> | string[];

  // Other
  sysctls?: Record<string, string>;
  security_opt?: string[];
  stop_grace_period?: string;
  stop_signal?: string;
  shm_size?: string;
  pid?: string;
  cgroup_parent?: string;
  group_add?: string[];
  devices?: string[];
  runtime?: string;
  mem_reservation?: string;
  mem_limit?: string;
  cpus?: string;
  cpu_shares?: number;
  cpu_quota?: number;
  cpu_period?: number;
  cpuset?: string;
  oom_kill_disable?: boolean;
  oom_score_adj?: number;
  scale?: number;
  isolation?: string;
  blkio_config?: {
    weight?: number;
    weight_device?: { path?: string; weight?: number }[];
    device_read_bps?: { path?: string; rate?: string }[];
    device_write_bps?: { path?: string; rate?: string }[];
    device_read_iops?: { path?: string; rate?: number }[];
    device_write_iops?: { path?: string; rate?: number }[];
  };
}

export interface VolumeDef {
  driver?: string;
  driver_opts?: Record<string, string>;
  external?: boolean | { name: string };
  labels?: Record<string, string>;
  name?: string;
}

export interface IpamConfigDef {
  subnet?: string;
  gateway?: string;
  ip_range?: string;
  aux_addresses?: Record<string, string>;
}

export interface NetworkDef {
  driver?: string;
  driver_opts?: Record<string, string>;
  ipam?: {
    driver?: string;
    config?: IpamConfigDef[];
    options?: Record<string, string>;
  };
  external?: boolean | { name: string };
  labels?: Record<string, string>;
  name?: string;
  enable_ipv6?: boolean;
  internal?: boolean;
  attachable?: boolean;
}

export interface ConfigDef {
  file?: string;
  external?: boolean | { name: string };
  name?: string;
  template_driver?: string;
}

export interface SecretDef {
  file?: string;
  external?: boolean | { name: string };
  name?: string;
  template_driver?: string;
}

export interface ComposeDefinition {
  version?: string;
  services: Record<string, ServiceDef>;
  volumes?: Record<string, VolumeDef>;
  networks?: Record<string, NetworkDef>;
  configs?: Record<string, ConfigDef>;
  secrets?: Record<string, SecretDef>;
  name?: string;
  x_headers?: Record<string, unknown>;
}

// ── Helper: default empty compose ──

export function emptyCompose(): ComposeDefinition {
  return {
    version: '3.9',
    services: {},
    volumes: {},
    networks: {},
    configs: {},
    secrets: {},
  };
}