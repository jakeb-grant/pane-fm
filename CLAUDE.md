# AGENTS.md

Baseline instructions for AI coding agents working in this project.

## Python

### Toolchain

Tools from [Astral](https://astral.sh/) should be run via `uvx` (ephemeral tool execution) unless installed in the project.

| Tool   | Purpose              | Command             |
| ------ | -------------------- | ------------------- |
| uv     | Package/project mgmt | `uv add`, `uv run`  |
| ruff   | Linting & formatting | `uvx ruff check`, `uvx ruff format` |
| ty     | Type checking        | `uvx ty check`      |
| pytest | Testing              | `uv run pytest`     |

### Project Setup

- Python version: **>= 3.14**
- Dependencies and tool config live in `pyproject.toml`
- Use **pydantic** for data models — do not use dataclasses

Initialize projects with `uv init`:

```bash
uv init myproject                    # Default app (main.py, no build system)
uv init --lib myproject              # Library (src/ layout, py.typed, build system)
uv init --package myproject          # Packaged app (src/ layout, entry points)
uv init --bare myproject             # Minimal (pyproject.toml only)
uv init --python 3.14 myproject      # Pin Python version
```

Default app structure:
```
myproject/
├── .python-version
├── README.md
├── main.py
└── pyproject.toml
```

### Configuration

```toml
# pyproject.toml

[tool.ruff.lint]
select = [
    "E",     # pycodestyle errors
    "F",     # Pyflakes
    "I",     # isort
    "UP",    # pyupgrade
    "B",     # flake8-bugbear
    "SIM",   # flake8-simplify
    "RUF",   # Ruff-specific rules
    "PT",    # flake8-pytest-style
    "PERF",  # Perflint
    "S",     # flake8-bandit (security)
    "N",     # pep8-naming
    "C4",    # flake8-comprehensions
    "ASYNC", # flake8-async
    "PTH",   # flake8-use-pathlib
    "TRY",   # tryceratops
]

[tool.ty.environment]
python-version = "3.14"

[tool.ty.rules]
all = "warn"
```

### Documentation

- uv: https://docs.astral.sh/uv/llms.txt
- ruff: https://docs.astral.sh/ruff/llms.txt
- ty: https://docs.astral.sh/ty/llms.txt
- pytest: https://docs.pytest.org/en/stable/how-to/index.html
- pydantic: https://docs.pydantic.dev/latest/llms.txt

## Rust

### Toolchain

Use `cargo` and its built-in tools.

| Tool          | Purpose    | Command        |
| ------------- | ---------- | -------------- |
| cargo build   | Build      | `cargo build`  |
| cargo test    | Testing    | `cargo test`   |
| cargo clippy  | Linting    | `cargo clippy` |
| cargo fmt     | Formatting | `cargo fmt`    |

### Project Setup

- Rust edition: **2024**
- Initialize projects with `cargo init`

## Web (SvelteKit)

### Toolchain

Use **bun** as the runtime, package manager, and test runner. Do not use npm/yarn/pnpm.

| Tool       | Purpose              | Command              |
| ---------- | -------------------- | -------------------- |
| bun        | Runtime & pkg mgmt   | `bun install`, `bun add` |
| bunx       | Ephemeral tool exec  | `bunx sv create`     |
| bun run    | Script runner        | `bun run dev`, `bun run build` |
| bun test   | Testing              | `bun test`           |
| biome      | Linting & formatting | `bunx biome check` (review first), `bunx biome check --write` |

### Project Setup

Initialize SvelteKit projects with bun:

```bash
bunx sv create my-app
cd my-app
bun install
bun add -D svelte-adapter-bun
bun run dev
```

After scaffolding, install dev dependencies:

```bash
bun add -D @biomejs/biome @testing-library/svelte @happy-dom/global-registrator
bunx biome init
```

Replace the generated `biome.json` with:

```json
{
  "$schema": "https://biomejs.dev/schemas/latest/schema.json",
  "files": {
    "includes": ["**/*.{js,ts,jsx,tsx}", "**/*.json", "**/*.css", "**/*.svelte"]
  },
  "vcs": {
    "enabled": true,
    "clientKind": "git",
    "useIgnoreFile": true
  },
  "formatter": {
    "enabled": true,
    "indentStyle": "tab"
  },
  "linter": {
    "rules": {
      "recommended": true,
      "correctness": {
        "recommended": true,
        "noNodejsModules": "off"
      },
      "security": "error",
      "suspicious": "warn"
    }
  },
  "javascript": {
    "formatter": {
      "quoteStyle": "double"
    }
  },
  "assist": {
    "enabled": true,
    "actions": {
      "source": {
        "organizeImports": "on"
      }
    }
  },
  "overrides": [
    {
      "includes": ["**/*.svelte"],
      "linter": {
        "rules": {
          "correctness": {
            "noUnusedVariables": "off",
            "noUnusedImports": "off"
          },
          "suspicious": {
            "noUnassignedVariables": "off"
          },
          "style": {
            "useConst": "off"
          }
        }
      }
    }
  ]
}
```

Create a Svelte loader plugin for bun test (see https://bun.com/docs/guides/test/svelte-test) and configure `bunfig.toml`:

```toml
[test]
preload = ["./svelte-loader.ts"]
```

Update `svelte.config.js` to use the bun adapter:

```js
import adapter from "svelte-adapter-bun";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
  },
};

export default config;
```

Default project structure:
```
my-app/
├ src/
│ ├ lib/
│ │ ├ server/
│ │ └ [lib files]
│ ├ routes/
│ │ └ [routes]
│ ├ app.html
│ ├ hooks.client.ts
│ └ hooks.server.ts
├ static/
├ tests/
├ package.json
├ svelte.config.js
├ tsconfig.json
└ vite.config.ts
```

### Documentation

- Bun: https://bun.sh/docs/llms.txt
- Biome: https://biomejs.dev/guides/getting-started/
- Svelte: Use the **Svelte MCP server** — see [MCP Servers](#mcp-servers) below

## MCP Servers

The following MCP servers should be available to agents. Check for them before starting work. Attempt to install if missing, but ask the user to install manually if unable.

- **Svelte** (`@sveltejs/mcp`) — Svelte/SvelteKit documentation and assistance
  ```bash
  # Claude Code
  claude mcp add -t stdio -s user svelte -- bunx -y @sveltejs/mcp
  ```
  ```json
  {
    "mcpServers": {
      "svelte": {
        "command": "bunx",
        "args": ["-y", "@sveltejs/mcp"]
      }
    }
  }
  ```
- **Context7** — provides up-to-date documentation for any library. Use it to look up APIs and examples rather than guessing.
  ```bash
  # Claude Code
  claude mcp add -t stdio -s user context7 -- bunx -y @context7/mcp
  ```
  ```json
  {
    "mcpServers": {
      "context7": {
        "command": "bunx",
        "args": ["-y", "@context7/mcp"]
      }
    }
  }
  ```

## General Guidelines

- Prefer simple, minimal solutions over abstractions.
- Do not add features, refactoring, or "improvements" beyond what was asked.
- Run linting and type checking before considering a task complete.
- Run tests after making changes to verify nothing is broken.
- Do not add `Co-Authored-By` or other self-attribution to git commits.
