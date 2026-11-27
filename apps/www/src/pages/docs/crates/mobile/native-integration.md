---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Native integration"
description: "Call the Rust library and manage handles and responses."
---

# Native integration

## Calling from native code

The crate builds as a static library, a dynamic library, and a Rust library.
Its C interface is implemented in `src/ffi.rs`.

1. Create a handle with `audexis_mobile_create`.
2. Pass UTF-8 JSON requests to `audexis_mobile_call`.
3. Read the returned JSON. Successful calls return an `ok` field; failed calls
   return an `error` field.
4. Free each response with `audexis_mobile_string_free`.
5. Release the handle with `audexis_mobile_destroy` when finished.

Keep calls on the same handle serialized. The handle contains mutable library
and player state.
