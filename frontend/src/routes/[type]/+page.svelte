<!--
	The entire [type]/... workspace is rendered by +layout.svelte so
	the component instance survives navigation to /[type]/[fileId]
	and back — selection does not remount the workspace and so does
	not refetch the list. This file exists only to make /[type] a
	valid route; the layout draws the actual UI.
-->
