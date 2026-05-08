## Version Control

This project uses **jj (Jujutsu)** instead of git for version control.

### Committing changes

Always prefer `jjit commit` over manual `jj describe` + `jj new`:

```bash
jjit commit
```

This automatically summarizes the working copy changes using LLM and creates a commit.

Useful flags:
- `--no-thinking` — hide the LLM thinking process
- `--show-prompt` — debug the prompt sent to LLM

### Viewing history

```bash
jj log
```

### Checking out revisions

You can use native jj commands:

```bash
jj checkout <revision>
# or
jj co <revision>
```

Or use the AI-powered `jjit goto` to find and checkout by description:

```bash
jjit goto
```

### Other common jj commands

```bash
jj status           # Show working copy status
jj diff             # Show changes in working copy
jj abandon <rev>    # Abandon a revision
jj squash           # Squash working copy into parent
jj split            # Split a revision into two
```

Note: jj automatically syncs with the underlying git repo, so git-compatible operations work seamlessly.

---

## Commit Rhythm

- Commit early and often
- Use `jjit commit` for all commits
- Each logical unit of work gets its own commit
