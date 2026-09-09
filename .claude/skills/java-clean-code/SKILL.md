---
name: java-clean-code
description: Refactors and reviews Java code for cohesive methods, consistent abstraction levels, and behavior-preserving helper extraction. Use when implementing or reviewing Java/JDBC code, simplifying a long method, extracting helpers, or when the user asks for Java clean code or a Java refactor.
---

# Java clean code

Improve readability without changing observable behavior. Treat method length as a signal, never as the rule.

## Workflow

1. Read the changed method, its callers, tests, and nearby helpers.
2. State the behavior that must remain stable: return values, exception types and order, side effects, input mutation, null handling, and public visibility.
3. Identify cohesive phases. Common phases are input guards, resolution/parsing, validation, domain decisions, result construction, and side effects.
4. Keep the entry method at one abstraction level. Give each extracted helper one purpose and a name that describes the domain action.
5. Choose the narrowest boundary:
   - use a `private static` helper for stateless logic local to one class;
   - use a package-private collaborator when logic owns state/dependencies, is reused, or merits focused tests;
   - do not widen visibility solely to test an implementation detail.
6. Preserve execution order and data flow. Do not combine cleanup, renaming, or semantic changes with extraction unless tests explicitly cover them.
7. Run focused tests and formatting. Review the diff for accidental API or behavior changes.

## Extraction standard

Extract when a changed method mixes multiple domain responsibilities and named helpers make its control flow easier to understand. A good orchestration method reads as a short sequence of intent:

```java
Properties resolved = resolveProperties(url, info);
validate(resolved);
return findMissingProperties(resolved);
```

Helpers should:

- operate at a consistent abstraction level;
- have a single reason to change;
- make contracts explicit through names and types;
- avoid hidden mutation and surprising side effects;
- avoid long parameter lists by keeping cohesive data together.
- reuse established project utilities for canonical predicates and conversions after verifying
  semantic equivalence. In JDBC code, prefer
  `net.snowflake.client.internal.util.StringUtil.isNullOrEmpty(value)` over repeating
  `value == null || value.isEmpty()`.

Do not substitute a similar-looking utility blindly. For example, null-or-empty checks preserve
whitespace while `isBlank` usually does not; changing between them is a behavior change.

## Prefer direct collection operations

Use the collection API that directly expresses an unchanged bulk operation:

```java
Properties resolved = new Properties();
resolved.putAll(parsed.getParameters());

Set<ResultSet> snapshot = new LinkedHashSet<>(openResultSets);
```

Prefer `putAll`, `addAll`, or a copy constructor when a loop only copies every entry or element
unchanged. Keep the loop when it filters, transforms, validates, accumulates failures, performs
per-item side effects, or depends on iteration order. Confirm that null handling, duplicate-key
behavior, destination type, and failure semantics remain compatible before simplifying.

## Do not over-refactor

Do not extract:

- a trivial expression whose helper name adds no information;
- every branch merely to satisfy a line-count threshold;
- code that requires many unrelated parameters and obscures data flow;
- logic into a generic `Utils` class without a cohesive owner;
- existing untouched code solely because it could be cleaner.

Prefer one cohesive refactor over a chain of tiny forwarding methods.

## Behavior-preservation checklist

- [ ] Public signatures and visibility are unchanged unless requested.
- [ ] Validation and exception precedence are unchanged.
- [ ] Null, empty, and malformed inputs follow the same paths.
- [ ] Mutable inputs are copied or mutated exactly as before.
- [ ] Side effects and resource cleanup occur in the same order.
- [ ] Existing tests pass; focused tests cover any newly exposed seam.

## Review output

Report only actionable findings introduced or materially worsened by the diff. Explain the mixed responsibilities, propose cohesive helper boundaries, and note the behavior that must be preserved. Do not report method length alone.
