# RimV Web Design

This folder defines web-specific UI and implementation guidance for RimV.

Use together with:

- `.aiassistance/design/RIMV_DESIGN_SYSTEM.md`
- `.aiassistance/agents/frontend-developer.md`
- `.aiassistance/agents/ui-ux-designer.md`

Current frontend stack:

- TypeScript
- Vite 6
- Tailwind CSS 3
- PostCSS
- Autoprefixer
- Node built-in test runner

Do not assume React, Vue, Svelte, Angular, or another framework unless the project adopts one.

## Principle

RimV Web should preserve the shared product identity while behaving like a high-quality browser application.

The web interface must remain:

- responsive;
- keyboard accessible;
- semantic;
- performant;
- readable at zoom;
- usable without framework-specific assumptions.
