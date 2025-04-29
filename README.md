# java_comment_extractor

A fast, offset-preserving Java comment and string extractor.

- Extracts comments (`//`, `/* ... */`) and optionally string contents.
- Replaces non-comment, non-string code with spaces, preserving line/column alignment.
- Supports Java 15+ triple-quoted `"""` text blocks.
- Useful for grammar checking with Flycheck + LanguageTool or Vale.

## Usage

```bash
java_comment_extractor [--preserve-strings] <input-file>
