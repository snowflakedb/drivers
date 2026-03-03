# GitHub Copilot Instructions: Effective Pull Requests

When generating or reviewing Pull Request titles and descriptions, you must adhere to the "Effective PRs" guidelines. Prioritize clarity, searchability, and narrative flow over snippets and bullet points.

## 1. PR Titles
- **Autonomy:** Titles must "stand alone" and be descriptive enough for a reader to understand the scope without opening the PR or a Jira ticket.
- **Context:** Include the subcomponent or system affected at the start (e.g., `config: `).
- **Searchability:** Use keywords that a developer would use when searching for changes in this specific area later.
- **Format:** `Jira-ID [component]: [descriptive action]`.
- **Avoid:** Generic titles like "fix bug," "small fix," or "update deps."

## 2. PR Descriptions (The Narrative)
Do NOT use section headers (e.g., ## Why). Instead, separate distinct concerns into clear, cohesive paragraphs. Use Markdown backticks for all code symbols, class names, or file paths (e.g., `WidgetValidator`). Use prose for all explanations unless providing a literal list of items.

Follow this narrative structure:

- **Paragraph 1: What is changing?** Provide a high-level summary of what the PR accomplishes. Focus on the intent and the "first audience" (the reviewer). Mention if new features are introduced or if existing APIs/assumptions are being modified.
- **Paragraph 2: Why is this change needed?** Explain the "Big Why." Detail the motivating reason, such as fixing a breakage or fulfilling a requirement for a larger effort. Reference Jira issues or external resources to provide context on importance.
- **Paragraph 3: Why this particular approach?** Explain the "Little Why." Detail implementation choices and why they are superior to alternatives. Explain how the code fits into existing architectures (e.g., deriving from a specific base class).
- **Paragraph 4: What are the risks?** Explicitly state the risks or lack thereof. If risks exist (e.g., CPU spikes from regex), explain how they are mitigated (e.g., using a `SafeRE` library or timeouts). 
- **Paragraph 5: How has it been tested?** Detail the specific steps taken to ensure correctness. Mention new unit tests, regression tests, or fuzz tests. If risks were identified in the previous paragraph, explain how the tests specifically address those risks.
- **Paragraph 6: What's next?** (If applicable) Explain if this is part of a multi-stage rollout, such as a change currently disabled by a feature flag or a precursor to a follow-up PR.
