import type { Entry, FilterRule, ViewConfig } from './types';

export type RelationIndex = Record<string, Record<string, string[]>>;

export function cellValue(entry: Entry, field: string, relations: RelationIndex = {}): unknown {
  if (field.startsWith('relation:')) return relations[entry.id]?.[field] ?? [];
  if (field === 'title') return entry.title;
  if (field === 'body') return entry.body;
  if (field === 'tags') return entry.tags;
  if (field === 'updated_at') return new Date(entry.updated_at * 1000).toISOString().slice(0, 10);
  return entry.values[field] ?? null;
}

export function displayValue(value: unknown): string {
  if (value === null || value === undefined || value === '') return '—';
  if (Array.isArray(value)) return value.join(', ') || '—';
  if (typeof value === 'boolean') return value ? 'Yes' : 'No';
  return String(value).replaceAll('_', ' ');
}

function compareValues(left: unknown, right: unknown): number {
  if (left === null || left === undefined || left === '') return 1;
  if (right === null || right === undefined || right === '') return -1;
  if (typeof left === 'number' && typeof right === 'number') return left - right;
  return String(left).localeCompare(String(right), undefined, { numeric: true, sensitivity: 'base' });
}

function matches(entry: Entry, rule: FilterRule, relations: RelationIndex): boolean {
  const actual = cellValue(entry, rule.field, relations);
  if (rule.operator === 'is_empty') return actual === null || actual === undefined || actual === '' || (Array.isArray(actual) && !actual.length);
  const left = Array.isArray(actual) ? actual.join(', ') : String(actual ?? '');
  const right = rule.value.trim();
  if (rule.operator === 'contains') return left.toLowerCase().includes(right.toLowerCase());
  if (rule.operator === 'equals') return left.toLowerCase() === right.toLowerCase();
  if (rule.operator === 'not_equals') return left.toLowerCase() !== right.toLowerCase();
  const leftNumber = Number(left);
  const rightNumber = Number(right);
  const comparison = Number.isFinite(leftNumber) && Number.isFinite(rightNumber) && left && right
    ? leftNumber - rightNumber
    : left.localeCompare(right, undefined, { numeric: true });
  return rule.operator === 'greater_than' ? comparison > 0 : comparison < 0;
}

export function applyTableView(records: Entry[], config: ViewConfig, search: string, relations: RelationIndex = {}): Array<{ key: string; rows: Entry[] }> {
  const query = search.trim().toLowerCase();
  const filtered = records.filter((entry) =>
    config.filters.every((rule) => matches(entry, rule, relations)) &&
    (!query || (entry.title + ' ' + entry.body + ' ' + JSON.stringify(entry.values)).toLowerCase().includes(query))
  );
  filtered.sort((a, b) => {
    for (const rule of config.sorts) {
      const comparison = compareValues(cellValue(a, rule.field, relations), cellValue(b, rule.field, relations));
      if (comparison) return rule.direction === 'desc' ? -comparison : comparison;
    }
    return b.updated_at - a.updated_at || a.title.localeCompare(b.title);
  });
  if (!config.group_by) return [{ key: '', rows: filtered }];
  const groups = new Map<string, Entry[]>();
  for (const entry of filtered) {
    const value = displayValue(cellValue(entry, config.group_by, relations));
    if (!groups.has(value)) groups.set(value, []);
    groups.get(value)?.push(entry);
  }
  return [...groups].map(([key, rows]) => ({ key, rows }));
}
