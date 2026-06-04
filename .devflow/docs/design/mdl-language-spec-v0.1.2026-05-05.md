---
title: MDL Language Specification — Design Plan
status: draft
created: 2026-05-05
scope: Language syntax, compilation model, scoping rules
decisions:
  - YAML frontmatter for variables
  - Single-brace {var} interpolation
  - "@"-prefixed directives with colon + @end blocks
  - .mdl file extension
  - Markdown/text output only (no structured JSON in v0.1)
  - "@if + @else" supported
  - Plain "@end" for all block terminators (innermost matching)
  - No built-in functions in v0.1
  - Rust compiler
---

# MDL Language Specification — Design Plan

## 1. What is MDL?

MDL (Markdown Definition Language) is a domain-specific language for composing, reusing, and compiling LLM prompts.

- **Input**: `.mdl` files (Markdown-native syntax with lightweight directives)
- **Output**: Compiled Markdown/plain text strings
- **Compiler**: Rust
- **Audience**: Prompt engineers, AI developers

## 2. Design Principles

1. Looks like Markdown — not code
2. Minimal new syntax — leverage existing conventions (YAML frontmatter, `@` directives)
3. Composable — imports, functions, modules
4. Deterministic — same input always produces same output
5. Fail fast — clear errors with file:line:col, no partial output

## 3. File Format

- Extension: `.mdl`
- Encoding: UTF-8
- Structure: optional frontmatter → directives/content (order-independent for directives)

## 4. Syntax

### 4.1 Variables (YAML Frontmatter)

```mdl
---
name: Alice
items: [apple, banana]
premium: true
count: 3
---
```

**Rules:**
- Standard YAML between `---` fences at file start
- Types: string, number, boolean, array
- Runtime vars (CLI `--vars vars.json`) override frontmatter values

### 4.2 Interpolation

```mdl
Hello {name}!
```

**Rules:**
- Single braces: `{identifier}`
- Valid interpolation: content must be a valid identifier (`[a-zA-Z_][a-zA-Z0-9_]*`) or function call
- Escaping: `\{` produces literal `{`
- Inside fenced code blocks (triple backtick): no interpolation (raw passthrough)
- Undefined variable → compilation error (not silent empty string)

### 4.3 Conditionals

```mdl
@if premium:
Thanks for being premium!
@end

@if premium:
Premium content here.
@else:
Free tier content here.
@end
```

**Rules:**
- Condition is a variable name (truthy check) or comparison (future)
- Falsy values: `false`, `null`, empty string `""`, empty array `[]`, `0`
- Nesting: plain `@end`, resolved by innermost matching
- No `@elseif` in v0.1 (use nested `@if` or restructure)

### 4.4 Loops

```mdl
@for item in items:
- {item}
@end
```

**Rules:**
- Iterates over arrays only
- Loop variable (`item`) is block-scoped to the `@for...@end`
- Loop variable shadows any outer variable with same name
- Iterating over non-array → compilation error

### 4.5 Functions

Definition:
```mdl
@define greet(name):
Hello {name}, welcome!
@end
```

Invocation:
```mdl
{greet("Alice")}
```

**Rules:**
- Functions are pure text templates (no side effects)
- Arguments are positional
- Functions can call other functions
- Recursive calls → compilation error (no recursion in v0.1)
- Function body has its own scope; params shadow outer vars
- No default arguments in v0.1

### 4.6 Imports

```mdl
@import "./base.mdl"
@import "./footer.mdl" as footer
```

**Rules:**
- Relative paths only (no bare module names in v0.1)
- `as alias` namespaces all exports: `{footer.greet("hi")}`
- Without alias: exports merge into current scope (name collision → error)
- Circular imports → compilation error
- Import resolution is recursive (imports can import)

### 4.7 Exports

```mdl
@export greet
@export prompt
```

**Rules:**
- Only exported symbols are visible to importers
- If no `@export` directives: everything is exported (default-public)
- Exportable: functions, the prompt body (as `prompt`)

### 4.8 Includes

```mdl
@include footer
```

**Rules:**
- Renders an imported module's compiled prompt body inline
- Equivalent to calling the module's default prompt output
- Module must be imported first via `@import`
- `@include footer.section` for named exports (future)

## 5. Compilation Model

| Phase | Description | Errors |
|-------|-------------|--------|
| 1. Parse | Tokenize → AST (frontmatter, directives, text) | Syntax errors (unexpected token, unclosed block) |
| 2. Resolve | Recursively load imports, build dependency graph | File not found, circular import |
| 3. Validate | Check all references, types, arity | Undefined var/function, type mismatch, wrong arg count |
| 4. Evaluate | Execute directives (expand loops, resolve conditions, call functions) | Runtime: iterate non-array, infinite recursion guard |
| 5. Render | Flatten evaluated tree → final Markdown string | (none expected) |

**Error format:**
```
error[E001]: undefined variable 'username'
  --> src/welcome.mdl:12:8
   |
12 | Hello {username}!
   |        ^^^^^^^^ not defined in frontmatter or imports
```

## 6. Scoping Rules

1. **File scope** — frontmatter vars visible everywhere in that file
2. **Runtime override** — `--vars` JSON values override frontmatter
3. **Block scope** — `@for` loop vars scoped to their `@for...@end`
4. **Function scope** — params scoped to function body
5. **Import scope** — namespaced (aliased) or merged (unaliased), never implicit
6. **Shadowing** — inner scope wins, no warning (intentional override)

## 7. CLI Interface

```bash
mdl build input.mdl -o output.md          # compile to file
mdl build input.mdl                        # compile to stdout
mdl build input.mdl --vars vars.json       # with runtime variables
mdl check input.mdl                        # validate without rendering
mdl fmt input.mdl                          # format (future)
```

## 8. What's NOT in v0.1

- Structured JSON output (chat message arrays)
- Dot notation for objects (`{user.name}`)
- TypeScript/JS integration or runtime bindings
- Built-in functions (upper, lower, join, etc.)
- Macros, async functions, streaming
- Object type in variables
- `@elseif` chains
- Recursion
- IDE/LSP support
- Source maps

## 9. Complete Example

```mdl
---
name: Alice
items: [apple, banana]
premium: true
---

@import "./footer.mdl" as footer

@define list(items):
@for item in items:
- {item}
@end
@end

Hello {name}!

Your items:
{list(items)}

@if premium:
Thanks for being premium!
@else:
Upgrade for premium features.
@end

@include footer
```

**Output:**
```markdown
Hello Alice!

Your items:
- apple
- banana

Thanks for being premium!

[footer content]
```

## 10. Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Single-brace conflicts with JSON in prose | Only interpolate valid identifiers; raw inside code blocks |
| YAML frontmatter parsing edge cases | Use a battle-tested YAML parser (serde_yaml in Rust) |
| Ambiguous `@end` in deep nesting | Innermost-match rule; linter can warn on 3+ depth |
| Large prompt libraries — slow compilation | Cache compiled modules by content hash |
| Users confuse functions vs includes | Clear docs: functions take args and return text; includes inline a module's output |

## 11. Open Design Questions (for later versions)

- Should `@elseif` be added in v0.2?
- Object/map variables with dot access — when?
- Built-in function stdlib — what's the minimal useful set?
- Should imports support URL paths (remote modules)?
- Template inheritance (base/child) pattern?
