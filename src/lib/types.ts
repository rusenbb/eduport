export type FieldKind = 'text' | 'number' | 'date' | 'boolean' | 'url' | 'select';

export interface FieldDef {
  key: string;
  label: string;
  kind: FieldKind;
  options: string[];
  required: boolean;
}

export interface RecordType {
  key: string;
  label: string;
  icon: string;
  fields: FieldDef[];
  built_in: boolean;
}

export interface Entry {
  id: string;
  type_key: string;
  title: string;
  body: string;
  values: Record<string, unknown>;
  tags: string[];
  revision: number;
  created_at: number;
  updated_at: number;
  deleted_at: number | null;
}

export interface EntryInput {
  id?: string | null;
  type_key: string;
  title: string;
  body: string;
  values: Record<string, unknown>;
  tags: string[];
  expected_revision?: number | null;
}

export interface Relation {
  id: string;
  source_id: string;
  target_id: string;
  kind: string;
  role: string | null;
  position: number | null;
  start_date: string | null;
  end_date: string | null;
  definition_id: string | null;
}

export type RelationInput = Omit<Relation, 'id'>;

export interface RelationDefinition {
  id: string;
  source_type: string;
  target_type: string;
  name: string;
  inverse_name: string;
  built_in: boolean;
}

export type RelationDefinitionInput = Omit<RelationDefinition, 'id' | 'built_in'> & { id?: string | null };

export interface FilterRule { field: string; operator: 'contains' | 'equals' | 'not_equals' | 'is_empty' | 'greater_than' | 'less_than'; value: string; }
export interface FilterGroup { mode: 'and' | 'or'; rules: FilterRule[]; groups: FilterGroup[]; }
export interface SortRule { field: string; direction: 'asc' | 'desc'; }
export interface ViewConfig { columns: string[]; filters: FilterRule[]; filter_tree?: FilterGroup | null; sorts: SortRule[]; group_by: string | null; column_widths?: Record<string, number>; }
export interface TableView { id: string; type_key: string; name: string; config: ViewConfig; is_default: boolean; }
export type TableViewInput = Omit<TableView, 'id' | 'is_default'> & { id?: string | null };

export interface ImportPreview {
  fingerprint: string;
  format: string;
  rows: Array<{ id: string; type_key: string; title: string; deleted: boolean }>;
  file_count: number;
  warnings: string[];
  blockers: string[];
}

export interface AiConfig {
  provider: 'ollama' | 'openrouter';
  model: string;
  trust_mode: 'controlled' | 'delegated';
  delegated_types: string[];
}

export interface AgentAction {
  id: string;
  tool: string;
  arguments: Record<string, unknown>;
  target_types: string[];
  status: 'pending' | 'applying' | 'applied' | 'rejected';
  created_at: number;
  error: string | null;
  chat_message_id: string | null;
  decision: 'auto' | 'approved' | 'rejected' | null;
  decided_at: number | null;
}

export interface ChatSession {
  id: string;
  title: string;
  created_at: number;
  updated_at: number;
  archived_at: number | null;
}

export interface ChatMessage {
  id: string;
  session_id: string;
  sequence: number;
  reply_to: string | null;
  role: 'user' | 'assistant';
  content: string;
  status: 'running' | 'completed' | 'failed';
  provider: string | null;
  model: string | null;
  error: string | null;
  created_at: number;
  updated_at: number;
}

export interface ChatEvent {
  id: string;
  assistant_message_id: string;
  ordinal: number;
  tool: string;
  arguments: Record<string, unknown>;
  result: unknown;
  action_id: string | null;
  created_at: number;
}

export interface ActionOutcome { action: AgentAction; result: unknown | null; }

export interface AiResult {
  session_id: string;
  assistant_message_id: string;
  answer: string;
  actions: ActionOutcome[];
  provider: string;
  model: string;
  read_calls: number;
  write_calls: number;
}

export interface GmailAccount {
  provider: 'gmail';
  email: string;
  client_id: string;
}

export interface GmailMessage {
  id: string;
  subject: string;
  from: string;
  date: string;
  snippet: string;
  imported: boolean;
}

export interface MailPreview {
  id: string;
  subject: string;
  from: string;
  to: string;
  date: string;
  body: string;
  raw_bytes: number;
  imported: boolean;
}
