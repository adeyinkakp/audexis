---
layout: ../../layouts/MarkdownLayout.astro
title: "Audexis — Library and folders"
description: "Understand scanning, library views, Songs columns, and missing files."
---

# Library and folders

## How the library works

Audexis watches the music folders you choose, reads file metadata, and keeps a
local database for browsing. Your audio files remain at their original paths.

Library views use embedded tags. A track’s displayed album or artist can differ
from its folder name. If tracks are grouped incorrectly, inspect their tags
before reorganizing folders.

## Choose a view

- **Home** highlights activity and music from your collection.
- **Songs** provides a configurable table of individual tracks.
- **Albums** groups music by album information.
- **Artists** helps you browse by artist.
- **Favorites** collects tracks you have marked with a heart.
- **Playlists** contains collections you create yourself.
- **Rewind** explores your recorded listening activity.

## Customize Songs columns

Right-click the Songs table header to choose visible columns. The menu includes
ordinary song information, regular metadata fields, and custom text/URL fields
discovered in the library or defined in settings.

Click a column heading to change its sort order. Use the heading’s drag handle
to reorder columns and its right edge to resize it. Column selection, order, and
widths are saved locally.

A blank cell means there is no indexed value for that song and field. Adding a
column does not create a tag in the file. Multiple values may be displayed
together in a cell; that display is separate from how values are edited and
stored.

See [Custom fields](/docs/custom-fields) for named columns and
[Metadata editing](/docs/metadata) for the Details field picker.

## Add, remove, or rescan folders

Use **Settings → Library** to manage watched folders. Folder changes apply when
you save. You can also import a folder or rescan through the File menu.

Save pending folder changes before using Rescan. Rescanning uses the saved
folder list and rereads library information; it does not repair invalid audio or
fill in missing tags from the internet.

Removing a folder from the configured library can remove its songs and playlist
membership from Audexis. It does not delete the source audio files. Re-adding a
folder is not a substitute for backing up library data.

## Moved and missing files

Audexis monitors filesystem changes, including moves and renames. If a file can
no longer be found, it can appear in **File → Library → Missing Files**.

For a disconnected external drive, reconnect it and rescan before removing
records. For a moved file, use the missing-file relink action to locate its new
path. Select the corresponding song, not an unrelated replacement with a similar
name.

Removing a missing-file record removes that library entry rather than deleting
an audio file from disk. This can also affect references such as playlist
membership. Relink first if you want to keep the existing library identity.

## Related guides

[Playlists](/docs/playlists) ·
[Troubleshooting](/docs/troubleshooting)
