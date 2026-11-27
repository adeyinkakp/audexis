---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Cloud sync"
description: "Connect a cloud library, upload music, download songs, and resolve sync issues."
---

# Cloud sync

Cloud sync connects your desktop library to audexis Cloud or a custom audexis
server. Cloud music appears alongside local music in **Songs**, **Albums**,
**Artists**, **Favorites**, and **Playlists**. Use **Cloud Sync** in the sidebar
to check progress and resolve problems.

You can keep using a local library without signing in.

## Connect a server

Open **Settings → Cloud** and choose one provider:

- For audexis Cloud, choose **Log in to audexis Cloud** and complete sign-in in
  your browser. The request expires after two minutes. Cancel and start again
  if it expires.
- For a custom server, enter its **Server URL** and **API key**, then choose
  **Save custom server**. Use the address and key of your custom server.

The app checks the connection before saving it. Log out of the active provider
before switching to another one.

Credentials are saved in the operating system’s keyring when available. If the
app reports a session-only connection, you will need to sign in again after
closing Audexis.

## Upload your music

Add your music folders and let the library scan finish. Keep the source files
and external drives available while uploading.

Choose **Sync now** in Settings → Cloud or on the Cloud Sync page to upload
eligible music from your library and send pending library changes. This applies
to the library, not just the selected song or album. The first upload can take
time, depending on the collection and connection.

Enable **Automatically sync library** to upload new music automatically. This
preference saves immediately and is off by default.

Turning it off pauses automatic uploads. Cloud-library updates still arrive,
and edits to synced music and playlists can still be sent. It is not an offline
mode. **Sync now** also runs uploads while the preference is off.

Enable cloud sync for the playlists you want available on other devices.
Playlist sync waits until its member tracks have cloud counterparts. If a
playlist has not appeared, check its sync setting and whether its songs have
finished uploading first.

## Stream or download songs

Play cloud songs from the ordinary library views. Audexis uses an available
local copy first and streams when it needs the cloud audio.

To keep a cloud song available offline, right-click it and choose **Download
for Offline**. Finish the download before disconnecting. Keep the same account
signed in to access its downloaded songs.

**Remove Download** removes the downloaded copy from this device. It does not
delete the cloud song. Streaming also uses a playback cache, but a partially
cached song may need a connection to finish playing. Use an explicit download
for planned offline listening.

## Check progress and storage

Open **Cloud Sync** to see sync status, queued operations, and errors. Choose
**Check storage usage** to see used, pending, and total cloud storage. Pending
storage includes space reserved for uploads that have not finished.

An upload may still need server processing after its file transfer finishes.
Keep the app open and check for errors before assuming the song is ready on
another device.

## Resolve sync problems

Read the error beside the queued operation. Check the connection, source file,
and available cloud storage before choosing **Retry**. Retry starts a manual
sync pass, including eligible uploads, even when automatic uploads are off.

When a change conflicts with the cloud version, choose deliberately:

- **Use my change** retries your pending edit against the latest known cloud
  revision and starts a manual sync pass. It can replace the conflicting cloud
  values.
- **Use cloud version** discards that pending edit and keeps the cloud version
  in the library.

These actions resolve library records. Do not assume they rewrite the embedded
tags in your original audio files. Check the source file separately if its tags
also need changing.

If sign-in is no longer valid, return to **Settings → Cloud** and sign in again.
A busy server may ask the app to wait before retrying. Repeatedly pressing
Sync now does not bypass that waiting period.

## Log out and keep backups

Logging out stops sync and cloud playback. It does not delete your library from
the server or erase your original music files. Logging out is not a way to
remove all cached data from a shared computer.

Keep separate backups of your music and local app data. Cloud sync is not a
complete backup of every desktop preference or listening-history record.

[Library and folders](/docs/apps/desktop/library) ·
[Privacy and data](/docs/apps/desktop/privacy-and-data) ·
[Troubleshooting](/docs/apps/desktop/troubleshooting)
