# Skill Bake-off Candidates

Model: `deepseek-v4-flash`  
Staging: `eval/skill-bakeoff/skills-candidates/`  
Isolation: `skills.extra_dirs` → candidates root (prefixed ids, no merge into `skills-starter` until approved)

## Baselines (existing anyCode)

| id | source | why |
|---|---|---|
| anycode-xlsx | skills-starter / ~/.anycode/skills | office control |
| deep-research | skills-starter | research control |
| verify-discover | skills-starter | verify control |
| mindmap | skills-starter | structured markdown control |

## Candidates (18)

| # | bakeoff id | upstream | license | domain |
|---|---|---|---|---|
| 1 | bakeoff-frontend-design | anthropics/skills/frontend-design | Apache-2.0 | design |
| 2 | bakeoff-webapp-testing | anthropics/skills/webapp-testing | Apache-2.0 | qa |
| 3 | bakeoff-doc-coauthoring | anthropics/skills/doc-coauthoring | Apache-2.0 (repo) | docs |
| 4 | bakeoff-internal-comms | anthropics/skills/internal-comms | Apache-2.0 | writing |
| 5 | bakeoff-mcp-builder | anthropics/skills/mcp-builder | Apache-2.0 | platform |
| 6 | bakeoff-canvas-design | anthropics/skills/canvas-design | Apache-2.0 | visual |
| 7 | bakeoff-algorithmic-art | anthropics/skills/algorithmic-art | Apache-2.0 | creative-code |
| 8 | bakeoff-theme-factory | anthropics/skills/theme-factory | Apache-2.0 | theming |
| 9 | bakeoff-web-artifacts-builder | anthropics/skills/web-artifacts-builder | Apache-2.0 | frontend |
| 10 | bakeoff-slack-gif-creator | anthropics/skills/slack-gif-creator | Apache-2.0 | media |
| 11 | bakeoff-skill-creator | anthropics/skills/skill-creator | Apache-2.0 | meta |
| 12 | bakeoff-claude-api | anthropics/skills/claude-api | Apache-2.0 | api |
| 13 | bakeoff-vercel-react-best-practices | vercel-labs/agent-skills | MIT | react |
| 14 | bakeoff-vercel-web-design-guidelines | vercel-labs/agent-skills | MIT | a11y/web |
| 15 | bakeoff-vercel-composition-patterns | vercel-labs/agent-skills | MIT | react |
| 16 | bakeoff-vercel-writing-guidelines | vercel-labs/agent-skills | MIT | writing |
| 17 | bakeoff-design-taste-frontend | ~/.agents/skills/design-taste-frontend | local copy | design |
| 18 | bakeoff-find-skills | ~/.agents/skills/find-skills | local copy | discovery |

**Excluded on purpose:** anthropic pdf/docx/pptx/xlsx (overlap with anycode office four).

## Decision gate

After `SCORECARD.md` + artifacts review: user picks which ids become built-in (if any). No auto-merge.
