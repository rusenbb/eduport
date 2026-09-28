<script lang="ts">
  import {
    ArrowLeft, ArrowRight, BookOpen, BookText, Building2, Check, ChevronRight, CircleHelp,
    Clock3, Download, FileText, FlaskConical, FolderKanban, GraduationCap, GripVertical,
    Home, Layers3, Link2, Mail, Plus, Search, Settings2, SlidersHorizontal, Sparkles, StickyNote,
    Trash2, UserRound, X
  } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { api, inDesktop } from '$lib/api';
  import { applyTableView, cellValue, displayValue } from '$lib/table';
  import type { RelationIndex } from '$lib/table';
  import type { AiConfig, AiResult, AgentAction, Entry, EntryInput, FieldDef, GmailAccount, GmailMessage, ImportPreview, MailPreview, RecordType, Relation, RelationInput, RelationDefinition, RelationDefinitionInput, TableView, TableViewInput, ViewConfig } from '$lib/types';

  let types = $state<RecordType[]>([]);
  let entries = $state<Entry[]>([]);
  let relations = $state<Relation[]>([]);
  let allRelations = $state<Relation[]>([]);
  let definitions = $state<RelationDefinition[]>([]);
  let views = $state<TableView[]>([]);
  let currentViewId = $state('');
  let viewDraft = $state<TableViewInput | null>(null);
  let newTypeOpen = $state(false);
  let newTypeLabel = $state('');
  let newTypeKey = $state('');
  let newTypeIcon = $state('file');
  let definitionOpen = $state(false);
  let definitionDraft = $state<RelationDefinitionInput>({ id: null, source_type: 'lab', target_type: 'university', name: '', inverse_name: '' });
  let active = $state('home');
  let selectedId = $state<string | null>(null);
  let searchQuery = $state('');
  let listQuery = $state('');
  let loading = $state(true);
  let error = $state('');
  let toast = $state('');
  let editorOpen = $state(false);
  let tagText = $state('');
  let editingType = $state<RecordType | null>(null);
  let importPreview = $state<ImportPreview | null>(null);
  let relationOpen = $state(false);
  let aiConfig = $state<AiConfig>({ provider: 'ollama', model: '', trust_mode: 'controlled', delegated_types: [] });
  let openrouterKey = $state('');
  let keyConfigured = $state(false);
  let assistantPrompt = $state('');
  let pendingActions = $state<AgentAction[]>([]);
  let allowCloud = $state(false);
  let assistantBusy = $state(false);
  let assistantResult = $state<AiResult | null>(null);
  let gmailAccount = $state<GmailAccount | null>(null);
  let gmailClientId = $state('');
  let gmailQuery = $state('');
  let gmailMessages = $state<GmailMessage[]>([]);
  let gmailSelected = $state<string[]>([]);
  let gmailPreview = $state<MailPreview[]>([]);
  let gmailBusy = $state(false);
  const defaultTypeOrder = ['university', 'lab', 'person', 'program', 'application', 'publication', 'project', 'mail', 'document', 'note'];
  let sidebarOrder = $state<string[]>(defaultTypeOrder);
  let draggedType = $state<string | null>(null);
  let dropTarget = $state<string | null>(null);
  let dragFinishedAt = 0;
  let newField = $state({ key: '', label: '', kind: 'text' as FieldDef['kind'], required: false });
  let newFieldOptions = $state('');
  let relationDraft = $state<RelationInput>({
    source_id: '', target_id: '', kind: 'related_to', role: null,
    position: null, start_date: null, end_date: null, definition_id: null
  });
  let draft = $state<EntryInput>({
    type_key: 'note', title: '', body: '', values: {}, tags: []
  });

  const selected = $derived(entries.find((entry) => entry.id === selectedId) ?? null);
  const activeType = $derived(types.find((type) => type.key === active) ?? null);
  const editType = $derived(types.find((type) => type.key === draft.type_key) ?? null);
  const liveEntries = $derived(entries.filter((entry) => entry.deleted_at === null));
  const currentView = $derived(views.find((view) => view.id === currentViewId) ?? views[0] ?? null);
  const relationIndex = $derived.by(() => {
    const index: RelationIndex = {};
    const titles = new Map(entries.map((entry) => [entry.id, entry.title]));
    for (const relation of allRelations) {
      if (!relation.definition_id) continue;
      const field = `relation:${relation.definition_id}`;
      for (const [id, otherId] of [[relation.source_id, relation.target_id], [relation.target_id, relation.source_id]]) {
        const title = titles.get(otherId);
        if (!title) continue;
        index[id] ??= {};
        index[id][field] ??= [];
        index[id][field].push(title);
      }
    }
    return index;
  });
  const tableGroups = $derived(activeType && currentView ? applyTableView(liveEntries.filter((entry) => entry.type_key === activeType.key), currentView.config, listQuery, relationIndex) : []);
  const tableCount = $derived(tableGroups.reduce((total, group) => total + group.rows.length, 0));
  const applicableDefinitions = $derived(selected ? definitions.filter((item) => item.source_type === selected.type_key || item.target_type === selected.type_key) : []);
  const chosenDefinition = $derived(definitions.find((item) => item.id === relationDraft.definition_id) ?? null);
  const relationTargetType = $derived(selected && chosenDefinition ? (chosenDefinition.source_type === selected.type_key ? chosenDefinition.target_type : chosenDefinition.source_type) : null);
  const viewKind = $derived(types.find((kind) => kind.key === viewDraft?.type_key) ?? null);
  const viewFields = $derived(viewKind ? ['title', 'body', ...viewKind.fields.map((field) => field.key),
    ...definitions.filter((definition) => definition.source_type === viewKind.key || definition.target_type === viewKind.key)
      .map((definition) => `relation:${definition.id}`), 'tags', 'updated_at'] : []);
  const orderedTypes = $derived([...types].sort((a, b) => {
    const aIndex = sidebarOrder.indexOf(a.key);
    const bIndex = sidebarOrder.indexOf(b.key);
    return (aIndex < 0 ? 999 : aIndex) - (bIndex < 0 ? 999 : bIndex) || a.label.localeCompare(b.label);
  }));
  const visibleEntries = $derived.by(() => {
    let pool = active === 'trash'
      ? entries.filter((entry) => entry.deleted_at !== null)
      : liveEntries;
    if (active !== 'all' && active !== 'trash' && active !== 'search') {
      pool = pool.filter((entry) => entry.type_key === active);
    }
    const term = (active === 'search' ? searchQuery : listQuery).trim().toLowerCase();
    if (term) {
      pool = pool.filter((entry) =>
        (entry.title + ' ' + entry.body + ' ' + JSON.stringify(entry.values))
          .toLowerCase().includes(term)
      );
    }
    return [...pool].sort((a, b) => b.updated_at - a.updated_at);
  });
  const countByType = $derived.by(() => {
    const counts: Record<string, number> = {};
    for (const entry of liveEntries) counts[entry.type_key] = (counts[entry.type_key] ?? 0) + 1;
    return counts;
  });
  const upcoming = $derived(
    liveEntries
      .filter((entry) => entry.type_key === 'application' && typeof entry.values.deadline === 'string' && String(entry.values.deadline) >= new Date().toISOString().slice(0, 10))
      .sort((a, b) => String(a.values.deadline).localeCompare(String(b.values.deadline)))
      .slice(0, 4)
  );
  const importConflicts = $derived(
    importPreview?.rows.filter((row) => entries.some((entry) => entry.id === row.id)) ?? []
  );

  onMount(() => { void load(); });

  async function load() {
    loading = true;
    error = '';
    try {
      [types, entries, definitions, allRelations, pendingActions] = await Promise.all([api.listTypes(), api.listRecords(undefined, true), api.listRelationDefinitions(), api.listAllRelations(), api.listAgentActions()]);
      const savedOrder = await api.getSetting<string[]>('sidebar_order');
      if (Array.isArray(savedOrder)) sidebarOrder = savedOrder.filter((key): key is string => typeof key === 'string');
      aiConfig = await api.getAiConfig();
      keyConfigured = await api.hasOpenRouterKey();
      gmailAccount = await api.getGmailAccount();
    } catch (cause) {
      error = message(cause);
    } finally {
      loading = false;
    }
  }

  function message(cause: unknown): string {
    return cause instanceof Error ? cause.message : String(cause);
  }

  function notify(text: string) {
    toast = text;
    window.setTimeout(() => { if (toast === text) toast = ''; }, 3600);
  }

  function navigate(next: string) {
    active = next;
    selectedId = null;
    listQuery = '';
    if (types.some((type) => type.key === next)) void loadViews(next);
    window.scrollTo({ top: 0, behavior: 'instant' });
  }

  async function loadViews(typeKey: string) {
    views = [];
    currentViewId = '';
    try {
      const loaded = await api.listViews(typeKey);
      if (active === typeKey) {
        views = loaded;
        currentViewId = loaded[0]?.id ?? '';
      }
    } catch (cause) {
      error = message(cause);
    }
  }

  async function choose(entry: Entry) {
    selectedId = entry.id;
    try {
      relations = await api.listRelations(entry.id);
    } catch (cause) {
      error = message(cause);
    }
  }

  function startCreate(typeKey?: string) {
    const chosenType = typeKey ?? (activeType?.key || 'note');
    draft = {
      type_key: chosenType,
      title: '',
      body: '',
      values: initialValues(chosenType),
      tags: []
    };
    error = '';
    tagText = '';
    editorOpen = true;
  }

  function initialValues(typeKey: string): Record<string, unknown> {
    return typeKey === 'application' ? { status: 'planning' } : {};
  }

  function changeDraftType(typeKey: string) {
    draft.type_key = typeKey;
    draft.values = initialValues(typeKey);
  }

  function startEdit(entry: Entry) {
    draft = {
      id: entry.id,
      type_key: entry.type_key,
      title: entry.title,
      body: entry.body,
      values: { ...entry.values },
      tags: [...entry.tags],
      expected_revision: entry.revision
    };
    error = '';
    tagText = entry.tags.join(', ');
    editorOpen = true;
  }

  async function saveEntry() {
    error = '';
    try {
      const values: Record<string, unknown> = {};
      for (const [key, value] of Object.entries(draft.values)) {
        if (value !== '' && value !== null && value !== undefined) values[key] = value;
      }
      const tags = [...new Set(tagText.split(',').map((tag) => tag.trim()).filter(Boolean))];
      const saved = await api.putRecord({ ...draft, title: draft.title.trim(), values, tags });
      entries = draft.id
        ? entries.map((entry) => entry.id === saved.id ? saved : entry)
        : [saved, ...entries];
      editorOpen = false;
      if (active === 'home' || active === 'trash') navigate(saved.type_key);
      await choose(saved);
      notify(draft.id ? 'Record updated' : 'Record created');
    } catch (cause) {
      error = message(cause);
    }
  }

  async function confirmAction(text: string): Promise<boolean> {
    if (!inDesktop) return window.confirm(text);
    const { ask } = await import('@tauri-apps/plugin-dialog');
    return ask(text, { title: 'Eduport', kind: 'warning' });
  }

  async function toggleDeleted(entry: Entry) {
    const deleting = entry.deleted_at === null;
    if (deleting && !await confirmAction('Move this record to Trash?')) return;
    try {
      const changed = await api.setDeleted(entry.id, deleting);
      entries = entries.map((item) => item.id === changed.id ? changed : item);
      selectedId = null;
      notify(deleting ? 'Moved to Trash' : 'Record restored');
    } catch (cause) {
      error = message(cause);
    }
  }

  function startRelation(entry: Entry) {
    const first = definitions.find((item) => item.source_type === entry.type_key || item.target_type === entry.type_key);
    relationDraft = {
      source_id: entry.id,
      target_id: '',
      kind: first?.name ?? '',
      role: null,
      position: null,
      start_date: null,
      end_date: null,
      definition_id: first?.id ?? null
    };
    error = '';
    relationOpen = true;
  }

  async function saveRelation() {
    if (!selected || !chosenDefinition || !relationDraft.target_id) {
      error = 'Choose a relationship and a matching record.';
      return;
    }
    try {
      const sourceIsCurrent = chosenDefinition.source_type === selected.type_key;
      await api.putRelation({ ...relationDraft, kind: chosenDefinition.name,
        source_id: sourceIsCurrent ? selected.id : relationDraft.target_id,
        target_id: sourceIsCurrent ? relationDraft.target_id : selected.id });
      relations = await api.listRelations(selected.id);
      allRelations = await api.listAllRelations();
      relationOpen = false;
      notify('Relation added');
    } catch (cause) {
      error = message(cause);
    }
  }

  async function removeRelation(id: string) {
    if (!await confirmAction('Remove this connection? It will not appear in Trash.')) return;
    try {
      await api.deleteRelation(id);
      relations = relations.filter((relation) => relation.id !== id);
      allRelations = allRelations.filter((relation) => relation.id !== id);
      notify('Relation removed');
    } catch (cause) {
      error = message(cause);
    }
  }

  function startFieldEditor(type: RecordType) {
    editingType = { ...type, fields: type.fields.map((field) => ({ ...field, options: [...field.options] })) };
    newField = { key: '', label: '', kind: 'text', required: false };
    newFieldOptions = '';
    error = '';
  }

  async function addField() {
    if (!editingType || !newField.key.trim() || !newField.label.trim()) {
      error = 'Key and label are required.';
      return;
    }
    const options = newFieldOptions.split(',').map((value) => value.trim()).filter(Boolean);
    if (newField.kind === 'select' && options.length === 0) {
      error = 'Add at least one choice for this field.';
      return;
    }
    const updated: RecordType = {
      ...editingType,
      fields: [...editingType.fields, {
        key: newField.key.trim().toLowerCase().replace(/[^a-z0-9_]/g, '_'),
        label: newField.label.trim(),
        kind: newField.kind,
        options: newField.kind === 'select' ? [...new Set(options)] : [],
        required: newField.required
      }]
    };
    try {
      const saved = await api.putType(updated);
      types = types.map((type) => type.key === saved.key ? saved : type);
      editingType = saved;
      if (active === saved.key) {
        const previouslySelected = currentViewId;
        views = await api.listViews(saved.key);
        currentViewId = views.some((view) => view.id === previouslySelected) ? previouslySelected : views[0]?.id ?? '';
      }
      newField = { key: '', label: '', kind: 'text', required: false };
      newFieldOptions = '';
      notify('Field added');
    } catch (cause) {
      error = message(cause);
    }
  }

  function startNewType() {
    newTypeLabel = '';
    newTypeKey = '';
    newTypeIcon = 'file';
    error = '';
    newTypeOpen = true;
  }

  async function saveNewType() {
    const key = (newTypeKey.trim() || newTypeLabel.trim().toLowerCase().replace(/[^a-z0-9]+/g, '_')).replace(/^_+|_+$/g, '');
    if (!key || !newTypeLabel.trim()) { error = 'Name and key are required.'; return; }
    try {
      const saved = await api.putType({ key, label: newTypeLabel.trim(), icon: newTypeIcon, fields: [], built_in: false });
      types = [...types, saved];
      sidebarOrder = [...sidebarOrder, saved.key];
      await api.putSetting('sidebar_order', sidebarOrder);
      newTypeOpen = false;
      navigate(saved.key);
      notify('Entity type created');
    } catch (cause) { error = message(cause); }
  }

  function startDefinition(sourceType?: string) {
    definitionDraft = { id: null, source_type: sourceType ?? activeType?.key ?? 'lab', target_type: 'university', name: '', inverse_name: '' };
    error = '';
    definitionOpen = true;
  }

  async function saveDefinition() {
    try {
      const saved = await api.putRelationDefinition(definitionDraft);
      definitions = [...definitions, saved];
      if (activeType && (saved.source_type === activeType.key || saved.target_type === activeType.key)) {
        const previous = currentViewId;
        views = await api.listViews(activeType.key);
        currentViewId = views.some((view) => view.id === previous) ? previous : views[0]?.id ?? '';
      }
      definitionOpen = false;
      notify('Relationship added');
    } catch (cause) { error = message(cause); }
  }

  function cloneViewConfig(config: ViewConfig): ViewConfig {
    return { columns: [...config.columns], filters: config.filters.map((rule) => ({ ...rule })),
      sorts: config.sorts.map((rule) => ({ ...rule })), group_by: config.group_by };
  }

  function startViewEditor(view?: TableView) {
    if (!activeType) return;
    const base = view ?? views[0];
    viewDraft = { id: view?.id ?? null, type_key: activeType.key,
      name: view?.name ?? 'New view',
      config: cloneViewConfig(base?.config ?? { columns: ['title'], filters: [], sorts: [], group_by: null }) };
    error = '';
  }

  async function saveView() {
    if (!viewDraft) return;
    try {
      const saved = await api.putView(viewDraft);
      views = [...views.filter((view) => view.id !== saved.id), saved];
      currentViewId = saved.id;
      viewDraft = null;
      notify('View saved');
    } catch (cause) { error = message(cause); }
  }

  async function removeView() {
    if (!viewDraft?.id || !await confirmAction('Delete this saved view? Records will stay unchanged.')) return;
    try {
      await api.deleteView(viewDraft.id);
      views = views.filter((view) => view.id !== viewDraft?.id);
      currentViewId = views[0]?.id ?? '';
      viewDraft = null;
      notify('View deleted');
    } catch (cause) { error = message(cause); }
  }

  function toggleViewColumn(column: string) {
    if (!viewDraft || column === 'title') return;
    viewDraft.config.columns = viewDraft.config.columns.includes(column)
      ? viewDraft.config.columns.filter((field) => field !== column)
      : [...viewDraft.config.columns, column];
  }

  function fieldLabel(type: RecordType, field: string): string {
    if (field.startsWith('relation:')) {
      const definition = definitions.find((item) => item.id === field.slice('relation:'.length));
      return definition ? (definition.source_type === type.key ? definition.name : definition.inverse_name) : 'Relationship';
    }
    return ({ title: 'Title', body: 'Notes', tags: 'Tags', updated_at: 'Updated' } as Record<string, string>)[field]
      ?? type.fields.find((item) => item.key === field)?.label ?? field;
  }

  async function previewImport() {
    error = '';
    try {
      importPreview = await api.previewMarkdownImport();
    } catch (cause) {
      error = message(cause);
    }
  }

  async function applyImport() {
    if (!importPreview) return;
    error = '';
    try {
      const imported = await api.applyMarkdownImport(importPreview.fingerprint);
      importPreview = null;
      await load();
      notify(imported + ' records imported');
    } catch (cause) {
      error = message(cause);
    }
  }

  async function exportMarkdown() {
    error = '';
    try {
      const path = await api.exportMarkdown();
      if (path) notify('Markdown export saved to ' + path);
    } catch (cause) {
      error = message(cause);
    }
  }

  async function backupDatabase() {
    error = '';
    try {
      const path = await api.backupDatabase();
      if (path) notify('Database backup saved to ' + path);
    } catch (cause) {
      error = message(cause);
    }
  }

  async function saveAiConfig() {
    error = '';
    try {
      await api.putAiConfig(aiConfig);
      notify('Assistant settings saved');
    } catch (cause) {
      error = message(cause);
    }
  }

  async function saveOpenRouterKey() {
    error = '';
    try {
      await api.setOpenRouterKey(openrouterKey);
      openrouterKey = '';
      keyConfigured = true;
      notify('OpenRouter key saved to the OS credential store');
    } catch (cause) {
      error = message(cause);
    }
  }

  async function askAssistant() {
    error = '';
    assistantBusy = true;
    try {
      assistantResult = await api.askAssistant({
        question: assistantPrompt,
        allow_cloud: allowCloud
      });
      if (assistantResult.actions.some((item) => item.action.status === 'applied')) {
        [types, entries, definitions, allRelations] = await Promise.all([api.listTypes(), api.listRecords(undefined, true), api.listRelationDefinitions(), api.listAllRelations()]);
      }
      pendingActions = await api.listAgentActions();
    } catch (cause) {
      error = message(cause);
    } finally {
      assistantBusy = false;
    }
  }

  async function approveAction(action: AgentAction) {
    if (action.status !== 'pending') return;
    if (!inDesktop && !await confirmAction(`Apply AI action ${action.tool}? Review its arguments first.`)) return;
    error = '';
    try {
      const outcome = await api.approveAgentAction(action.id);
      if (assistantResult) assistantResult.actions = assistantResult.actions.map((item) => item.action.id === action.id ? outcome : item);
      [types, entries, definitions, allRelations, pendingActions] = await Promise.all([api.listTypes(), api.listRecords(undefined, true), api.listRelationDefinitions(), api.listAllRelations(), api.listAgentActions()]);
      if (activeType) void loadViews(activeType.key);
      notify(outcome.action.status === 'applied' ? 'AI action applied' : outcome.action.error ?? 'AI action failed');
    } catch (cause) {
      error = message(cause);
    }
  }

  async function rejectAction(action: AgentAction) {
    try {
      await api.rejectAgentAction(action.id);
      pendingActions = await api.listAgentActions();
      notify('AI action rejected');
    } catch (cause) { error = message(cause); }
  }

  async function connectGmail() {
    error = '';
    gmailBusy = true;
    try {
      gmailAccount = await api.connectGmail(gmailClientId);
      gmailClientId = '';
      gmailMessages = [];
      gmailPreview = [];
      notify('Gmail connected: ' + gmailAccount.email);
    } catch (cause) {
      error = message(cause);
    } finally {
      gmailBusy = false;
    }
  }

  async function searchGmail() {
    error = '';
    gmailBusy = true;
    gmailPreview = [];
    gmailSelected = [];
    try {
      gmailMessages = await api.searchGmail(gmailQuery);
    } catch (cause) {
      error = message(cause);
    } finally {
      gmailBusy = false;
    }
  }

  function toggleGmail(id: string) {
    gmailPreview = [];
    gmailSelected = gmailSelected.includes(id)
      ? gmailSelected.filter((value) => value !== id)
      : gmailSelected.length < 20 ? [...gmailSelected, id] : gmailSelected;
  }

  async function previewSelectedMail() {
    error = '';
    gmailBusy = true;
    try {
      gmailPreview = await api.previewGmailImport(gmailSelected);
    } catch (cause) {
      error = message(cause);
    } finally {
      gmailBusy = false;
    }
  }

  async function importSelectedMail() {
    error = '';
    gmailBusy = true;
    try {
      const count = await api.importGmailSelected(gmailSelected);
      notify(`${count} Gmail messages imported`);
      const importedIds = new Set(gmailSelected);
      gmailPreview = [];
      gmailSelected = [];
      gmailMessages = gmailMessages.map((item) => ({ ...item, imported: item.imported || importedIds.has(item.id) }));
      entries = await api.listRecords(undefined, true);
    } catch (cause) {
      error = message(cause);
    } finally {
      gmailBusy = false;
    }
  }

  function typeLabel(key: string): string {
    return ({ university: 'University', lab: 'Lab', person: 'Person', program: 'Program',
      application: 'Application', publication: 'Publication', project: 'Project',
      mail: 'Mail', document: 'Document', note: 'Note' } as Record<string, string>)[key]
      ?? types.find((type) => type.key === key)?.label ?? key;
  }

  function compactDate(seconds: number): string {
    if (seconds <= 0) return 'date unknown';
    return new Date(seconds * 1000).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
  }

  function recordIcon(key: string) {
    const icon = types.find((type) => type.key === key)?.icon ?? key;
    return ({
      building: Building2, flask: FlaskConical, user: UserRound, graduation: GraduationCap,
      folder: FolderKanban, book: BookText, layers: Layers3,
      mail: Mail, file: FileText, note: StickyNote
    } as Record<string, typeof Home>)[icon] ?? FileText;
  }

  async function moveType(source: string, target: string) {
    if (source === target) return;
    const order = orderedTypes.map((type) => type.key);
    const from = order.indexOf(source);
    const to = order.indexOf(target);
    if (from < 0 || to < 0) return;
    order.splice(from, 1);
    order.splice(to, 0, source);
    sidebarOrder = order;
    try {
      await api.putSetting('sidebar_order', order);
    } catch (cause) {
      error = message(cause);
    }
  }

  function dropType(event: DragEvent, target: string) {
    event.preventDefault();
    const source = draggedType;
    dragFinishedAt = Date.now();
    draggedType = null;
    dropTarget = null;
    if (source) void moveType(source, target);
  }

  function typeKeydown(event: KeyboardEvent, key: string) {
    if (!event.altKey || !['ArrowUp', 'ArrowDown'].includes(event.key)) return;
    const order = orderedTypes.map((type) => type.key);
    const index = order.indexOf(key);
    const other = event.key === 'ArrowUp' ? order[index - 1] : order[index + 1];
    if (other) {
      event.preventDefault();
      void moveType(key, other);
    }
  }

  function onGlobalKey(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null;
    if (target?.closest('input, textarea, select, [contenteditable="true"]')) return;
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      document.getElementById('global-search')?.focus();
    }
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'n') {
      event.preventDefault();
      startCreate();
    }
  }
</script>

<svelte:window onkeydown={onGlobalKey} />

<div class="app-shell">
  <aside class="sidebar">
    <div class="brand">
      <span class="brand-mark">e</span>
      <div>
        <div class="brand-name">eduport</div>
        <div class="brand-caption">Research workspace</div>
      </div>
    </div>

    <nav aria-label="Main navigation">
      <button class:active={active === 'home'} class="nav-button" onclick={() => navigate('home')}>
        <Home size={16} strokeWidth={1.8} /> Overview
      </button>
      <button class:active={active === 'all'} class="nav-button" onclick={() => navigate('all')}>
        <BookOpen size={16} strokeWidth={1.8} /> All records <span class="count">{liveEntries.length}</span>
      </button>
      <button class:active={active === 'assistant'} class="nav-button" onclick={() => navigate('assistant')}>
        <Sparkles size={16} strokeWidth={1.8} /> Assistant
      </button>
      <div class="nav-label">Workspace</div>
      {#each orderedTypes as type (type.key)}
        {@const TypeIcon = recordIcon(type.key)}
        <button class:active={active === type.key} class:drag-target={dropTarget === type.key} class:dragging={draggedType === type.key} class="nav-button type-nav" draggable="true" ondragstart={(event) => { draggedType = type.key; event.dataTransfer?.setData('text/plain', type.key); }} ondragend={() => { dragFinishedAt = Date.now(); draggedType = null; dropTarget = null; }} ondragover={(event) => { event.preventDefault(); dropTarget = type.key; }} ondragleave={() => { if (dropTarget === type.key) dropTarget = null; }} ondrop={(event) => dropType(event, type.key)} onkeydown={(event) => typeKeydown(event, type.key)} onclick={() => { if (Date.now() - dragFinishedAt > 350) navigate(type.key); }} title="Drag to reorder, or use Alt + arrow keys">
          <TypeIcon size={16} strokeWidth={1.8} /> <span class="nav-text">{type.label}</span>
          {#if countByType[type.key]}<span class="count">{countByType[type.key]}</span>{/if}
          <GripVertical class="drag-grip" size={14} strokeWidth={1.7} />
        </button>
      {/each}
      <div class="nav-divider"></div>
      <button class:active={active === 'trash'} class="nav-button" onclick={() => navigate('trash')}>
        <Trash2 size={16} strokeWidth={1.8} /> Trash
      </button>
    </nav>
    <div class="sidebar-bottom">
      <button class:active={active === 'settings'} class="nav-button" onclick={() => navigate('settings')}>
        <Settings2 size={16} strokeWidth={1.8} /> Settings & fields
      </button>
      <p class="sidebar-note">Your records live in a local SQLite database on this device.</p>
    </div>
  </aside>

  <div class="workspace">
    <header class="topbar">
      <div class="breadcrumb">Workspace <ChevronRight size={13} style="vertical-align:middle" /> <strong>{active === 'home' ? 'Overview' : activeType?.label ?? (active === 'all' ? 'All records' : active === 'search' ? 'Search' : active === 'trash' ? 'Trash' : active === 'assistant' ? 'Assistant' : 'Settings')}</strong></div>
      <label class="topbar-search">
        <Search size={15} strokeWidth={1.7} />
        <input id="global-search" aria-label="Search records" placeholder="Search your workspace..." bind:value={searchQuery} oninput={() => { if (searchQuery) navigate('search'); else if (active === 'search') navigate('home'); }} />
      </label>
      {#if !inDesktop}<span class="preview-pill">LOCAL SQLITE</span>{/if}
      <button class="topbar-icon" title="Help" aria-label="Help" onclick={() => notify('Tip: use ⌘K to search and ⌘N to create a record.')}><CircleHelp size={18} /></button>
      <button class="primary-button" onclick={() => startCreate()}><Plus size={15} /> New record</button>
    </header>

    <main class="content">
      {#if error && !editorOpen && !editingType && !relationOpen}<div class="error-box page-error" role="alert">{error}</div>{/if}
      {#if loading}
        <div class="empty-state"><Clock3 size={25} /> <h3>Opening your workspace…</h3></div>
      {:else if error && !editorOpen && !relationOpen && !editingType}
        <div class="error-box" role="alert">{error}</div>
      {/if}

      {#if !loading && active === 'home'}
        <section class="overview-hero">
          <div class="section-kicker"><Sparkles size={14} /> Your academic workspace</div>
          <h1>Make room for what comes next.</h1>
          <p>Keep programs, labs, people, research and applications connected in one calm place — from first curiosity to final decision.</p>
          <button class="primary-button" onclick={() => startCreate('application')}>Start an application <ArrowRight size={15} /></button>
        </section>
        <div class="stat-grid">
          <div class="stat-card"><div class="top"><span>Applications</span><FolderKanban size={16} /></div><strong>{countByType.application ?? 0}</strong></div>
          <div class="stat-card"><div class="top"><span>Programs</span><BookOpen size={16} /></div><strong>{countByType.program ?? 0}</strong></div>
          <div class="stat-card"><div class="top"><span>People & labs</span><Link2 size={16} /></div><strong>{(countByType.person ?? 0) + (countByType.lab ?? 0)}</strong></div>
          <div class="stat-card"><div class="top"><span>Research</span><FileText size={16} /></div><strong>{(countByType.publication ?? 0) + (countByType.project ?? 0)}</strong></div>
        </div>
        <div class="section-heading"><h2>Recently updated</h2><button class="topbar-icon" title="All records" aria-label="All records" onclick={() => navigate('all')}><ArrowRight size={17} /></button></div>
        {#if liveEntries.length}
          <div class="record-grid">
            {#each [...liveEntries].sort((a, b) => b.updated_at - a.updated_at).slice(0, 3) as entry}
              <button class="record-card" onclick={() => { navigate(entry.type_key); void choose(entry); }}>
                <span class="record-type"><span class="type-dot"></span>{typeLabel(entry.type_key)}</span>
                <h3>{entry.title}</h3>
                <p>{entry.body.slice(0, 95) || 'No description yet'}{entry.body.length > 95 ? '…' : ''}</p>
              </button>
            {/each}
          </div>
        {:else}
          <div class="empty-state list-panel"><BookOpen size={25} /><h3>Start with one record</h3><p>Add a program, person or lab to begin connecting your research.</p><button class="secondary-button" onclick={() => startCreate('program')}>Add a program</button></div>
        {/if}
        {#if upcoming.length}
          <div class="section-heading"><h2>Upcoming deadlines</h2><span>From your applications</span></div>
          <div class="list-panel">
            {#each upcoming as entry}
              <button class="record-row" onclick={() => { navigate('application'); void choose(entry); }}>
                <span><span class="title">{entry.title}</span><span class="sub">{String(entry.values.intake ?? 'Application')}</span></span>
                <span class="meta">{String(entry.values.deadline)}</span>
                <span class="kind">{String(entry.values.status ?? 'planning')}</span>
                <ChevronRight size={15} />
              </button>
            {/each}
          </div>
        {/if}
      {:else if !loading && active === 'assistant'}
        <div class="page-head"><div><div class="section-kicker"><Sparkles size={14} /> Native AI</div><h1>Research assistant</h1><p>The agent can inspect every entity type through bounded tools. Writes follow your per-type permissions.</p></div><button class="secondary-button" onclick={() => navigate('settings')}><Settings2 size={14} /> Provider settings</button></div>
        <div class="assistant-layout">
          <section class="assistant-card">
            <div class="assistant-provider"><span class="record-type"><span class="type-dot"></span>{aiConfig.provider === 'ollama' ? 'Ollama · local' : 'OpenRouter · cloud'}</span><span class="muted">{aiConfig.model || 'Choose a model in Settings'} · {aiConfig.trust_mode} mode</span></div>
            <label class="form-field wide"><span>What would you like to work on?</span><textarea bind:value={assistantPrompt} placeholder="Compare programs, find a contact, or ask the agent to update records."></textarea></label>
            {#if aiConfig.provider === 'openrouter'}
              <label class="cloud-consent"><input type="checkbox" bind:checked={allowCloud} /> For this request, allow the agent to send any records or imported files it reads to OpenRouter.</label>
            {/if}
            <button class="primary-button" disabled={assistantBusy || !assistantPrompt.trim() || !aiConfig.model || (aiConfig.provider === 'openrouter' && !allowCloud)} onclick={askAssistant}><Sparkles size={15} /> {assistantBusy ? 'Thinking…' : 'Ask assistant'}</button>
            {#if error}<div class="error-box" role="alert">{error}</div>{/if}
            {#if assistantResult}
              <div class="assistant-answer"><div class="section-kicker">Response</div><p>{assistantResult.answer}</p><span class="muted" style="font-size:10px">{assistantResult.provider} · {assistantResult.model} · {assistantResult.read_calls} reads · {assistantResult.write_calls} writes</span></div>
              {#each assistantResult.actions as outcome}<div class="suggestion-card"><div class="section-kicker">{outcome.action.status} · {outcome.action.tool.replaceAll('_', ' ')}</div><pre>{JSON.stringify(outcome.action.arguments, null, 2)}</pre>{#if outcome.action.error}<div class="error-box">{outcome.action.error}</div>{/if}</div>{/each}
            {/if}
          </section>
          <aside class="assistant-context">
            <div class="section-kicker">Database access</div><h3>{liveEntries.length} records available</h3><p>The agent reads through paginated tools. It cannot send mail or permanently delete records.</p>
            <div class="permission-summary"><strong>Unrestricted writes</strong><span>{aiConfig.trust_mode === 'delegated' && aiConfig.delegated_types.length ? aiConfig.delegated_types.map(typeLabel).join(', ') : 'None'}</span><small>All other entity types require approval.</small></div>
            <div class="section-heading"><h3>Pending approval</h3><span>{pendingActions.length}</span></div>
            {#each pendingActions as action (action.id)}<div class="pending-action"><strong>{action.tool.replaceAll('_', ' ')}</strong><small>{action.target_types.map(typeLabel).join(', ') || 'New entity type'} · {action.status}</small><pre>{JSON.stringify(action.arguments, null, 2)}</pre>{#if action.status === 'pending'}<div><button class="secondary-button" onclick={() => approveAction(action)}>Approve</button><button class="secondary-button danger-button" onclick={() => rejectAction(action)}>Reject</button></div>{:else}<p>Application was interrupted. Inspect the database before retrying.</p>{/if}</div>{/each}
            {#if !pendingActions.length}<p class="muted">No changes are waiting for approval.</p>{/if}
          </aside>
        </div>
      {:else if !loading && active === 'settings'}
        <div class="page-head"><div><div class="section-kicker">Shape your workspace</div><h1>Settings & fields</h1><p>Define entity types, their fields and relationships.</p></div><button class="primary-button" onclick={startNewType}><Plus size={15} /> New entity type</button></div>
        <div class="section-heading first-section-heading"><h2>Record fields</h2><span>Choose a type to add a field</span></div>
        <div class="list-panel settings-fields-list">
          {#each orderedTypes as type}
            <button class="record-row" onclick={() => startFieldEditor(type)}>
              <span><span class="title">{type.label}</span><span class="sub">{type.key}</span></span>
              <span class="meta">{type.fields.length} {type.fields.length === 1 ? 'field' : 'fields'}</span><span></span><ChevronRight size={15} />
            </button>
          {/each}
        </div>
        <div class="section-heading"><h2>Relationships</h2><button class="secondary-button" onclick={() => startDefinition()}><Plus size={14} /> Define relationship</button></div>
        <div class="list-panel relationship-definitions">
          {#each definitions as definition (definition.id)}
            <div class="relationship-definition-row"><span><strong>{typeLabel(definition.source_type)}</strong> <ArrowRight size={13} /> <strong>{typeLabel(definition.target_type)}</strong></span><span>{definition.name} <small>↔ {definition.inverse_name}</small></span></div>
          {/each}
        </div>
        <div class="settings-card">
          <div>
            <div class="section-kicker">Keep your work portable</div>
            <h2>Data & recovery</h2>
            <p>Preview an old Eduport Markdown vault before importing. Export readable Markdown or save a consistent SQLite backup at any time.</p>
          </div>
          <div class="settings-actions">
            <button class="secondary-button" disabled={!inDesktop} onclick={previewImport}>Import Markdown</button>
            <button class="secondary-button" disabled={!inDesktop} onclick={exportMarkdown}><Download size={14} /> Export Markdown</button>
            <button class="secondary-button" disabled={!inDesktop} onclick={backupDatabase}>Back up database</button>
          </div>
          {#if !inDesktop}<small class="desktop-only-note">File dialogs are available in the desktop app. Records here use the same SQLite database.</small>{/if}
          {#if importPreview}
            <div class="import-preview">
              <strong>{importPreview.format}</strong>
              <p>{importPreview.rows.length} records and {importPreview.file_count} files found. Existing records are kept; conflicting IDs are never overwritten. The source folder stays untouched.</p>
              {#if importPreview.rows.length}
                <div class="preview-rows">
                  {#each importPreview.rows.slice(0, 8) as row}<span>{row.title} <small>{row.type_key}{row.deleted ? ' · trash' : ''}</small></span>{/each}
                  {#if importPreview.rows.length > 8}<span>And {importPreview.rows.length - 8} more…</span>{/if}
                </div>
              {/if}
              {#each importPreview.warnings.slice(0, 8) as warning}<p class="import-warning">⚠ {warning}</p>{/each}
              {#each importPreview.blockers as blocker}<p class="error-box">{blocker}</p>{/each}
              {#if importConflicts.length}<p class="error-box">{importConflicts.length} record IDs already exist in this workspace. Resolve the conflicts before importing.</p>{/if}
              <button class="primary-button" disabled={!!importPreview.blockers.length || !!importConflicts.length} onclick={applyImport}>Import these records</button>
            </div>
          {/if}
        </div>
        <div class="settings-card">
          <div><div class="section-kicker">Your choice of model</div><h2>Assistant provider</h2><p>The agent can browse records and imported files through bounded tools. Ollama stays local; each OpenRouter request needs explicit consent to share the data it reads. Keys stay in the OS credential store.</p></div>
          <div class="ai-settings-grid">
            <label class="form-field"><span>Provider</span><select bind:value={aiConfig.provider} onchange={() => { aiConfig.model = ''; allowCloud = false; }}><option value="ollama">Ollama (local)</option><option value="openrouter">OpenRouter (cloud)</option></select></label>
            <label class="form-field"><span>Model name</span><input bind:value={aiConfig.model} placeholder={aiConfig.provider === 'ollama' ? 'e.g. llama3.2' : 'e.g. openrouter/free'} /></label>
            <label class="form-field"><span>Write permissions</span><select bind:value={aiConfig.trust_mode}><option value="controlled">Review every write</option><option value="delegated">Unrestricted for chosen types</option></select></label>
            {#if aiConfig.trust_mode === 'delegated'}
              <div class="form-field wide"><span>Entity types with unrestricted writes (all others stay gated)</span><div class="delegated-types">{#each orderedTypes as type}<label><input type="checkbox" checked={aiConfig.delegated_types.includes(type.key)} onchange={() => aiConfig.delegated_types = aiConfig.delegated_types.includes(type.key) ? aiConfig.delegated_types.filter((key) => key !== type.key) : [...aiConfig.delegated_types, type.key]} />{type.label}</label>{/each}</div></div>
            {/if}
            {#if aiConfig.provider === 'openrouter'}
              <label class="form-field wide"><span>OpenRouter API key {keyConfigured ? '(configured)' : ''}</span><input type="password" autocomplete="off" bind:value={openrouterKey} placeholder="Paste a key; it will not appear in exports" /></label>
              <button class="secondary-button" disabled={!openrouterKey.trim()} onclick={saveOpenRouterKey}>Save key</button>
            {/if}
            <button class="primary-button" onclick={saveAiConfig}>Save provider settings</button>
          </div>
        </div>
        <div class="settings-card mail-settings">
          <div><div class="section-kicker"><Mail size={14} /> Selected message import</div><h2>Gmail</h2><p>Search your account, select messages, inspect their contents, then import. Eduport never sends mail or syncs your inbox. The original message is included in Markdown export.</p></div>
          {#if !inDesktop}<small class="desktop-only-note">Gmail sign-in and import use the desktop app.</small>{/if}
          <div class="mail-settings-content">
            {#if gmailAccount}
              <div class="provider-connected"><Check size={14} /> Connected as {gmailAccount.email}</div>
              <div class="mail-query"><input aria-label="Search Gmail" disabled={!inDesktop} bind:value={gmailQuery} placeholder="Gmail search, e.g. from:university.edu" onkeydown={(event) => { if (event.key === 'Enter') void searchGmail(); }} /><button class="secondary-button" disabled={gmailBusy || !inDesktop} onclick={searchGmail}><Search size={14} /> Search</button></div>
              {#if gmailMessages.length}
                <div class="mail-results">
                  {#each gmailMessages as item}
                    <label class="mail-result"><input type="checkbox" disabled={item.imported || gmailBusy} checked={gmailSelected.includes(item.id)} onchange={() => toggleGmail(item.id)} /><span><strong>{item.subject || '(No subject)'}</strong><small>{item.from} · {item.date}{item.imported ? ' · Imported' : ''}</small><em>{item.snippet}</em></span></label>
                  {/each}
                </div>
              {/if}
              {#if gmailSelected.length}<button class="secondary-button" disabled={gmailBusy || !inDesktop} onclick={previewSelectedMail}>Preview {gmailSelected.length} selected {gmailSelected.length === 1 ? 'message' : 'messages'}</button>{/if}
              {#if gmailPreview.length}
                <div class="mail-preview"><div class="section-kicker">Import preview</div>
                  {#each gmailPreview as item}<div class="mail-preview-item"><strong>{item.subject}</strong><small>{item.from} → {item.to} · {item.date} · {item.raw_bytes} original bytes</small><p>{item.body}</p></div>{/each}
                  <button class="primary-button" disabled={gmailBusy || !inDesktop || gmailPreview.some((item) => item.imported)} onclick={importSelectedMail}>Import selected messages</button>
                </div>
              {/if}
              <details class="reconnect-provider"><summary>Switch Gmail account</summary><label class="form-field"><span>Google OAuth desktop client ID</span><input bind:value={gmailClientId} placeholder="...apps.googleusercontent.com" autocomplete="off" /></label><button class="secondary-button" disabled={gmailBusy || !inDesktop || !gmailClientId.trim()} onclick={connectGmail}>Connect account</button></details>
            {:else}
              <label class="form-field"><span>Google OAuth desktop client ID</span><input bind:value={gmailClientId} placeholder="...apps.googleusercontent.com" autocomplete="off" /></label>
              <button class="primary-button" disabled={gmailBusy || !inDesktop || !gmailClientId.trim()} onclick={connectGmail}>{gmailBusy ? 'Connecting…' : 'Connect Gmail'}</button>
              <small class="muted">Create a Desktop OAuth client in Google Cloud with the Gmail read-only scope. Your refresh token stays in the OS credential store.</small>
            {/if}
          </div>
        </div>
      {:else if !loading}
        <div class="page-head">
          <div>
            <div class="section-kicker">{active === 'trash' ? 'Recoverable records' : 'Your research collection'}</div>
            <h1>{activeType?.label ?? (active === 'all' ? 'All records' : active === 'search' ? 'Search results' : 'Trash')}</h1>
            <p>{activeType ? tableCount : visibleEntries.length} {(activeType ? tableCount : visibleEntries.length) === 1 ? 'record' : 'records'} in this view</p>
          </div>
          {#if activeType}<button class="secondary-button" onclick={() => startFieldEditor(activeType)}><Settings2 size={15} /> Fields</button>{/if}
        </div>
        <div class:detail-layout={selected} class="list-layout">
          <div class="list-panel">
            {#if activeType}
              <div class="view-tabs" aria-label="Saved table views">
                {#each views as view (view.id)}<button class:active={currentView?.id === view.id} onclick={() => { currentViewId = view.id; selectedId = null; }}>{view.name}</button>{/each}
                <button class="view-add" onclick={() => startViewEditor()}><Plus size={14} /> New view</button>
              </div>
            {/if}
            <div class="list-toolbar">
              <label class="small-search"><Search size={15} /><input placeholder="Filter this view..." aria-label="Filter this view" bind:value={listQuery} /></label>
              {#if activeType && currentView && !currentView.is_default}<button class="secondary-button" onclick={() => startViewEditor(currentView)}><SlidersHorizontal size={14} /> Configure view</button>{/if}
              <span class="muted" style="font-size:11px">{activeType ? tableCount : visibleEntries.length} shown</span>
            </div>
            {#if activeType && currentView && tableCount}
              <div class="table-scroll">
                {#each tableGroups as group (group.key)}
                  {#if currentView.config.group_by}<div class="table-group-heading">{group.key} <span>{group.rows.length}</span></div>{/if}
                  <table class="entity-table"><thead><tr>{#each currentView.config.columns as column}<th>{fieldLabel(activeType, column)}</th>{/each}</tr></thead>
                    <tbody>{#each group.rows as entry (entry.id)}<tr class:selected={selectedId === entry.id}>
                      {#each currentView.config.columns as column}<td>{#if column === 'title'}<button class="table-title" onclick={() => choose(entry)}>{entry.title}</button>{:else}{displayValue(cellValue(entry, column, relationIndex))}{/if}</td>{/each}
                    </tr>{/each}</tbody>
                  </table>
                {/each}
              </div>
            {:else if !activeType && visibleEntries.length}
              {#each visibleEntries as entry}
                <button class:selected={selectedId === entry.id} class="record-row" onclick={() => choose(entry)}>
                  <span><span class="title">{entry.title}</span><span class="sub">{entry.body.slice(0, 68) || 'No description'}</span></span>
                  <span class="meta">Updated {compactDate(entry.updated_at)}</span>
                  <span class="kind">{typeLabel(entry.type_key)}</span>
                  <ChevronRight size={15} />
                </button>
              {/each}
            {:else}
              <div class="empty-state">
                <BookOpen size={26} />
                <h3>{active === 'trash' ? 'Trash is empty' : 'Nothing here yet'}</h3>
                <p>{active === 'trash' ? 'Deleted records can be restored here.' : activeType && currentView?.config.filters.length ? 'No records match this view. Adjust its filters or create a record.' : 'Create a record or change your filter.'}</p>
                {#if active !== 'trash'}<button class="secondary-button" onclick={() => startCreate()}>Create a record</button>{/if}
              </div>
            {/if}
          </div>
          {#if selected}
            <div class="detail-panel">
              <div style="display:flex;justify-content:space-between;align-items:center">
                <span class="record-type"><span class="type-dot"></span>{typeLabel(selected.type_key)}</span>
                <button class="icon-button" aria-label="Close detail" title="Close detail" onclick={() => selectedId = null}><X size={17} /></button>
              </div>
              <h2>{selected.title}</h2>
              <span class="muted" style="font-size:11px">Updated {compactDate(selected.updated_at)} · Revision {selected.revision}</span>
              {#if Object.keys(selected.values).length}
                <dl class="property-list">
                  {#each Object.entries(selected.values) as [key, value]}
                    <div class="property-row"><dt>{types.find((type) => type.key === selected.type_key)?.fields.find((field) => field.key === key)?.label ?? key}</dt><dd>{String(value)}</dd></div>
                  {/each}
                </dl>
              {/if}
              <p class="body">{selected.body || 'Add notes and context to this record.'}</p>
              <div class="section-heading"><h3>Connections</h3><button class="secondary-button" onclick={() => startRelation(selected)}><Plus size={14} /> Link</button></div>
              {#if relations.length}
                {#each relations as relation}
                  {@const otherId = relation.source_id === selected.id ? relation.target_id : relation.source_id}
                  {@const other = entries.find((entry) => entry.id === otherId)}
                  {@const definition = definitions.find((item) => item.id === relation.definition_id)}
                  <div class="relation-row"><span><strong>{other?.title ?? 'Missing record'}</strong> <span class="muted"> · {definition ? (relation.source_id === selected.id ? definition.name : definition.inverse_name) : relation.kind}{relation.role ? ` · ${relation.role}` : ''}{relation.start_date ? ` · ${relation.start_date}` : ''}{relation.end_date ? `–${relation.end_date}` : ''}</span></span><button aria-label="Remove relation" title="Remove relation" onclick={() => removeRelation(relation.id)}><X size={14} /></button></div>
                {/each}
              {:else}
                <p class="muted" style="font-size:12px">No connected records yet.</p>
              {/if}
              <div class="actions">
                {#if selected.deleted_at === null}
                  <button class="secondary-button" onclick={() => startEdit(selected)}>Edit record</button>
                  <button class="secondary-button" onclick={() => { assistantPrompt = `Tell me about record ${selected.id} (${selected.title}).`; navigate('assistant'); }}><Sparkles size={14} /> Ask assistant</button>
                  <button class="secondary-button danger-button" onclick={() => toggleDeleted(selected)}><Trash2 size={14} /> Move to Trash</button>
                {:else}
                  <button class="secondary-button" onclick={() => toggleDeleted(selected)}><ArrowLeft size={14} /> Restore</button>
                {/if}
              </div>
            </div>
          {/if}
        </div>
      {/if}
    </main>
  </div>
</div>

{#if editorOpen}
  <div class="dialog-scrim" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) editorOpen = false; }}>
    <div class="dialog" role="dialog" aria-modal="true" aria-label={draft.id ? 'Edit record' : 'New record'}>
      <div class="dialog-head"><div><div class="section-kicker">{draft.id ? 'Update your research' : 'Add to your workspace'}</div><h2>{draft.id ? 'Edit record' : 'New record'}</h2></div><button class="icon-button" aria-label="Close" onclick={() => editorOpen = false}><X size={18} /></button></div>
      <form onsubmit={(event) => { event.preventDefault(); void saveEntry(); }}>
        <div class="form-grid">
          <label class="form-field"><span>Record type</span><select value={draft.type_key} disabled={!!draft.id} onchange={(event) => changeDraftType(event.currentTarget.value)}>{#each orderedTypes as type}<option value={type.key}>{type.label}</option>{/each}</select></label>
          <label class="form-field"><span>Title <b class="required-mark">Required</b></span><input bind:value={draft.title} required placeholder="A clear, searchable name" /></label>
          {#each editType?.fields ?? [] as field (field.key)}
            <label class="form-field" class:wide={field.kind === 'text'}>
              <span>{field.label} <b class:required-mark={field.required} class:optional-mark={!field.required}>{field.required ? 'Required' : 'Optional'}</b></span>
              {#if field.kind === 'select'}
                <select required={field.required} value={String(draft.values[field.key] ?? '')} onchange={(event) => draft.values[field.key] = event.currentTarget.value}>
                  <option value="">Choose {field.label.toLowerCase()}</option>
                  {#each field.options as option}<option value={option}>{option.replace(/_/g, ' ')}</option>{/each}
                </select>
              {:else if field.kind === 'boolean'}
                <select required={field.required} value={draft.values[field.key] === true ? 'true' : draft.values[field.key] === false ? 'false' : ''} onchange={(event) => draft.values[field.key] = event.currentTarget.value === '' ? null : event.currentTarget.value === 'true'}><option value="">Choose an answer</option><option value="true">Yes</option><option value="false">No</option></select>
              {:else}
                <input required={field.required} type={field.kind === 'date' ? 'date' : field.kind === 'number' ? 'number' : field.kind === 'url' ? 'url' : 'text'} value={String(draft.values[field.key] ?? '')} oninput={(event) => draft.values[field.key] = field.kind === 'number' ? (event.currentTarget.value ? Number(event.currentTarget.value) : '') : event.currentTarget.value} />
              {/if}
            </label>
          {/each}
          <label class="form-field wide"><span>Notes & context</span><textarea bind:value={draft.body} placeholder="What matters about this record?"></textarea></label>
          <label class="form-field wide"><span>Tags (comma separated)</span><input bind:value={tagText} placeholder="e.g. fall-2027, reach" /></label>
        </div>
        {#if error}<div class="error-box" role="alert">{error}</div>{/if}
        <div class="dialog-footer"><button type="button" class="secondary-button" onclick={() => editorOpen = false}>Cancel</button><button type="submit" class="primary-button"><Check size={15} /> Save record</button></div>
      </form>
    </div>
  </div>
{/if}

{#if relationOpen}
  <div class="dialog-scrim" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) relationOpen = false; }}>
    <div class="dialog" role="dialog" aria-modal="true" aria-label="Link records">
      <div class="dialog-head"><div><div class="section-kicker">Connect the dots</div><h2>Link a record</h2></div><button class="icon-button" aria-label="Close" onclick={() => relationOpen = false}><X size={18} /></button></div>
      {#if applicableDefinitions.length}
      <div class="form-grid">
        <label class="form-field wide"><span>Relationship type</span><select value={relationDraft.definition_id ?? ''} onchange={(event) => { relationDraft.definition_id = event.currentTarget.value; relationDraft.target_id = ''; }}><option value="">Choose a relationship</option>{#each applicableDefinitions as definition}<option value={definition.id}>{definition.source_type === selected?.type_key ? definition.name : definition.inverse_name} → {typeLabel(definition.source_type === selected?.type_key ? definition.target_type : definition.source_type)}</option>{/each}</select></label>
        <label class="form-field wide"><span>Related {relationTargetType ? typeLabel(relationTargetType) : 'record'}</span><select bind:value={relationDraft.target_id}><option value="">Choose a record</option>{#each liveEntries.filter((item) => item.id !== selected?.id && item.type_key === relationTargetType) as entry}<option value={entry.id}>{entry.title}</option>{/each}</select></label>
        <label class="form-field"><span>Role (optional)</span><input value={relationDraft.role ?? ''} oninput={(event) => relationDraft.role = event.currentTarget.value || null} placeholder="e.g. PI, author, member" /></label>
        <label class="form-field"><span>Start date (optional)</span><input type="date" value={relationDraft.start_date ?? ''} oninput={(event) => relationDraft.start_date = event.currentTarget.value || null} /></label>
        <label class="form-field"><span>End date (optional)</span><input type="date" value={relationDraft.end_date ?? ''} oninput={(event) => relationDraft.end_date = event.currentTarget.value || null} /></label>
      </div>
      {:else}<p class="muted">No relationship is defined for this entity type yet.</p><button class="secondary-button" onclick={() => { relationOpen = false; startDefinition(selected?.type_key); }}>Define a relationship</button>{/if}
      {#if error}<div class="error-box" role="alert">{error}</div>{/if}
      <div class="dialog-footer"><button class="secondary-button" onclick={() => relationOpen = false}>Cancel</button>{#if applicableDefinitions.length}<button class="primary-button" onclick={saveRelation}>Add connection</button>{/if}</div>
    </div>
  </div>
{/if}

{#if editingType}
  <div class="dialog-scrim" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) editingType = null; }}>
    <div class="dialog" role="dialog" aria-modal="true" aria-label="Customize fields">
      <div class="dialog-head"><div><div class="section-kicker">Make it yours</div><h2>{editingType.label} fields</h2></div><button class="icon-button" aria-label="Close" onclick={() => editingType = null}><X size={18} /></button></div>
      <div class="list-panel">
        {#each editingType.fields as field}<div class="relation-row" style="padding:12px 15px"><span>{field.label} <small class:required-mark={field.required} class:optional-mark={!field.required}>{field.required ? 'Required' : 'Optional'}</small></span><span class="muted">{field.kind}</span></div>{/each}
        {#if editingType.fields.length === 0}<div class="empty-state" style="padding:25px">No fields yet.</div>{/if}
      </div>
      <div class="section-heading"><h3>Add a field</h3></div>
      <div class="form-grid">
        <label class="form-field"><span>Key</span><input bind:value={newField.key} placeholder="e.g. priority" /></label>
        <label class="form-field"><span>Label</span><input bind:value={newField.label} placeholder="e.g. Priority" /></label>
        <label class="form-field"><span>Type</span><select bind:value={newField.kind}><option value="text">Text</option><option value="number">Number</option><option value="date">Date</option><option value="boolean">Checkbox</option><option value="url">URL</option><option value="select">Select</option></select></label>
        {#if newField.kind === 'select'}<label class="form-field wide"><span>Choices (comma separated)</span><input bind:value={newFieldOptions} placeholder="e.g. planned, active, complete" /></label>{/if}
        <label class="field-required-toggle"><input type="checkbox" bind:checked={newField.required} /> Require a value for new records</label>
      </div>
      {#if error}<div class="error-box" role="alert">{error}</div>{/if}
      <div class="dialog-footer"><button class="secondary-button" onclick={() => editingType = null}>Done</button><button class="primary-button" onclick={addField}><Plus size={15} /> Add field</button></div>
    </div>
  </div>
{/if}

{#if newTypeOpen}
  <div class="dialog-scrim" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) newTypeOpen = false; }}>
    <div class="dialog" role="dialog" aria-modal="true" aria-label="Create entity type">
      <div class="dialog-head"><div><div class="section-kicker">Your own database</div><h2>New entity type</h2></div><button class="icon-button" aria-label="Close" onclick={() => newTypeOpen = false}><X size={18} /></button></div>
      <p class="muted">Create a collection with its own fields, table views and relationships.</p>
      <div class="form-grid">
        <label class="form-field"><span>Name (plural)</span><input bind:value={newTypeLabel} placeholder="e.g. Scholarships" /></label>
        <label class="form-field"><span>Key (optional)</span><input bind:value={newTypeKey} placeholder="e.g. scholarship" /></label>
        <label class="form-field"><span>Icon</span><select bind:value={newTypeIcon}><option value="file">Document</option><option value="building">Institution</option><option value="flask">Lab</option><option value="user">Person</option><option value="graduation">Education</option><option value="folder">Folder</option><option value="book">Book</option><option value="layers">Project</option><option value="mail">Mail</option><option value="note">Note</option></select></label>
      </div>
      {#if error}<div class="error-box" role="alert">{error}</div>{/if}
      <div class="dialog-footer"><button class="secondary-button" onclick={() => newTypeOpen = false}>Cancel</button><button class="primary-button" onclick={saveNewType}>Create type</button></div>
    </div>
  </div>
{/if}

{#if definitionOpen}
  <div class="dialog-scrim" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) definitionOpen = false; }}>
    <div class="dialog" role="dialog" aria-modal="true" aria-label="Define relationship">
      <div class="dialog-head"><div><div class="section-kicker">Entity schema</div><h2>Define relationship</h2></div><button class="icon-button" aria-label="Close" onclick={() => definitionOpen = false}><X size={18} /></button></div>
      <p class="muted">Records of these types can then be linked. Each record may have multiple links.</p>
      <div class="form-grid">
        <label class="form-field"><span>From type</span><select bind:value={definitionDraft.source_type}>{#each orderedTypes as kind}<option value={kind.key}>{kind.label}</option>{/each}</select></label>
        <label class="form-field"><span>To type</span><select bind:value={definitionDraft.target_type}>{#each orderedTypes as kind}<option value={kind.key}>{kind.label}</option>{/each}</select></label>
        <label class="form-field"><span>Label from first type</span><input bind:value={definitionDraft.name} placeholder="e.g. Institutions" /></label>
        <label class="form-field"><span>Label from second type</span><input bind:value={definitionDraft.inverse_name} placeholder="e.g. Labs" /></label>
      </div>
      {#if error}<div class="error-box" role="alert">{error}</div>{/if}
      <div class="dialog-footer"><button class="secondary-button" onclick={() => definitionOpen = false}>Cancel</button><button class="primary-button" onclick={saveDefinition}>Add relationship</button></div>
    </div>
  </div>
{/if}

{#if viewDraft && viewKind}
  <div class="dialog-scrim" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) viewDraft = null; }}>
    <div class="dialog view-dialog" role="dialog" aria-modal="true" aria-label="Configure table view">
      <div class="dialog-head"><div><div class="section-kicker">{viewKind.label} table</div><h2>{viewDraft.id ? 'Edit view' : 'New view'}</h2></div><button class="icon-button" aria-label="Close" onclick={() => viewDraft = null}><X size={18} /></button></div>
      <label class="form-field"><span>View name</span><input bind:value={viewDraft.name} placeholder="e.g. Fall 2027 shortlist" /></label>
      <div class="view-editor-section"><h3>Columns</h3><div class="view-column-options">{#each viewFields as field}<label><input type="checkbox" checked={viewDraft.config.columns.includes(field)} disabled={field === 'title'} onchange={() => toggleViewColumn(field)} />{fieldLabel(viewKind, field)}</label>{/each}</div></div>
      <div class="view-editor-section"><div class="section-heading"><h3>Filters</h3><button class="secondary-button" onclick={() => viewDraft?.config.filters.push({ field: 'title', operator: 'contains', value: '' })}><Plus size={13} /> Add filter</button></div><p class="muted">Records must match every filter.</p>
        {#each viewDraft.config.filters as filter, index}<div class="view-rule"><select bind:value={filter.field}>{#each viewFields as field}<option value={field}>{fieldLabel(viewKind, field)}</option>{/each}</select><select bind:value={filter.operator}><option value="contains">contains</option><option value="equals">equals</option><option value="not_equals">does not equal</option><option value="is_empty">is empty</option><option value="greater_than">greater than / after</option><option value="less_than">less than / before</option></select>{#if filter.operator !== 'is_empty'}<input bind:value={filter.value} placeholder="Value" />{/if}<button class="icon-button" aria-label="Remove filter" onclick={() => { if (viewDraft) viewDraft.config.filters = viewDraft.config.filters.filter((_, i) => i !== index); }}><X size={15} /></button></div>{/each}
      </div>
      <div class="view-editor-section"><div class="section-heading"><h3>Sort</h3><button class="secondary-button" onclick={() => viewDraft?.config.sorts.push({ field: 'title', direction: 'asc' })}><Plus size={13} /> Add sort</button></div>
        {#each viewDraft.config.sorts as sort, index}<div class="view-rule"><select bind:value={sort.field}>{#each viewFields as field}<option value={field}>{fieldLabel(viewKind, field)}</option>{/each}</select><select bind:value={sort.direction}><option value="asc">Ascending</option><option value="desc">Descending</option></select><button class="icon-button" aria-label="Remove sort" onclick={() => { if (viewDraft) viewDraft.config.sorts = viewDraft.config.sorts.filter((_, i) => i !== index); }}><X size={15} /></button></div>{/each}
      </div>
      <div class="view-editor-section"><label class="form-field"><span>Group by</span><select value={viewDraft.config.group_by ?? ''} onchange={(event) => { if (viewDraft) viewDraft.config.group_by = event.currentTarget.value || null; }}><option value="">No grouping</option>{#each viewFields as field}<option value={field}>{fieldLabel(viewKind, field)}</option>{/each}</select></label></div>
      {#if error}<div class="error-box" role="alert">{error}</div>{/if}
      <div class="dialog-footer">{#if viewDraft.id}<button class="secondary-button danger-button" onclick={removeView}>Delete view</button>{/if}<button class="secondary-button" onclick={() => viewDraft = null}>Cancel</button><button class="primary-button" onclick={saveView}>Save view</button></div>
    </div>
  </div>
{/if}

{#if toast}<div class="toast" role="status">{toast}</div>{/if}
