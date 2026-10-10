---
layout: ../../layouts/MarkdownLayout.astro
title: "Audexis — Custom metadata fields"
description: "Create reusable custom field definitions and display their values in Songs."
---

# Custom metadata fields

Custom fields store information beyond the standard tag list: a personal
category, a source link, a catalog note, or another text value you want to keep
with your music.

## Definition, key, and value

These are three different things:

- **Display title:** the friendly name shown in Audexis, such as `My category`.
- **Metadata key:** the name stored in the file, such as `MY_CATEGORY`.
- **Value:** the information for one song, such as `Late-night listening`.

A saved definition contains the display title, key, and field type. Saving it
makes the field reusable; it does not insert the same value into all your songs.

## Create a definition

1. Open **Settings → Custom Fields**. The Songs header’s **Manage Custom
   Fields…** action opens this section too.
2. Choose **New Field**.
3. Enter a display title and metadata key.
4. Choose Text or URL.
5. Choose **Save Fields**.

Changing or removing a definition does not rename or delete existing tags. To
move values to a new key, edit the actual song fields as well.

## Save a value into a song

1. Open **Get Info → Custom Fields**.
2. If several songs are selected, choose the song to edit.
3. Add a saved field, or create an individual text/URL row.
4. Enter its key and value.
5. Choose **Save Song Values**.

You can save a row’s key and type as a reusable definition. Removing a row and
saving removes that custom entry from the selected file. Reset discards the
pending custom-field draft.

## Key rules

Use simple ASCII keys such as `SOURCE_URL` or `MY_CATEGORY`. The editor rejects
empty keys, NUL characters, and equals signs in keys. It also restricts the
accepted key character range and rejects duplicate names within the same field
type.

The equals sign separates a name from its value in formats such as Vorbis
comments; it belongs in the value, not the key. For example, the key can be
`SOURCE_URL` and the value can be `https://example.org/track?id=42`.

Semicolons and equals signs in values are preserved as text. URL values must be
ASCII; percent-encode non-ASCII characters in a URL.

## Add a Songs column

Right-click the Songs header and open **Custom Text / URL Fields**. Choose a
saved field or a custom key discovered in your library.

Saved titles are used as column labels. A song needs a matching stored key to
display a value; adding the column itself does not write anything. FLAC/Ogg keys
are normalized to uppercase for storage lookup. Separate text and URL frames can
exist in ID3, while other formats use text storage for both.

[Metadata editing](/docs/metadata) · [Field reference](/docs/metadata-reference)
· [Formats](/docs/formats)
