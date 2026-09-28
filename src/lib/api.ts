import { invoke } from '@tauri-apps/api/core';
import type { AiConfig, AiResult, AiSuggestion, Entry, EntryInput, GmailAccount, GmailMessage, ImportPreview, MailPreview, RecordType, Relation, RelationInput } from './types';

export const inDesktop = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

function invokeApp<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (inDesktop) return invoke<T>(command, args);
  if (import.meta.env.DEV) return previewCall<T>(command, args);
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
  putRelation: (input: RelationInput) =>
    invokeApp<Relation>('put_relation', { input }),
  deleteRelation: (id: string) =>
    invokeApp<void>('delete_relation', { id }),
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
  getAiConfig: () =>
    invokeApp<AiConfig>('get_ai_config'),
  putAiConfig: (config: AiConfig) =>
    invokeApp<void>('put_ai_config', { config }),
  setOpenRouterKey: (key: string) =>
    invokeApp<void>('set_openrouter_key', { key }),
  hasOpenRouterKey: () =>
    invokeApp<boolean>('has_openrouter_key'),
  askAssistant: (input: { question: string; context_ids: string[]; allow_cloud: boolean }) =>
    invokeApp<AiResult>('ask_assistant', { input }),
  approveAiSuggestion: (suggestion: AiSuggestion) =>
    invokeApp<string>('approve_ai_suggestion', { suggestion }),
  getGmailAccount: () => invokeApp<GmailAccount | null>('get_gmail_account'),
  connectGmail: (clientId: string) => invokeApp<GmailAccount>('connect_gmail', { clientId }),
  searchGmail: (query: string) => invokeApp<GmailMessage[]>('search_gmail', { query }),
  previewGmailImport: (ids: string[]) => invokeApp<MailPreview[]>('preview_gmail_import', { ids }),
  importGmailSelected: (ids: string[]) => invokeApp<number>('import_gmail_selected', { ids })
};

// Browser development preview. The production bundle never uses this path.
const previewTypes: RecordType[] = [
  ['university', 'Universities', 'building'],
  ['lab', 'Labs', 'flask'],
  ['person', 'People', 'user'],
  ['program', 'Programs', 'graduation'],
  ['application', 'Applications', 'folder'],
  ['publication', 'Publications', 'book'],
  ['project', 'Projects', 'layers'],
  ['mail', 'Mail', 'mail'],
  ['document', 'Documents', 'file'],
  ['note', 'Notes', 'note']
].map(([key, label, icon]) => ({
  key,
  label,
  icon,
  fields: key === 'program'
    ? [{ key: 'level', label: 'Level', kind: 'select', options: ['masters', 'phd'], required: false }]
    : key === 'application'
      ? [
          { key: 'status', label: 'Status', kind: 'select', options: ['planning', 'drafting', 'submitted', 'decision_pending', 'accepted', 'rejected', 'withdrawn'], required: false },
          { key: 'intake', label: 'Intake', kind: 'text', options: [], required: false },
          { key: 'deadline', label: 'Deadline', kind: 'date', options: [], required: false }
        ]
      : [],
  built_in: true
})) as RecordType[];

const previewNow = Math.floor(Date.now() / 1000);
let previewEntries: Entry[] = [
  {
    id: 'demo-1',
    type_key: 'program',
    title: 'MSc Computer Science',
    body: 'Research track with a focus on language technologies.',
    values: { level: 'masters' },
    tags: ['ai', 'masters'],
    revision: 1,
    created_at: previewNow,
    updated_at: previewNow,
    deleted_at: null
  },
  {
    id: 'demo-2',
    type_key: 'lab',
    title: 'Language & Learning Lab',
    body: 'Research in multilingual NLP and human-centered AI.',
    values: {},
    tags: ['nlp'],
    revision: 1,
    created_at: previewNow,
    updated_at: previewNow - 3600,
    deleted_at: null
  },
  {
    id: 'demo-3',
    type_key: 'application',
    title: 'Fall 2027 application',
    body: 'Prepare statement of purpose and contact potential supervisors.',
    values: { status: 'drafting', intake: 'Fall 2027', deadline: '2027-01-15' },
    tags: ['2027'],
    revision: 1,
    created_at: previewNow,
    updated_at: previewNow - 7200,
    deleted_at: null
  }
];
let previewRelations: Relation[] = [];
let previewAiConfig: AiConfig = { provider: 'ollama', model: 'local-preview', trust_mode: 'controlled', delegated_types: [] };

async function previewCall<T>(command: string, args: Record<string, unknown>): Promise<T> {
  switch (command) {
    case 'list_types':
      return previewTypes as T;
    case 'put_type': {
      const definition = args.definition as RecordType;
      const index = previewTypes.findIndex((type) => type.key === definition.key);
      if (index >= 0) previewTypes[index] = definition;
      else previewTypes.push(definition);
      return definition as T;
    }
    case 'list_records':
      return previewEntries
        .filter((item) => (!args.typeKey || item.type_key === args.typeKey) && (args.includeDeleted || item.deleted_at === null))
        .sort((a, b) => b.updated_at - a.updated_at) as T;
    case 'get_record':
      return (previewEntries.find((item) => item.id === args.id) ?? null) as T;
    case 'put_record': {
      const input = args.input as EntryInput;
      const existing = previewEntries.find((item) => item.id === input.id);
      if (existing && input.expected_revision !== existing.revision) throw new Error('Record changed since it was opened.');
      const now = Math.floor(Date.now() / 1000);
      const result: Entry = {
        id: existing?.id ?? crypto.randomUUID(),
        type_key: input.type_key,
        title: input.title,
        body: input.body,
        values: input.values,
        tags: input.tags,
        revision: (existing?.revision ?? 0) + 1,
        created_at: existing?.created_at ?? now,
        updated_at: now,
        deleted_at: null
      };
      previewEntries = existing
        ? previewEntries.map((item) => item.id === result.id ? result : item)
        : [result, ...previewEntries];
      return result as T;
    }
    case 'set_deleted': {
      const item = previewEntries.find((record) => record.id === args.id);
      if (!item) throw new Error('Record not found.');
      item.deleted_at = args.deleted ? previewNow : null;
      return item as T;
    }
    case 'search_records': {
      const query = String(args.query).toLowerCase();
      return previewEntries.filter((item) =>
        item.deleted_at === null && (item.title + ' ' + item.body).toLowerCase().includes(query)
      ) as T;
    }
    case 'list_relations':
      return previewRelations.filter((item) => item.source_id === args.id || item.target_id === args.id) as T;
    case 'put_relation': {
      const relation = { ...(args.input as RelationInput), id: crypto.randomUUID() };
      previewRelations.push(relation);
      return relation as T;
    }
    case 'delete_relation':
      previewRelations = previewRelations.filter((item) => item.id !== args.id);
      return undefined as T;
    case 'get_setting':
      return JSON.parse(localStorage.getItem('eduport:' + String(args.key)) || 'null') as T;
    case 'put_setting':
      localStorage.setItem('eduport:' + String(args.key), JSON.stringify(args.value));
      return undefined as T;
    case 'get_ai_config':
      return previewAiConfig as T;
    case 'put_ai_config':
      previewAiConfig = args.config as AiConfig;
      return undefined as T;
    case 'has_openrouter_key':
      return false as T;
    case 'set_openrouter_key':
      return undefined as T;
    case 'ask_assistant': {
      const input = args.input as { question: string; context_ids: string[] };
      const names = previewEntries.filter((entry) => input.context_ids.includes(entry.id)).map((entry) => entry.title);
      return {
        answer: names.length
          ? 'Preview answer based on ' + names.join(', ') + '. In the desktop app, the selected provider analyzes the records you share.'
          : 'This is a browser preview. Open Eduport on the desktop to ask your selected model.',
        suggestions: [],
        provider: previewAiConfig.provider,
        model: previewAiConfig.model,
        shared_record_ids: input.context_ids
      } as T;
    }
    case 'approve_ai_suggestion':
      throw new Error('AI approval is only available in the desktop app.');
    case 'get_gmail_account':
      return null as T;
    case 'connect_gmail':
    case 'search_gmail':
    case 'preview_gmail_import':
    case 'import_gmail_selected':
      throw new Error('Gmail connection is available in the desktop app.');
    case 'preview_markdown_import':
    case 'export_markdown':
    case 'backup_database':
      throw new Error('File operations are available in the desktop app.');
    default:
      throw new Error('Preview does not support ' + command);
  }
}
