# Rules

Instructions scoped to file paths. A rule with `paths:` frontmatter loads only when a matching
file is read, so it costs nothing while working elsewhere:

```markdown
---
paths:
  - "src/api/**/*.ts"
---

# API

- Validate every request body at the boundary.
```

A rule with no `paths:` loads every session, same as `CLAUDE.md`. Prefer scoping.
