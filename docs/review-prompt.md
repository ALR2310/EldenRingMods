# Review prompt for AI-written changes

Run this after an AI assistant finishes a task, before committing.

```text
Review the code you just implemented as a senior developer.

Focus on:
1. Removing unnecessary abstractions and complexity.
2. Making the implementation concise and natural.
3. Following existing project conventions (see AGENTS.md and CONTRIBUTING.md).
4. Removing redundant comments and documentation.
5. Avoiding unnecessary defensive programming, except at the game boundary.
6. Ensuring the code is maintainable and easy to understand.

Do not refactor unrelated code.
Do not change behavior.
Prefer straightforward solutions over clever ones.

Apply improvements directly, then run cargo fmt, clippy and the relevant tests.
```
