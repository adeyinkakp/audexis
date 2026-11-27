---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Editing metadata"
description: "Load edits, check for conflicts, and save tags safely."
---

# Editing metadata

## Working with metadata

Start with `metadata_editor` when changing tags through the app. It keeps the
original metadata alongside your edits, so it can tell which fields changed.
Before saving, it checks those fields against the file on disk. If another
program changed them, the save reports a conflict instead of replacing them.

The format-specific code lives in `tag_manager`. Writes use a temporary copy
and verify the result before replacing the original file. Keep that behavior
when adding support for a field or format.

See [Editing metadata](/docs/apps/desktop/metadata) for the editor's behavior
and [Supported formats](/docs/apps/desktop/formats) for file-format details.
