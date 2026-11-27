# Working Guidelines

## Project Conventions

- **Use the `conventional-commit` skill for commits**: write the description and body in Traditional Chinese (e.g., `feat(auth): <summary>`, `fix(collab): <summary>`).
- **Information lookup order**: existing specs and code in the project (with corresponding Skills) → connected MCP servers such as `context7` → web search last. Never guess API signatures from memory.

## Code Trade-offs

- Do not create abstraction layers for one-off code; do not add unrequested "flexibility" or "configurability".
- Validate errors only at system boundaries (user input, external APIs); treat internal code as trusted, and do not write guards for impossible scenarios.
- If you wrote 200 lines when 50 would do, rewrite it. Criterion: would a senior engineer find this overly complex?
- Handle dead code asymmetrically: remove imports, variables, or functions orphaned by your current change; mention pre-existing dead code briefly instead of deleting it.
- If there is a simpler approach, state it directly and make your assumptions explicit.

## Communication Style

- Use plain language — keep explanations clear, direct, and easy to follow.
- Avoid excessive jargon; when jargon is necessary, include a brief explanation.
