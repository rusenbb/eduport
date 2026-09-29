<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { Archive, ArchiveRestore, Check, ChevronRight, MessageSquare, Plus, Send, Settings2, Sparkles, X } from '@lucide/svelte';
  import { api, inDesktop } from './api';
  import type { AgentAction, AiConfig, ChatEvent, ChatMessage, ChatSession, RecordType } from './types';

  let { config, types, recordCount, initialPrompt, onSettings, onDataChanged } = $props<{
    config: AiConfig;
    types: RecordType[];
    recordCount: number;
    initialPrompt: string;
    onSettings: () => void;
    onDataChanged: () => Promise<void>;
  }>();

  let sessions = $state<ChatSession[]>([]);
  let currentId = $state<string | null>(null);
  let messages = $state<ChatMessage[]>([]);
  let events = $state<ChatEvent[]>([]);
  let actions = $state<AgentAction[]>([]);
  let pendingActions = $state<AgentAction[]>([]);
  let showArchived = $state(false);
  let loading = $state(true);
  let busy = $state(false);
  let prompt = $state('');
  let pendingPrompt = $state('');
  let allowCloud = $state(false);
  let error = $state('');
  let editingTitle = $state<string | null>(null);
  let titleDraft = $state('');
  let transcript: HTMLDivElement | undefined;

  const current = $derived(sessions.find((session) => session.id === currentId) ?? null);
  const unrestricted = $derived(config.trust_mode === 'delegated'
    ? config.delegated_types.map((key: string) => types.find((type: RecordType) => type.key === key)?.label ?? key)
    : []);

  onMount(() => {
    prompt = initialPrompt;
    void initialize();
    const refreshPending = () => { void api.listAgentActions().then((value) => pendingActions = value).catch(() => {}); };
    const interval = window.setInterval(refreshPending, 5000);
    window.addEventListener('focus', refreshPending);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener('focus', refreshPending);
    };
  });

  function describe(cause: unknown): string {
    return cause instanceof Error ? cause.message : String(cause);
  }

  async function initialize() {
    try {
      [sessions, pendingActions] = await Promise.all([api.listChatSessions(), api.listAgentActions()]);
      if (sessions[0]) await selectSession(sessions[0].id);
    } catch (cause) {
      error = describe(cause);
    } finally {
      loading = false;
    }
  }

  async function refreshSessions() {
    sessions = await api.listChatSessions(showArchived);
  }

  async function refreshConversation(id: string) {
    const [loadedMessages, loadedEvents, loadedActions] = await Promise.all([
      api.listChatMessages(id), api.listChatEvents(id), api.listChatActions(id)
    ]);
    if (currentId !== id) return;
    messages = loadedMessages;
    events = loadedEvents;
    actions = loadedActions;
    await tick();
    transcript?.scrollTo({ top: transcript.scrollHeight, behavior: 'smooth' });
  }

  async function selectSession(id: string) {
    currentId = id;
    messages = [];
    events = [];
    actions = [];
    error = '';
    try {
      await refreshConversation(id);
    } catch (cause) {
      error = describe(cause);
    }
  }

  async function newSession() {
    error = '';
    try {
      if (showArchived) showArchived = false;
      const session = await api.createChatSession();
      await refreshSessions();
      await selectSession(session.id);
      prompt = '';
    } catch (cause) {
      error = describe(cause);
    }
  }

  async function toggleArchived() {
    showArchived = !showArchived;
    currentId = null;
    messages = [];
    events = [];
    actions = [];
    try {
      await refreshSessions();
      if (sessions[0]) await selectSession(sessions[0].id);
    } catch (cause) {
      error = describe(cause);
    }
  }

  async function saveTitle(id: string) {
    if (editingTitle !== id) return;
    editingTitle = null;
    try {
      if (titleDraft.trim()) await api.renameChatSession(id, titleDraft.trim());
      await refreshSessions();
    } catch (cause) {
      error = describe(cause);
    }
  }

  async function archiveSession(id: string, archived: boolean) {
    try {
      await api.setChatArchived(id, archived);
      await refreshSessions();
      if (currentId === id) {
        currentId = null;
        messages = [];
        events = [];
        actions = [];
        if (sessions[0]) await selectSession(sessions[0].id);
      }
    } catch (cause) {
      error = describe(cause);
    }
  }

  async function send() {
    const question = prompt.trim();
    if (!question || !config.model || busy || showArchived || (config.provider === 'openrouter' && !allowCloud)) return;
    error = '';
    busy = true;
    pendingPrompt = question;
    prompt = '';
    let id = currentId;
    try {
      if (!id) {
        const session = await api.createChatSession();
        id = session.id;
        await refreshSessions();
        await selectSession(id);
      }
      await api.askAssistant({ session_id: id, question, allow_cloud: allowCloud });
      allowCloud = false;
      await Promise.all([refreshSessions(), refreshConversation(id), api.listAgentActions().then((value) => pendingActions = value)]);
      await onDataChanged();
    } catch (cause) {
      error = describe(cause);
      if (id) {
        try { await refreshConversation(id); } catch { /* retain the provider error */ }
      }
      if (!messages.some((message) => message.role === 'user' && message.content === question)) prompt = question;
    } finally {
      pendingPrompt = '';
      busy = false;
    }
  }

  async function reviewAction(action: AgentAction, approve: boolean) {
    if (action.status !== 'pending') return;
    if (approve && !inDesktop && !window.confirm(`Apply ${action.tool}? Review its arguments first.`)) return;
    error = '';
    try {
      const outcome = approve ? await api.approveAgentAction(action.id) : null;
      if (!approve) await api.rejectAgentAction(action.id);
      pendingActions = await api.listAgentActions();
      if (currentId) await refreshConversation(currentId);
      if (outcome?.action.status === 'applied') await onDataChanged();
    } catch (cause) {
      error = describe(cause);
    }
  }

  function eventsFor(id: string): ChatEvent[] { return events.filter((event) => event.assistant_message_id === id); }
  function actionsFor(id: string): AgentAction[] { return actions.filter((action) => action.chat_message_id === id); }
  function typeLabel(key: string): string { return types.find((type: RecordType) => type.key === key)?.label ?? key; }
</script>

<div class="page-head assistant-heading">
  <div><div class="section-kicker"><Sparkles size={14} /> Native AI</div><h1>Research assistant</h1><p>Continue a conversation with your research workspace. Every tool step and database change stays attached to its chat.</p></div>
  <button class="secondary-button" onclick={onSettings}><Settings2 size={14} /> Provider settings</button>
</div>

<div class="chat-workspace">
  <aside class="chat-sidebar">
    <div class="chat-sidebar-head"><strong>Chats</strong><button class="icon-button" aria-label="New chat" title="New chat" onclick={newSession}><Plus size={17} /></button></div>
    <button class="chat-new" onclick={newSession}><Plus size={15} /> New chat</button>
    {#if loading}<p class="muted">Loading chats…</p>{/if}
    <div class="chat-session-list">
      {#each sessions as session (session.id)}
        <div class:active={session.id === currentId} class="chat-session">
          {#if editingTitle === session.id}
            <input aria-label="Chat title" bind:value={titleDraft} onkeydown={(event) => { if (event.key === 'Enter') void saveTitle(session.id); if (event.key === 'Escape') editingTitle = null; }} onblur={() => saveTitle(session.id)} />
          {:else}
            <button class="chat-session-open" onclick={() => selectSession(session.id)}><MessageSquare size={14} /><span>{session.title}</span></button>
            <button class="chat-session-action" aria-label={`Rename ${session.title}`} title="Rename" onclick={() => { editingTitle = session.id; titleDraft = session.title; }}><Settings2 size={13} /></button>
            <button class="chat-session-action" aria-label={`${showArchived ? 'Restore' : 'Archive'} ${session.title}`} title={showArchived ? 'Restore' : 'Archive'} onclick={() => archiveSession(session.id, !showArchived)}>{#if showArchived}<ArchiveRestore size={13} />{:else}<Archive size={13} />{/if}</button>
          {/if}
        </div>
      {/each}
      {#if !loading && !sessions.length}<p class="chat-empty-list">{showArchived ? 'No archived chats.' : 'Start a chat to keep your questions and answers together.'}</p>{/if}
    </div>
    <button class="chat-archive-toggle" onclick={toggleArchived}>{#if showArchived}<ChevronRight size={14} /> Active chats{:else}<Archive size={14} /> Archived chats{/if}</button>
  </aside>

  <section class="chat-main">
    <div class="chat-topline"><div><strong>{current?.title ?? 'New chat'}</strong><span>{config.provider === 'ollama' ? 'Ollama · local' : 'OpenRouter · cloud'} · {config.model || 'Choose a model in Settings'}</span></div>{#if current?.archived_at}<span class="muted">Archived</span>{/if}</div>
    <div class="chat-transcript" bind:this={transcript}>
      {#if !messages.length && !pendingPrompt}
        <div class="chat-welcome"><Sparkles size={24} /><h2>Ask about your workspace</h2><p>Compare programs, explore people and labs, or ask the agent to organize records. It will show the tools and approvals it uses.</p></div>
      {/if}
      {#each messages as message (message.id)}
        <div class:from-user={message.role === 'user'} class="chat-message">
          <div class="chat-avatar">{message.role === 'user' ? 'You' : 'e'}</div>
          <div class="chat-message-content"><div class="chat-message-meta"><strong>{message.role === 'user' ? 'You' : 'Eduport'}</strong>{#if message.role === 'assistant'}<span>{message.provider} · {message.model}</span>{/if}</div>
            {#if message.status === 'running'}<p class="chat-thinking">This response may still be running. If Eduport was closed, you can send another message.</p>{:else if message.status === 'failed'}<p class="error-box">{message.error ?? 'The provider request failed.'}</p>{:else}<p>{message.content}</p>{/if}
            {#if message.role === 'assistant'}
              {@const toolEvents = eventsFor(message.id)}
              {@const toolActions = actionsFor(message.id)}
              {#if toolEvents.length}<details class="chat-tools"><summary>{toolEvents.length} tool {toolEvents.length === 1 ? 'step' : 'steps'}</summary>{#each toolEvents as event}<div class="chat-tool-event"><strong>{event.tool.replaceAll('_', ' ')}</strong><pre>{JSON.stringify(event.arguments, null, 2)}</pre></div>{/each}</details>{/if}
              {#each toolActions as action}<div class="chat-action"><div><strong>{action.tool.replaceAll('_', ' ')}</strong><span class:pending={action.status === 'pending'} title={action.decided_at ? new Date(action.decided_at * 1000).toLocaleString() : undefined}>{action.status === 'pending' ? 'pending approval' : action.decision === 'auto' ? 'auto applied' : action.decision === 'approved' ? 'approved by you' : action.decision === 'rejected' ? 'rejected by you' : action.status}</span></div><pre>{JSON.stringify(action.arguments, null, 2)}</pre>{#if action.error}<small>{action.error}</small>{/if}{#if action.status === 'pending'}<div class="chat-action-buttons"><button class="secondary-button" onclick={() => reviewAction(action, true)}><Check size={13} /> Approve</button><button class="secondary-button danger-button" onclick={() => reviewAction(action, false)}><X size={13} /> Reject</button></div>{/if}</div>{/each}
            {/if}
          </div>
        </div>
      {/each}
      {#if pendingPrompt}<div class="chat-message from-user"><div class="chat-avatar">You</div><div class="chat-message-content"><div class="chat-message-meta"><strong>You</strong></div><p>{pendingPrompt}</p></div></div><div class="chat-message"><div class="chat-avatar">e</div><div class="chat-message-content chat-thinking">Thinking and checking your data…</div></div>{/if}
    </div>
    {#if error}<div class="error-box chat-error" role="alert">{error}</div>{/if}
    <div class="chat-composer">
      {#if config.provider === 'openrouter'}<label class="cloud-consent"><input type="checkbox" bind:checked={allowCloud} /> Share this conversation and any records or imported files read for this turn with OpenRouter.</label>{/if}
      <div class="chat-input-row"><textarea aria-label="Message assistant" bind:value={prompt} placeholder={showArchived ? 'Restore this chat to continue.' : 'Ask a question or describe a change…'} disabled={showArchived || busy} onkeydown={(event) => { if (event.key === 'Enter' && !event.shiftKey) { event.preventDefault(); void send(); } }}></textarea><button class="primary-button" aria-label="Send message" disabled={!prompt.trim() || busy || showArchived || !config.model || (config.provider === 'openrouter' && !allowCloud)} onclick={send}><Send size={16} /></button></div>
      <small>{config.trust_mode === 'delegated' && unrestricted.length ? `Unrestricted writes: ${unrestricted.join(', ')}. Other changes require approval.` : 'Every database change requires your approval.'}</small>
    </div>
  </section>

  <aside class="chat-context"><div class="section-kicker">Database access</div><h3>{recordCount} records available</h3><p>The agent can read records and imported files through bounded tools. It cannot send mail or permanently delete records.</p><div class="permission-summary"><strong>Pending approval</strong><span>{pendingActions.length}</span><small>Changes remain here until you decide.</small></div>
    {#each pendingActions as action (action.id)}<div class="pending-action"><strong>{action.tool.replaceAll('_', ' ')}</strong><small>{action.target_types.map(typeLabel).join(', ') || 'New entity type'}</small><pre>{JSON.stringify(action.arguments, null, 2)}</pre>{#if action.status === 'pending'}<div><button class="secondary-button" onclick={() => reviewAction(action, true)}>Approve</button><button class="secondary-button danger-button" onclick={() => reviewAction(action, false)}>Reject</button></div>{/if}</div>{/each}
  </aside>
</div>
