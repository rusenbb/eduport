import { describe, expect, it } from 'vitest';
import { groupableFrom, groupItems } from '../../src/lib/utils/viewPipeline';
import type { Property, SingleSelectProperty } from '../../src/lib/types/schema';
import type { EntityListItem } from '../../src/lib/types';

const item = (id: string): EntityListItem => ({
	file_id: id,
	entity_type: 'university',
	name: id,
	path: `${id}.md`,
	mtime_ns: 0
});

const detailWith = (entity: Record<string, unknown>) =>
	({ entity }) as { entity: Record<string, unknown> };

const countrySelect: SingleSelectProperty = {
	key: 'country',
	name: 'Country',
	type: 'single-select',
	options: [
		{ value: 'usa', label: 'USA', color: 'blue' },
		{ value: 'japan', label: 'Japan', color: 'red' }
	]
};

describe('viewPipeline.groupItems', () => {
	it('preserves option order for single-select even when buckets are empty', () => {
		const items = [item('a'), item('b')];
		const details = {
			a: detailWith({ country: 'japan' }),
			b: detailWith({ country: 'japan' })
		};
		const buckets = groupItems(items, details, { property: countrySelect });
		expect(buckets.map((b) => b.value)).toEqual(['usa', 'japan']);
		expect(buckets[0].items).toHaveLength(0);
		expect(buckets[1].items).toHaveLength(2);
	});

	it('routes items with no value to an Uncategorized bucket', () => {
		const items = [item('a'), item('b')];
		const details = { a: detailWith({}), b: detailWith({ country: 'usa' }) };
		const buckets = groupItems(items, details, { property: countrySelect });
		const last = buckets[buckets.length - 1];
		expect(last.value).toBe('__uncategorized__');
		expect(last.items.map((i) => i.file_id)).toEqual(['a']);
	});

	it('multi-select places an item in every matching bucket', () => {
		const language: Property = {
			key: 'language',
			name: 'Language',
			type: 'multi-select',
			options: [
				{ value: 'english', label: 'English', color: 'blue' },
				{ value: 'german', label: 'German', color: 'yellow' }
			]
		};
		const items = [item('p1')];
		const details = { p1: detailWith({ language: ['english', 'german'] }) };
		const buckets = groupItems(items, details, { property: language });
		expect(buckets.find((b) => b.value === 'english')!.items).toHaveLength(1);
		expect(buckets.find((b) => b.value === 'german')!.items).toHaveLength(1);
	});

	it('date grouping defaults to month buckets', () => {
		const deadline: Property = {
			key: 'deadline',
			name: 'Deadline',
			type: 'date'
		};
		const items = [item('a'), item('b'), item('c')];
		const details = {
			a: detailWith({ deadline: '2026-05-01' }),
			b: detailWith({ deadline: '2026-05-15' }),
			c: detailWith({ deadline: '2026-06-02' })
		};
		const buckets = groupItems(items, details, { property: deadline });
		expect(buckets.map((b) => b.value)).toEqual(['2026-05', '2026-06']);
		expect(buckets[0].items).toHaveLength(2);
	});

	it('number grouping uses the configured step', () => {
		const tuition: Property = {
			key: 'tuition',
			name: 'Tuition',
			type: 'number'
		};
		const items = [item('a'), item('b'), item('c')];
		const details = {
			a: detailWith({ tuition: 4000 }),
			b: detailWith({ tuition: 8000 }),
			c: detailWith({ tuition: 12500 })
		};
		const buckets = groupItems(items, details, {
			property: tuition,
			numberStep: 5000
		});
		expect(buckets.map((b) => b.value)).toEqual(['[0,5000)', '[5000,10000)', '[10000,15000)']);
	});

	it('text grouping buckets by uppercased first letter', () => {
		const name: Property = {
			key: 'name',
			name: 'Name',
			type: 'text'
		};
		const items = [item('alpha'), item('antelope'), item('beta')];
		const details = {
			alpha: detailWith({ name: 'alpha' }),
			antelope: detailWith({ name: 'antelope' }),
			beta: detailWith({ name: 'Beta' })
		};
		const buckets = groupItems(items, details, { property: name });
		expect(buckets.map((b) => b.value)).toEqual(['A', 'B']);
	});
});

describe('viewPipeline.groupableFrom', () => {
	it('excludes relation and resource-list properties', () => {
		const props: Property[] = [
			{ key: 'k1', name: 'k1', type: 'single-select', options: [] },
			{ key: 'k2', name: 'k2', type: 'date' },
			{ key: 'k3', name: 'k3', type: 'number' },
			{ key: 'k4', name: 'k4', type: 'text' },
			{ key: 'k5', name: 'k5', type: 'multi-select', options: [] },
			{ key: 'k6', name: 'k6', type: 'relation' }
		];
		const out = groupableFrom(props).map((p) => p.key);
		expect(out).toEqual(['k1', 'k2', 'k3', 'k4', 'k5']);
	});
});
