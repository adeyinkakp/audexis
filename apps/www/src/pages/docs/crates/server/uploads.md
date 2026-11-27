---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Uploads and limits"
description: "Understand upload processing and storage limits."
---

# Uploads and limits

An upload is not ready as soon as the client finishes sending bytes. The server
also checks the file and adds it to the cloud library.

## THe Upload flow

1. The client starts an upload with the file name, size, hash, metadata,
   and an operation ID.
2. The server returns the upload state and, when needed, an upload URL.
3. The client sends the file and submits the upload for completion.
4. The server verifies it and creates the library track.
5. The client checks the upload state until it is ready or reports an error.

If the same file already exists in the account, the server may return the
existing track without asking the client to send it again.

## Server and direct uploads

`AUDEXIS_UPLOAD_MODE=server` sends uploads through the server's ingestion path.
This is the default. Give the ingestion directory enough free space for files
waiting to be processed.

`AUDEXIS_UPLOAD_MODE=direct` uses the storage upload path. The server still
needs to verify the uploaded file; direct uploads do not skip that work.

## Account and file limits

`AUDEXIS_MAX_FILE_BYTES` limits a single upload. The account quota limits the
library's storage use. Pending uploads reserve space, so available capacity
can be lower than the size of the finished library suggests.

An account can have up to three pending uploads. If the server reports that
three are already pending, let them finish or cancel an unwanted upload before
starting another.

## Retrying

Reuse an operation ID only for the same upload request. Reusing it with
different file data returns a conflict.

For rate limits, wait before retrying and respect `Retry-After`. If storage has
hit a daily limit, the server puts transfers into a cooldown instead of
repeatedly retrying them immediately.

See [Troubleshooting](/docs/crates/server/troubleshooting) for failures that
continue after a retry.
