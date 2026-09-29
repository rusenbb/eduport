import { invoke } from '@tauri-apps/api/core';
import type { AiConfig, AiResult, AgentAction, ActionOutcome, ChatEvent, ChatMessage, ChatSession, Entry, EntryInput, GmailAccount, GmailMessage, ImportPreview, JsonImportPreview, MailPreview, RecordType, Relation, RelationInput, RelationDefinition, RelationDefinitionInput, TableView, TableViewInput } from './types';

export const inDesktop = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

function invokeApp<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (inDesktop) return invoke<T>(command, args);
  if (import.meta.env.DEV) return browserCall<T>(command, args);
  return Promise.reject(new Error('Open Eduport as a desktop app.'));
}

export const api = {
  listTypes: () => invokeApp<RecordType[]>('list_types'),
  putType: (definition: RecordType) =>
    invokeApp<RecordType>('put_type', { definition }),
  listRecords: (typeKey?: string, includeDeleted = false) =>
    invokeApp<Entry[]>('list_records', {
      typeKey: typeKey || null,
      includeDeleted
    }),
  getRecord: (id: string) => invokeApp<Entry | null>('get_record', { id }),
  putRecord: (input: EntryInput) => invokeApp<Entry>('put_record', { input }),
  setDeleted: (id: string, deleted: boolean) =>
    invokeApp<Entry>('set_deleted', { id, deleted }),
  search: (query: string, limit = 30) =>
    invokeApp<Entry[]>('search_records', { query, limit }),
  listRelations: (id: string) =>
    invokeApp<Relation[]>('list_relations', { id }),
  listAllRelations: () => invokeApp<Relation[]>('list_all_relations'),
  putRelation: (input: RelationInput) =>
    invokeApp<Relation>('put_relation', { input }),
  deleteRelation: (id: string) =>
    invokeApp<void>('delete_relation', { id }),
  listRelationDefinitions: (typeKey?: string) =>
    invokeApp<RelationDefinition[]>('list_relation_definitions', { typeKey: typeKey || null }),
  putRelationDefinition: (input: RelationDefinitionInput) =>
    invokeApp<RelationDefinition>('put_relation_definition', { input }),
  listViews: (typeKey: string) => invokeApp<TableView[]>('list_views', { typeKey }),
  putView: (input: TableViewInput) => invokeApp<TableView>('put_view', { input }),
  deleteView: (id: string) => invokeApp<void>('delete_view', { id }),
  getSetting: <T>(key: string) =>
    invokeApp<T | null>('get_setting', { key }),
  putSetting: (key: string, value: unknown) =>
    invokeApp<void>('put_setting', { key, value }),
  previewMarkdownImport: () =>
    invokeApp<ImportPreview | null>('preview_markdown_import'),
  applyMarkdownImport: (fingerprint: string) =>
    invokeApp<number>('apply_markdown_import', { fingerprint }),
  exportMarkdown: () =>
    invokeApp<string | null>('export_markdown'),
  backupDatabase: () =>
    invokeApp<string | null>('backup_database'),
  exportJsonBackup: (includeHistory: boolean) =>
    invokeApp<string | null>('export_json_backup', { includeHistory }),
  previewJsonImport: () =>
    invokeApp<JsonImportPreview | null>('preview_json_import'),
  applyJsonImport: (fingerprint: string) =>
    invokeApp<number>('apply_json_import', { fingerprint }),
  getAiConfig: () =>
    invokeApp<AiConfig>('get_ai_config'),
  putAiConfig: (config: AiConfig) =>
    invokeApp<void>('put_ai_config', { config }),
  setOpenRouterKey: (key: string) =>
    invokeApp<void>('set_openrouter_key', { key }),
  hasOpenRouterKey: () =>
    invokeApp<boolean>('has_openrouter_key'),
  createChatSession: () => invokeApp<ChatSession>('create_chat_session'),
  listChatSessions: (archived = false) => invokeApp<ChatSession[]>('list_chat_sessions', { archived }),
  renameChatSession: (id: string, title: string) => invokeApp<ChatSession>('rename_chat_session', { id, title }),
  setChatArchived: (id: string, archived: boolean) => invokeApp<ChatSession>('set_chat_archived', { id, archived }),
  listChatMessages: (sessionId: string) => invokeApp<ChatMessage[]>('list_chat_messages', { sessionId }),
  listChatEvents: (sessionId: string) => invokeApp<ChatEvent[]>('list_chat_events', { sessionId }),
  listChatActions: (sessionId: string) => invokeApp<AgentAction[]>('list_chat_actions', { sessionId }),
  askAssistant: (input: { session_id: string | null; question: string; allow_cloud: boolean }) =>
    invokeApp<AiResult>('ask_assistant', { input }),
  listAgentActions: () => invokeApp<AgentAction[]>('list_agent_actions'),
  approveAgentAction: (id: string) => invokeApp<ActionOutcome>('approve_agent_action', { id }),
  rejectAgentAction: (id: string) => invokeApp<AgentAction>('reject_agent_action', { id }),
  getGmailAccount: () => invokeApp<GmailAccount | null>('get_gmail_account'),
  connectGmail: (clientId: string) => invokeApp<GmailAccount>('connect_gmail', { clientId }),
  searchGmail: (query: string) => invokeApp<GmailMessage[]>('search_gmail', { query }),
  previewGmailImport: (ids: string[]) => invokeApp<MailPreview[]>('preview_gmail_import', { ids }),
  importGmailSelected: (ids: string[]) => invokeApp<number>('import_gmail_selected', { ids })
};

// Browser development uses the same SQLite database through a loopback-only Rust API.
async function browserCall<T>(command: string, args: Record<string, unknown>): Promise<T> {
  let response: Response;
  try {
    response = await fetch('/api/command', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ command, args })
    });
  } catch {
    throw new Error('The local SQLite API is unavailable. Start Eduport with npm run dev.');
  }
  const payload = await response.json() as { result?: T; error?: string };
  if (!response.ok) throw new Error(payload.error ?? `Local API returned ${response.status}.`);
  return payload.result as T;
}
