-- Skill grouping: optional category from SKILL.md frontmatter
-- (conceptually office/docs/dev/data/other; stored as free-form text).
ALTER TABLE skills ADD COLUMN category TEXT;
