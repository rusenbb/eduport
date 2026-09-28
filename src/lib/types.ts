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
}

export type RelationInput = Omit<Relation, 'id'>;

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

export interface AiSuggestion {
  operation: 'create' | 'update';
  id: string | null;
  expected_revision: number | null;
  type_key: string;
  title: string;
  body: string;
  values: Record<string, unknown>;
  tags: string[];
}

export interface AiResult {
  answer: string;
  suggestions: Array<{
    suggestion: AiSuggestion;
    status: 'pending' | 'applied' | 'rejected';
    note: string | null;
  }>;
  provider: string;
  model: string;
  shared_record_ids: string[];
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
