# AGENTS.md

Instructions for coding agents working in this repository. Read this file in full before making changes. When a rule here conflicts with an established convention in the existing codebase, **the codebase wins**. Match what's already there and flag the inconsistency instead of "fixing" it unasked.

---

## 0. Prime Directives

1. **Understand before you change.** Read the surrounding code, its callers, and its tests before editing anything.
2. **Make it work, make it right, make it fast**, in that order.
3. **Leave the code better than you found it**, but only within the scope of your task. Don't refactor the neighbourhood.
4. **Small, reversible steps.** Prefer many small, verifiable changes over one large rewrite.
5. **Never guess silently.** If requirements are ambiguous, ask. If you must assume, state the assumption explicitly in your summary.
6. **Verify everything.** Code that hasn't been run, built, or tested is not done.

---

## 1. Code Readability

Code is read far more often than it is written. Optimise for the next reader.

### Naming
- Names reveal intent: `elapsedDays`, not `d`; `isEligibleForRefund`, not `check`.
- Functions are verbs (`calculateTotal`, `sendInvoice`). Classes and data are nouns (`Invoice`, `PaymentGateway`).
- Booleans read as questions: `isActive`, `hasPermission`, `shouldRetry`.
- Avoid abbreviations unless universal in the domain (`id`, `url`, `http` are fine; `usrMgr` is not).
- Avoid noise words: `data`, `info`, `manager`, `helper`, `util` usually signal an unclear responsibility.
- Name length scales with scope: `i` is fine in a three-line loop; a module-level constant deserves a full name.
- Use the same word for the same concept everywhere. Don't mix `fetch`, `get`, and `retrieve` for one operation.

### Functions
- **Do one thing.** If you need "and" to describe a function, split it.
- Keep functions short enough to understand at a glance (as a guideline, under ~30 lines).
- Few parameters: 0–3 is ideal. More than that usually means you need a parameter object.
- Avoid boolean flag parameters (`render(true)`); they mean the function does two things. Split it or use a named option.
- One level of abstraction per function. Don't mix high-level orchestration with byte-level fiddling.
- Prefer pure functions: same input, same output, no hidden side effects.
- Return early with guard clauses instead of deep nesting.

```python
# Avoid
def process(order):
    if order:
        if order.is_paid:
            if order.items:
                ship(order)

# Prefer
def process(order):
    if not order or not order.is_paid or not order.items:
        return
    ship(order)
```

### Structure & Formatting
- Follow the project's formatter and linter. Never hand-format against them.
- Keep nesting shallow (max ~3 levels).
- Group related code; separate concepts with blank lines.
- Declare variables close to where they're used.
- Replace magic numbers and strings with named constants.
- Keep files focused. A file that does many unrelated things should be split.

### Comments
- Code explains **what** and **how**; comments explain **why**.
- Good comments: intent, non-obvious decisions, trade-offs, warnings, links to specs or tickets.
- Bad comments: restating the code, commented-out code, outdated notes, journal entries.
- If you need a comment to explain *what* a block does, extract it into a well-named function instead.
- Update or delete comments when you change the code they describe. A wrong comment is worse than none.
- Public APIs get doc comments describing parameters, return values, errors, and side effects.

---

## 2. Core Principles

### KISS: Keep It Simple
The simplest solution that fully solves the problem is the best one. Clever code is a liability. If a junior developer can't follow it, rewrite it.

### DRY: Don't Repeat Yourself
Every piece of *knowledge* should have a single authoritative source. But:
- Duplication of *code* is not always duplication of *knowledge*. Two blocks that look alike but change for different reasons should stay separate.
- **Rule of three:** tolerate duplication twice; abstract on the third occurrence.
- A wrong abstraction is more expensive than duplication.

### YAGNI: You Aren't Gonna Need It
Build what's required now. No speculative features, no "just in case" configuration options, no unused extension points.

### Separation of Concerns
Keep distinct responsibilities in distinct places: presentation, business logic, data access, and infrastructure should not be tangled together.

### Principle of Least Astonishment
Code should behave the way its name and signature suggest. A `getUser()` must not modify state or send emails.

### Composition Over Inheritance
Prefer assembling behaviour from small collaborating objects over deep class hierarchies. Inherit only for genuine "is-a" relationships, and keep hierarchies shallow.

### Law of Demeter
Talk to your direct collaborators, not their internals. `order.getCustomer().getAddress().getCity()` couples you to three classes; prefer `order.shippingCity()`.

### Fail Fast
Validate inputs at boundaries and surface errors immediately, close to their cause.

---

## 3. SOLID

| Principle | Rule | Smell when violated |
|---|---|---|
| **S**ingle Responsibility | A module has one reason to change. | A class touched by every unrelated feature. |
| **O**pen/Closed | Open for extension, closed for modification. | Growing `switch`/`if` chains on a type field. |
| **L**iskov Substitution | Subtypes must be usable wherever the base type is. | Overrides that throw `NotImplemented` or tighten preconditions. |
| **I**nterface Segregation | Many small, specific interfaces beat one fat one. | Implementers forced to stub methods they don't use. |
| **D**ependency Inversion | Depend on abstractions, not concretions. | Business logic that instantiates databases or HTTP clients directly. |

Apply SOLID with judgement. A 50-line script does not need five interfaces.

---

## 4. Design Patterns

Patterns are a shared vocabulary for solving recurring problems, **not goals in themselves**.

### Rules for using patterns
- Use a pattern only when the problem it solves is actually present.
- Prefer the language's idioms first: a first-class function often replaces Strategy or Command; a module often replaces Singleton.
- Name things after the pattern when you use one (`PaymentStrategy`, `OrderFactory`) so readers recognise it.
- Never introduce a pattern purely for "future flexibility" (see YAGNI).

### When to reach for which

**Creational**
- **Factory / Factory Method:** object creation involves logic or the concrete type is decided at runtime.
- **Builder:** constructing an object needs many optional parameters or steps.
- **Singleton:** almost never. Prefer dependency injection of a single instance. Global state hurts testing.

**Structural**
- **Adapter:** make an external or legacy interface fit the one your code expects.
- **Facade:** give a simple entry point to a complex subsystem.
- **Decorator:** add behaviour (logging, caching, retries) without modifying the wrapped object.
- **Composite:** treat individual items and groups of items uniformly (trees, menus, file systems).

**Behavioral**
- **Strategy:** swap interchangeable algorithms at runtime; replaces type-switching conditionals.
- **Observer / Pub-Sub:** notify many dependents of a change without coupling to them.
- **Command:** represent actions as objects (undo/redo, queues, auditing).
- **State:** an object's behaviour changes substantially based on its internal state.
- **Template Method:** a fixed algorithm skeleton with customisable steps (prefer Strategy if composition works).

**Architectural**
- **Dependency Injection:** pass collaborators in; don't construct them inside.
- **Repository:** isolate data access behind a domain-oriented interface.
- **Layered / Hexagonal (Ports & Adapters):** keep the domain core independent of frameworks, databases, and UI.

### Anti-patterns to avoid
- **God object:** one class that knows and does everything.
- **Spaghetti code:** tangled control flow with no clear structure.
- **Golden hammer:** forcing one familiar pattern or tool onto every problem.
- **Lava flow:** dead code nobody dares remove. Remove it (version control remembers).
- **Premature optimisation** and **premature abstraction.**
- **Primitive obsession:** passing raw strings and ints where a small domain type (`Email`, `Money`) would prevent bugs.
- **Shotgun surgery:** one change requires edits in many unrelated places.

---

## 5. Error Handling

- Handle errors at the level that has enough context to do something meaningful about them.
- **Never swallow exceptions silently.** An empty `catch` block is a bug.
- Catch specific exceptions, not the broadest base type, unless at a top-level boundary.
- Error messages say what happened, with which input, and (if possible) what to do about it.
- Use exceptions for exceptional cases, not for normal control flow.
- Don't return `null` / `None` to signal errors when the language offers better options (exceptions, `Result`/`Option` types).
- Clean up resources deterministically (`finally`, `with`, `using`, `defer`, RAII).
- Validate all external input at system boundaries: user input, files, network, environment variables.

---

## 6. Testing

- **New behaviour gets tests. Bug fixes get a regression test that fails before the fix.**
- Run the existing test suite before and after your change. Don't break unrelated tests.
- Tests follow **Arrange → Act → Assert** and test one behaviour each.
- Test names describe behaviour: `returns_empty_list_when_no_orders_exist`.
- Test behaviour through public interfaces, not implementation details.
- Tests must be deterministic: no reliance on wall-clock time, randomness, network, or test order without control.
- Mock at boundaries (network, filesystem, clock), not your own domain logic.
- Cover edge cases: empty, null, zero, negative, maximum, unicode, concurrent access.
- **Never** delete, skip, or weaken a failing test to make the suite pass. Fix the code, or report the conflict.
- Test code is production code: keep it readable and DRY-ish (but clarity beats DRY in tests).

---

## 7. Security

- **Never** commit secrets, API keys, tokens, or credentials. Use environment variables or a secrets manager.
- Treat all input as untrusted. Validate, sanitise, and encode for the output context.
- Use parameterised queries. Never build SQL, shell commands, or file paths by string concatenation with user input.
- Apply the principle of least privilege for permissions, tokens, and access scopes.
- Don't roll your own cryptography or authentication. Use vetted libraries.
- Don't log sensitive data (passwords, tokens, personal data, full card numbers).
- Prefer secure defaults: deny by default, opt in to access.
- Before adding a dependency, check that it's maintained, widely used, and doesn't have known vulnerabilities.

---

## 8. Performance

- Correctness and clarity first. Optimise only with evidence: **measure, don't guess.**
- Know the complexity of what you write. Avoid accidental O(n²) loops over large collections.
- Watch for N+1 queries, unnecessary I/O inside loops, and repeated expensive computation.
- Cache deliberately, with a clear invalidation strategy.
- When you do optimise for performance, leave a comment explaining why the less obvious code exists, with the measurement that justified it.

---

## 9. Dependencies & Architecture

- Prefer the standard library and existing project dependencies before adding new ones.
- Every new dependency needs a justification in your summary.
- Respect existing module boundaries and layering. Don't let low-level modules import high-level ones.
- Avoid circular dependencies.
- Keep public APIs small and stable. Changing a public interface is a breaking change; call it out.
- Configuration lives in config, not hard-coded in logic.

---

## 10. Version Control

- One logical change per commit. Don't mix refactoring with behaviour changes.
- Commit messages: imperative mood summary line (≤ 72 chars), blank line, then *why* the change was made.
  ```
  Fix rounding error in invoice totals

  Totals were summed as floats, causing off-by-one-cent errors on
  large orders. Switch to integer cents throughout the calculation.
  ```
- Never commit generated files, build output, local config, or secrets unless the project already does so intentionally.
- Never rewrite shared history (force-push to main, rebase published branches) without explicit instruction.

---

## 11. Agent Workflow

### Before coding
1. Read this file and any `README`, `CONTRIBUTING`, or nested `AGENTS.md` relevant to the area you're touching.
2. Explore the relevant code: entry points, callers, existing tests, similar features.
3. Identify the project's conventions (style, patterns, error handling, test structure) and follow them.
4. For non-trivial tasks, outline a short plan before writing code.
5. If the request is ambiguous or contradicts the codebase, ask before building.

### While coding
- Make the **smallest change** that fully solves the problem.
- Stay in scope. Note unrelated problems you find; don't fix them unasked.
- **Don't invent APIs.** Verify that functions, methods, flags, and library versions actually exist in this project before using them.
- Don't leave `TODO`s, placeholder implementations, or stubbed logic without saying so explicitly.
- Don't introduce new tools, frameworks, or patterns where the project already has an established one.
- Preserve existing behaviour unless changing it is the task.

### Before declaring done
- [ ] Code builds/compiles without new warnings.
- [ ] Linter and formatter pass.
- [ ] All tests pass, including new ones for the change.
- [ ] No debug output, commented-out code, or leftover scratch files.
- [ ] No secrets or credentials added.
- [ ] Public API or behaviour changes are documented.
- [ ] Edge cases and error paths are handled.

### Reporting
Summarise concisely:
- What changed and why.
- Any assumptions made.
- Anything not done, not verified, or that needs human review.
- Follow-up issues noticed but left out of scope.

Be honest. If tests weren't run or something failed, say so plainly. Never claim work is complete or verified when it isn't.

---

## 12. When Rules Conflict

Priority order:
1. **Correctness and security**
2. **Explicit instructions from the user** for this task
3. **Existing codebase conventions**
4. **This file**
5. Personal or general style preferences

Rules are tools for writing good software, not ends in themselves. When following a rule would clearly make the code worse, break it deliberately and explain why.