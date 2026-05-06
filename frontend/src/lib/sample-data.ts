import { createEntity } from '$lib/api/entities';

export async function createSampleData(): Promise<void> {
	const university = await createEntity(
		'university',
		{
			tags: ['eduport-type/university', 'sample'],
			name: 'Sample University',
			country: 'United States',
			city: 'Boston',
			website: 'https://example.edu/'
		},
		'A small sample university record.'
	);
	const program = await createEntity(
		'program',
		{
			tags: ['eduport-type/program', 'sample'],
			name: 'Sample AI Program',
			level: 'masters',
			department: 'Computer Science',
			deadline: '2026-12-15',
			university: `[[${university.file_id}]]`
		},
		'Use this sample to test deadlines, links, and search.'
	);
	await createEntity(
		'application',
		{
			tags: ['eduport-type/application', 'sample'],
			name: 'Sample Application',
			program: `[[${program.file_id}]]`,
			status: 'planning',
			internal_deadline: '2026-11-15'
		},
		'- [ ] Draft statement by 2026-10-15\n- [ ] Submit by 2026-11-15\n'
	);
}
