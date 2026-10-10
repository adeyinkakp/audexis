---
layout: ../../layouts/MarkdownLayout.astro
title: "Audexis — Artwork"
description: "Read, import, organize, and save album artwork in audio files."
---

# Artwork

Audexis displays embedded artwork in the library and players. A placeholder
appears when usable artwork is absent or cannot be loaded.

## Open the artwork editor

Right-click a song, choose **Get Info**, and open **Artwork**. A file can
contain more than one image, such as a front cover, back cover, or booklet
image.

Select an image to inspect and edit its details. Use the editor’s add/import
action to choose an image from your computer. Arrange the images using the
available ordering controls and remove entries you no longer want.

Choose **Save Artwork** to write the image list. Reset returns to the loaded
artwork without reversing a previous successful save.

## Picture type and description

The picture type describes the image’s role. The description is accompanying
text; it is not the song’s album title.

Storage support differs by format. Some containers retain richer image
descriptions and picture types than others. For example, MP4 cover artwork does
not offer the same per-picture fields as ID3 or FLAC.

Use conventional front-cover artwork for the image you want other players to
recognize most reliably. Audexis’s display and another player’s
artwork-selection rules may differ when several images are embedded.

## Several selected songs

When selected songs have different artwork, the editor indicates that the
selection is mixed. Saving the edited image list applies it to the selected
files; review the selection before replacing album covers across unrelated
tracks.

A mixed selection is not a merged view of every embedded image in every file.
Open a song individually if you need to preserve or inspect its unique image
set.

## Download a cover with metadata

In Details, use **Find metadata online** to search iTunes. Review the matching
release and cover before applying it.

The downloader accepts JPEG or PNG artwork and limits downloaded images to 20
MB. It tries a larger cover URL and falls back to the supplied image when
needed. The metadata-and-cover flow replaces the front cover while retaining
other existing artwork entries.

This lookup needs internet access. Importing artwork from your computer does
not.

## If an image does not save or appear

Check the reported error and whether the source audio file is writable. Use a
normal JPEG or PNG if the original image format is not accepted. Reopen Track
Info to inspect what was saved, and rescan if an externally changed file has not
refreshed.

Artwork is embedded when saved; moving or deleting the original image file
afterward does not remove that embedded copy.

[Metadata editing](/docs/metadata) · [Formats](/docs/formats)
